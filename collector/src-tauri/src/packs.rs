//! Сборки модов: хранение, добавление из каталогов/файлов/ссылок, проверка, экспорт .mrpack.
use crate::core::{self, emit, now_ms, Job, DATA};
use mccore::{http, modmeta, modpack, providers, versions, Ctx, Res};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn d_ver() -> String { "1.0.0".into() }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Pack {
    pub id: String,
    pub name: String,
    #[serde(default)] pub author: String,
    #[serde(default = "d_ver")] pub version: String,
    #[serde(default)] pub description: String,
    pub mc: String,
    pub loader: String,
    #[serde(default)] pub loader_version: String,
    #[serde(default)] pub created: u64,
    #[serde(default)] pub updated: u64,
    #[serde(default)] pub mods: Vec<Value>,
}

fn dir(id: &str) -> PathBuf { DATA.join("packs").join(id) }
fn files_dir(id: &str) -> PathBuf { dir(id).join("files") }
pub fn load(id: &str) -> Res<Pack> {
    core::read_json(&dir(id).join("pack.json")).and_then(|v| serde_json::from_value(v).ok()).ok_or("Сборка не найдена".into())
}
fn save(p: &mut Pack) -> Res<()> {
    p.updated = now_ms();
    core::write_json(&dir(&p.id).join("pack.json"), p)?;
    emit(json!({ "type": "pack", "id": p.id }));
    Ok(())
}
fn key(e: &Value) -> String {
    match e["source"].as_str().unwrap_or("") {
        "modrinth" | "curseforge" => format!("{}:{}", e["source"].as_str().unwrap(), e["project"].as_str().unwrap_or("")),
        s => format!("{s}:{}", e["file"]["name"].as_str().unwrap_or("")),
    }
}
fn with_key(mut e: Value) -> Value { let k = key(&e); e["key"] = json!(k); e }

pub fn list() -> Value {
    let mut v: Vec<Value> = mccore::ls(&DATA.join("packs")).into_iter().filter_map(|id| load(&id).ok()).map(|p| json!({
        "id": p.id, "name": p.name, "mc": p.mc, "loader": p.loader, "count": p.mods.len(), "updated": p.updated,
        "icon": p.mods.iter().find_map(|m| m["icon"].as_str().filter(|i| i.starts_with("http")).map(String::from)),
    })).collect();
    v.sort_by(|a, b| b["updated"].as_u64().cmp(&a["updated"].as_u64()));
    json!(v)
}
pub fn get(id: &str) -> Res<Value> { Ok(json!(load(id)?)) }

pub fn create(a: &Value) -> Res<Value> {
    let name = core::arg(a, "name");
    let mc = core::arg(a, "mc");
    if mc.is_empty() { mccore::bail!("Выберите версию Minecraft"); }
    let mut p = Pack {
        id: core::new_id(), name: if name.trim().is_empty() { "Новая сборка".into() } else { name.trim().into() },
        author: core::set_str("author"), version: d_ver(), description: String::new(), mc,
        loader: match core::arg(a, "loader") { l if l.is_empty() => "fabric".into(), l => l },
        loader_version: core::arg(a, "loaderVersion"), created: now_ms(), updated: 0, mods: vec![],
    };
    save(&mut p)?;
    Ok(json!(p))
}
pub fn update(id: &str, patch: &Value) -> Res<Value> {
    let mut p = load(id)?;
    for (k, f) in [("name", &mut p.name), ("author", &mut p.author), ("version", &mut p.version), ("description", &mut p.description), ("loaderVersion", &mut p.loader_version)] {
        if let Some(v) = patch.get(k).and_then(|v| v.as_str()) { *f = v.trim().to_string(); }
    }
    save(&mut p)?;
    Ok(json!(p))
}
pub fn delete(id: &str) -> Res<()> { std::fs::remove_dir_all(dir(id)).ctx("Удаление сборки") }
pub fn duplicate(id: &str) -> Res<Value> {
    let mut p = load(id)?;
    let old = p.id.clone();
    p.id = core::new_id();
    p.name = format!("{} (копия)", p.name);
    p.created = now_ms();
    if files_dir(&old).exists() {
        std::fs::create_dir_all(files_dir(&p.id)).ctx("")?;
        for f in mccore::ls(&files_dir(&old)) { let _ = std::fs::copy(files_dir(&old).join(&f), files_dir(&p.id).join(&f)); }
    }
    save(&mut p)?;
    Ok(json!(p))
}

/// Добавляет записи, не дублируя уже имеющиеся; явное добавление снимает пометку «зависимость».
fn merge(p: &mut Pack, entries: Vec<Value>) -> Vec<String> {
    let mut added = vec![];
    for e in entries {
        let e = with_key(e);
        let k = e["key"].as_str().unwrap_or("").to_string();
        if let Some(old) = p.mods.iter_mut().find(|m| m["key"] == k.as_str()) {
            if e["dependency"] == false { old["dependency"] = json!(false); }
            continue;
        }
        added.push(e["name"].as_str().unwrap_or("").to_string());
        p.mods.push(e);
    }
    p.mods.sort_by_key(|m| (m["dependency"].as_bool().unwrap_or(false), m["name"].as_str().unwrap_or("").to_lowercase()));
    added
}

pub async fn add(id: &str, provider: &str, project: &str) -> Res<Value> {
    let p = load(id)?;
    let entries = providers::resolve(provider, project, &p.mc, &p.loader, true).await?;
    let mut p = load(id)?;
    let added = merge(&mut p, entries);
    save(&mut p)?;
    Ok(json!({ "added": added }))
}

/// Запись для .jar: опознаём в каталогах, иначе — локальный файл внутри сборки.
async fn entry_for_bytes(id: &str, name: &str, bytes: Vec<u8>, url: Option<&str>) -> Res<Value> {
    if let Some(e) = providers::identify(&bytes).await { return Ok(e); }
    let tmp = DATA.join("tmp").join(mccore::file_name(name));
    std::fs::create_dir_all(tmp.parent().unwrap()).ctx("")?;
    std::fs::write(&tmp, &bytes).ctx("")?;
    let meta = modmeta::read_jar(&tmp);
    let file = json!({ "name": mccore::file_name(name), "url": url, "sha1": mccore::sha1_hex(&bytes), "sha512": mccore::sha512_hex(&bytes), "size": bytes.len() });
    let source = if url.is_some() { "url" } else {
        std::fs::create_dir_all(files_dir(id)).ctx("")?;
        std::fs::rename(&tmp, files_dir(id).join(mccore::file_name(name))).or_else(|_| std::fs::copy(&tmp, files_dir(id).join(mccore::file_name(name))).map(|_| ())).ctx("Копирование файла")?;
        "file"
    };
    let _ = std::fs::remove_file(&tmp);
    Ok(json!({
        "source": source, "name": meta["name"].as_str().unwrap_or(&mccore::file_name(name).replace(".jar", "")), "version": meta["version"],
        "desc": meta["desc"], "icon": meta["icon"], "page": url, "client": "required", "server": "required", "dependency": false, "file": file,
    }))
}
pub async fn add_files(id: &str, paths: &[String]) -> Res<Value> {
    load(id)?;
    let (mut entries, mut failed) = (vec![], vec![]);
    for path in paths.iter().filter(|p| p.to_lowercase().ends_with(".jar")) {
        match std::fs::read(path) {
            Ok(b) => match entry_for_bytes(id, path, b, None).await { Ok(e) => entries.push(e), Err(e) => failed.push(format!("{path}: {e}")) },
            Err(e) => failed.push(format!("{path}: {e}")),
        }
    }
    let mut p = load(id)?;
    let added = merge(&mut p, entries);
    save(&mut p)?;
    Ok(json!({ "added": added, "failed": failed }))
}
pub async fn add_url(id: &str, url: &str) -> Res<Value> {
    load(id)?;
    let url = url.trim();
    if !url.starts_with("http") { mccore::bail!("Нужна прямая ссылка на .jar"); }
    let tmp = DATA.join("tmp").join(format!("dl-{}.jar", core::new_id()));
    http::download(url, &tmp, |_| {}).await?;
    let bytes = std::fs::read(&tmp).ctx("")?;
    let _ = std::fs::remove_file(&tmp);
    if !bytes.starts_with(b"PK") { mccore::bail!("По ссылке не .jar-файл (возможно, это страница сайта, а не прямая ссылка)"); }
    let name = url.split(['?', '#']).next().unwrap_or(url).rsplit('/').next().filter(|n| n.to_lowercase().ends_with(".jar")).map(|n| urlencoding::decode(n).map(|s| s.into_owned()).unwrap_or(n.into())).unwrap_or_else(|| format!("mod-{}.jar", core::new_id()));
    let e = entry_for_bytes(id, &name, bytes, Some(url)).await?;
    let mut p = load(id)?;
    let added = merge(&mut p, vec![e]);
    save(&mut p)?;
    Ok(json!({ "added": added }))
}
pub fn remove(id: &str, k: &str) -> Res<()> {
    let mut p = load(id)?;
    if let Some(m) = p.mods.iter().find(|m| m["key"] == k) {
        if m["source"] == "file" { let _ = std::fs::remove_file(files_dir(id).join(mccore::file_name(m["file"]["name"].as_str().unwrap_or("")))); }
    }
    p.mods.retain(|m| m["key"] != k);
    save(&mut p)
}
pub fn set_side(id: &str, k: &str, client: &str, server: &str) -> Res<()> {
    let mut p = load(id)?;
    let m = p.mods.iter_mut().find(|m| m["key"] == k).ok_or("Мод не найден")?;
    m["client"] = json!(client);
    m["server"] = json!(server);
    save(&mut p)
}

/// Проверка: какие моды можно обновить и какие недоступны для версии сборки.
pub async fn check(id: &str, mc: Option<&str>, loader: Option<&str>) -> Res<Value> {
    let p = load(id)?;
    let mc = mc.unwrap_or(&p.mc).to_string();
    let loader = loader.unwrap_or(&p.loader).to_string();
    let items: Vec<Value> = p.mods.iter().filter(|m| m["source"] == "modrinth" || m["source"] == "curseforge").cloned().collect();
    let jobs: Vec<futures_util::future::BoxFuture<'static, Value>> = items.into_iter().map(|m| {
        let (src, proj, mc, loader) = (m["source"].as_str().unwrap_or("").to_string(), m["project"].as_str().unwrap_or("").to_string(), mc.clone(), loader.clone());
        Box::pin(async move {
            match providers::resolve(&src, &proj, &mc, &loader, false).await {
                Ok(v) if !v.is_empty() => {
                    let n = with_key(v[0].clone());
                    let st = if n["versionId"] == m["versionId"] { "ok" } else { "update" };
                    json!({ "key": m["key"], "name": m["name"], "status": st, "current": m["version"], "latest": n["version"], "entry": n })
                }
                Ok(_) => json!({ "key": m["key"], "name": m["name"], "status": "missing" }),
                Err(e) => json!({ "key": m["key"], "name": m["name"], "status": if e.contains("Сеть") || e.contains("HTTP 5") { "error" } else { "missing" }, "error": e }),
            }
        }) as futures_util::future::BoxFuture<'static, Value>
    }).collect();
    let res: Vec<Value> = futures_util::stream::iter(jobs).buffer_unordered(8).collect().await;
    let other = p.mods.iter().filter(|m| m["source"] != "modrinth" && m["source"] != "curseforge").map(|m| m["name"].clone()).collect::<Vec<_>>();
    Ok(json!({ "results": res, "manual": other }))
}
use futures_util::StreamExt;

pub fn apply_updates(id: &str, entries: &Value) -> Res<usize> {
    let mut p = load(id)?;
    let mut n = 0;
    for e in entries.as_array().cloned().unwrap_or_default() {
        let e = with_key(e);
        if let Some(m) = p.mods.iter_mut().find(|m| m["key"] == e["key"]) {
            let (dep, c, s) = (m["dependency"].clone(), m["client"].clone(), m["server"].clone());
            *m = e;
            m["dependency"] = dep; m["client"] = c; m["server"] = s;
            n += 1;
        }
    }
    save(&mut p)?;
    Ok(n)
}
/// Смена версии сборки: моды переводятся на подходящие версии, недоступные помечаются.
pub async fn change_version(id: &str, mc: &str, loader: &str, lv: &str) -> Res<Value> {
    let r = check(id, Some(mc), Some(loader)).await?;
    let mut p = load(id)?;
    p.mc = mc.into(); p.loader = loader.into(); p.loader_version = lv.into();
    let mut missing = vec![];
    for x in r["results"].as_array().cloned().unwrap_or_default() {
        let Some(m) = p.mods.iter_mut().find(|m| m["key"] == x["key"]) else { continue };
        match x["status"].as_str() {
            Some("update") | Some("ok") => {
                let (dep, c, s) = (m["dependency"].clone(), m["client"].clone(), m["server"].clone());
                *m = x["entry"].clone();
                m["dependency"] = dep; m["client"] = c; m["server"] = s;
            }
            _ => { m["incompatible"] = json!(true); missing.push(x["name"].clone()); }
        }
    }
    for m in p.mods.iter_mut() { if !missing.contains(&m["name"]) { if let Some(o) = m.as_object_mut() { o.remove("incompatible"); } } }
    save(&mut p)?;
    Ok(json!({ "missing": missing, "manual": r["manual"] }))
}

fn loader_dep(loader: &str) -> &'static str { match loader { "fabric" => "fabric-loader", "forge" => "forge", "neoforge" => "neoforge", "quilt" => "quilt-loader", _ => "" } }
pub async fn loader_version_or_latest(p: &Pack) -> Res<String> {
    if !p.loader_version.is_empty() { return Ok(p.loader_version.clone()); }
    let v = versions::loader_versions(&p.loader, &p.mc).await?;
    v.get(0).and_then(|x| x.as_str()).map(String::from).ok_or(format!("Нет версий {} для Minecraft {}", mccore::loader_title(&p.loader), p.mc))
}

/// Экспорт в .mrpack: моды из каталогов и по ссылкам — ссылками, локальные .jar — внутри архива.
pub fn export(id: &str, dest: &str) -> Res<u64> {
    let p = load(id)?;
    let dest = PathBuf::from(dest);
    let job = Job::new(format!("Экспорт «{}»", p.name));
    let jid = job.id;
    tauri::async_runtime::spawn(async move {
        let r: Res<String> = async {
            let lv = loader_version_or_latest(&p).await?;
            let mut deps = json!({ "minecraft": p.mc });
            if !loader_dep(&p.loader).is_empty() { deps[loader_dep(&p.loader)] = json!(lv); }
            let (mut files, mut overrides) = (vec![], vec![]);
            let total = p.mods.len().max(1);
            for (i, m) in p.mods.iter().enumerate() {
                let f = &m["file"];
                let name = mccore::file_name(f["name"].as_str().unwrap_or(""));
                job.step(Some(&name), Some(i as f64 / total as f64 * 95.0));
                if m["source"] == "file" { overrides.push((format!("mods/{name}"), files_dir(&p.id).join(&name))); continue; }
                let url = f["url"].as_str().unwrap_or("").to_string();
                let (mut sha1, mut sha512, mut size) = (f["sha1"].as_str().unwrap_or("").to_string(), f["sha512"].as_str().unwrap_or("").to_string(), f["size"].as_u64().unwrap_or(0));
                // у CurseForge нет SHA-512 — скачиваем файл и считаем сами (нужно по формату .mrpack)
                if sha512.is_empty() || sha1.is_empty() || size == 0 {
                    let tmp = DATA.join("tmp").join(format!("h-{name}"));
                    http::download(&url, &tmp, |_| {}).await.map_err(|e| format!("{name}: {e}"))?;
                    let b = std::fs::read(&tmp).ctx("")?;
                    let _ = std::fs::remove_file(&tmp);
                    sha1 = mccore::sha1_hex(&b); sha512 = mccore::sha512_hex(&b); size = b.len() as u64;
                }
                files.push(modpack::OutFile { path: format!("mods/{name}"), url, sha1, sha512, size,
                    client: m["client"].as_str().unwrap_or("required").into(), server: m["server"].as_str().unwrap_or("required").into() });
            }
            let summary = if p.description.is_empty() { format!("{} {} · модов: {}", mccore::loader_title(&p.loader), p.mc, p.mods.len()) } else { p.description.clone() };
            let (d2, name, ver) = (dest.clone(), p.name.clone(), p.version.clone());
            tauri::async_runtime::spawn_blocking(move || modpack::write(&d2, &name, &ver, &summary, &deps, &files, &overrides)).await.map_err(|e| e.to_string())??;
            Ok(format!("Сохранено: {}", dest.display()))
        }.await;
        job.finish(&r);
        if r.is_ok() { emit(json!({ "type": "exported", "path": dest })); }
    });
    Ok(jid)
}

/// Новая сборка из .mrpack (своего или чужого).
pub async fn import_mrpack(path: &str) -> Res<Value> {
    let pk = modpack::read(Path::new(path))?;
    let mut p = Pack {
        id: core::new_id(), name: pk.name.clone(), author: String::new(), version: pk.index["versionId"].as_str().unwrap_or("1.0.0").into(),
        description: pk.index["summary"].as_str().unwrap_or("").into(), mc: pk.mc.clone(), loader: pk.loader.clone(), loader_version: pk.loader_version.clone(),
        created: now_ms(), updated: 0, mods: vec![],
    };
    let re = regex::Regex::new(r"cdn\.modrinth\.com/data/([A-Za-z0-9]+)/versions/([A-Za-z0-9]+)/").unwrap();
    let mut entries = vec![];
    let mut pids = vec![];
    for f in pk.index["files"].as_array().cloned().unwrap_or_default() {
        let path = f["path"].as_str().unwrap_or("");
        if !path.starts_with("mods/") { continue; }
        let url = f["downloads"][0].as_str().unwrap_or("").to_string();
        let name = mccore::file_name(path);
        let (source, project, vid) = match re.captures(&url) { Some(c) => ("modrinth", c[1].to_string(), c[2].to_string()), None => ("url", String::new(), String::new()) };
        if source == "modrinth" { pids.push(project.clone()); }
        entries.push(json!({ "source": source, "project": project, "versionId": vid, "name": name.trim_end_matches(".jar"), "client": f["env"]["client"].as_str().unwrap_or("required"),
            "server": f["env"]["server"].as_str().unwrap_or("required"), "dependency": false, "page": if source == "url" { json!(url) } else { Value::Null },
            "file": { "name": name, "url": url, "sha1": f["hashes"]["sha1"], "sha512": f["hashes"]["sha512"], "size": f["fileSize"] } }));
    }
    // версии модов Modrinth по id версии
    let vids: Vec<String> = entries.iter().filter(|e| e["source"] == "modrinth").filter_map(|e| e["versionId"].as_str().map(String::from)).collect();
    for chunk in vids.chunks(100) {
        if let Ok(Value::Array(vs)) = http::get_json(&format!("https://api.modrinth.com/v2/versions?ids={}", urlencoding::encode(&json!(chunk).to_string()))).await {
            for e in entries.iter_mut() {
                if let Some(v) = vs.iter().find(|v| v["id"] == e["versionId"]) { e["version"] = v["version_number"].clone(); }
            }
        }
    }
    // названия и иконки модов Modrinth
    if !pids.is_empty() {
        for chunk in pids.chunks(100) {
            if let Ok(Value::Array(ps)) = http::get_json(&format!("https://api.modrinth.com/v2/projects?ids={}", urlencoding::encode(&json!(chunk).to_string()))).await {
                for e in entries.iter_mut() {
                    if let Some(pj) = ps.iter().find(|x| x["id"] == e["project"]) {
                        e["name"] = pj["title"].clone(); e["icon"] = pj["icon_url"].clone(); e["desc"] = pj["description"].clone();
                        e["page"] = json!(format!("https://modrinth.com/mod/{}", pj["slug"].as_str().unwrap_or("")));
                    }
                }
            }
        }
    }
    // локальные .jar из overrides/mods
    let ov = DATA.join("tmp").join(format!("ov-{}", p.id));
    modpack::extract_overrides(&pk, &ov, false).await?;
    let jars: Vec<String> = mccore::ls(&ov.join("mods")).into_iter().filter(|f| f.to_lowercase().ends_with(".jar")).map(|f| ov.join("mods").join(f).to_string_lossy().to_string()).collect();
    core::write_json(&dir(&p.id).join("pack.json"), &p)?;
    for j in &jars { if let Ok(b) = std::fs::read(j) { if let Ok(e) = entry_for_bytes(&p.id, j, b, None).await { entries.push(e); } } }
    let _ = std::fs::remove_dir_all(&ov);
    merge(&mut p, entries);
    save(&mut p)?;
    Ok(json!(p))
}

/// Подсказка для новой сборки из папки mods: загрузчик и версия Minecraft.
pub async fn detect_folder(path: &str) -> Res<Value> {
    let d = PathBuf::from(path);
    let d = if d.join("mods").is_dir() { d.join("mods") } else { d };
    let jars: Vec<String> = mccore::ls(&d).into_iter().filter(|f| f.to_lowercase().ends_with(".jar")).collect();
    if jars.is_empty() { mccore::bail!("В папке нет .jar-файлов"); }
    let hashes: Vec<String> = jars.iter().filter_map(|f| std::fs::read(d.join(f)).ok()).map(|b| mccore::sha1_hex(&b)).collect();
    let r = http::post_json("https://api.modrinth.com/v2/version_files", &json!({ "hashes": hashes, "algorithm": "sha1" })).await.unwrap_or(json!({}));
    let (mut loaders, mut mcs) = (std::collections::HashMap::<String, u32>::new(), std::collections::HashMap::<String, u32>::new());
    if let Value::Object(m) = r {
        for v in m.values() {
            for l in v["loaders"].as_array().cloned().unwrap_or_default() { if let Some(l) = l.as_str() { *loaders.entry(l.into()).or_default() += 1; } }
            for g in v["game_versions"].as_array().cloned().unwrap_or_default() { if let Some(g) = g.as_str() { *mcs.entry(g.into()).or_default() += 1; } }
        }
    }
    let loader = ["fabric", "neoforge", "forge", "quilt"].iter().max_by_key(|l| loaders.get(**l).copied().unwrap_or(0)).filter(|l| loaders.get(**l).copied().unwrap_or(0) > 0).map(|s| s.to_string());
    let mc = mcs.into_iter().max_by_key(|x| x.1).map(|x| x.0);
    Ok(json!({ "dir": d, "count": jars.len(), "loader": loader, "mc": mc,
        "name": PathBuf::from(path).file_name().map(|s| s.to_string_lossy().to_string()) }))
}
pub fn folder_jars(path: &str) -> Vec<String> {
    let d = PathBuf::from(path);
    let d = if d.join("mods").is_dir() { d.join("mods") } else { d };
    mccore::ls(&d).into_iter().filter(|f| f.to_lowercase().ends_with(".jar")).map(|f| d.join(f).to_string_lossy().to_string()).collect()
}
