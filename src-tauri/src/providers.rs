//! Каталоги модов и плагинов: Modrinth, CurseForge, Hangar (PaperMC), Spigot (Spiget).
//! Поиск, установка с зависимостями, проверка и установка обновлений.
use crate::core::{self, Ctx, Res};
use crate::http;
use futures_util::future::BoxFuture;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MR: &str = "https://api.modrinth.com/v2";
const CF: &str = "https://api.curseforge.com/v1";
const HANGAR: &str = "https://hangar.papermc.io/api/v1";
const SPIGET: &str = "https://api.spiget.org/v2";

fn enc(v: &Value) -> String { urlencoding::encode(&v.to_string()).into_owned() }
fn cf_key() -> String { core::set_str("curseforgeKey").trim().to_string() }
fn is_plugin(loader: &str) -> bool { loader == "paper" }

/// Доступные каталоги для ядра.
pub fn list(loader: &str) -> Value {
    let cf = !cf_key().is_empty();
    let mut v = vec![json!({ "id": "modrinth", "name": "Modrinth" })];
    if is_plugin(loader) {
        v.push(json!({ "id": "hangar", "name": "Hangar" }));
        v.push(json!({ "id": "spigot", "name": "SpigotMC" }));
    }
    v.push(json!({ "id": "curseforge", "name": "CurseForge", "needsKey": !cf }));
    json!(v)
}

/// Под какие загрузчики Modrinth подходят файлы для нашего ядра.
fn mr_loaders(loader: &str) -> Vec<&'static str> {
    match loader {
        "paper" => vec!["paper", "spigot", "bukkit", "purpur"],
        "fabric" => vec!["fabric"],
        "forge" => vec!["forge"],
        "neoforge" => vec!["neoforge"],
        _ => vec![],
    }
}
/// Тип загрузчика в CurseForge: 1 Forge, 4 Fabric, 6 NeoForge.
fn cf_loader(loader: &str) -> Option<u32> { match loader { "forge" => Some(1), "fabric" => Some(4), "neoforge" => Some(6), _ => None } }

async fn cf_get(path: &str) -> Res<Value> {
    let key = cf_key();
    if key.is_empty() { crate::bail!("Для CurseForge нужен API-ключ — укажите его в настройках"); }
    let r = http::client().get(format!("{CF}{path}")).header("x-api-key", key).send().await.map_err(|e| format!("Сеть: {e}"))?;
    if r.status().as_u16() == 403 { crate::bail!("CurseForge отклонил API-ключ"); }
    if !r.status().is_success() { crate::bail!("CurseForge: HTTP {}", r.status().as_u16()); }
    r.json().await.ctx("CurseForge")
}
async fn cf_post(path: &str, body: &Value) -> Res<Value> {
    let r = http::client().post(format!("{CF}{path}")).header("x-api-key", cf_key()).json(body).send().await.map_err(|e| format!("Сеть: {e}"))?;
    if !r.status().is_success() { crate::bail!("CurseForge: HTTP {}", r.status().as_u16()); }
    r.json().await.ctx("CurseForge")
}

// ---------- поиск ----------
pub async fn search(provider: &str, q: &str, mc: &str, loader: &str, kind: &str, offset: u64) -> Res<Value> {
    let plugin = kind == "plugin";
    match provider {
        "curseforge" => {
            let mut url = format!("{CF}/mods/search?gameId=432&classId={}&sortField=2&sortOrder=desc&pageSize=30&index={offset}&searchFilter={}", if plugin { 5 } else { 6 }, urlencoding::encode(q));
            if !mc.is_empty() && !plugin { url += &format!("&gameVersion={mc}"); }
            if let Some(l) = cf_loader(loader) { url += &format!("&modLoaderType={l}"); }
            let r = cf_get(&url[CF.len()..]).await?;
            let hits: Vec<Value> = r["data"].as_array().cloned().unwrap_or_default().into_iter().map(|m| json!({
                "id": m["id"].to_string(), "title": m["name"], "desc": m["summary"], "icon": m["logo"]["thumbnailUrl"],
                "author": m["authors"][0]["name"], "downloads": m["downloadCount"], "url": m["links"]["websiteUrl"],
            })).collect();
            Ok(json!({ "hits": hits, "total": r["pagination"]["totalCount"].as_u64().unwrap_or(0).min(10000) }))
        }
        "hangar" => {
            let mut url = format!("{HANGAR}/projects?limit=30&offset={offset}&platform=PAPER&sort=-downloads&q={}", urlencoding::encode(q));
            if !mc.is_empty() { url += &format!("&version={mc}"); }
            let r = http::get_json(&url).await?;
            let hits: Vec<Value> = r["result"].as_array().cloned().unwrap_or_default().into_iter().map(|p| json!({
                "id": p["namespace"]["slug"], "title": p["name"], "desc": p["description"], "icon": p["avatarUrl"], "author": p["namespace"]["owner"],
                "downloads": p["stats"]["downloads"], "url": format!("https://hangar.papermc.io/{}/{}", p["namespace"]["owner"].as_str().unwrap_or(""), p["namespace"]["slug"].as_str().unwrap_or("")),
            })).collect();
            Ok(json!({ "hits": hits, "total": r["pagination"]["count"] }))
        }
        "spigot" => {
            let page = offset / 30 + 1;
            let url = if q.trim().is_empty() { format!("{SPIGET}/resources/free?size=30&page={page}&sort=-downloads") }
                else { format!("{SPIGET}/search/resources/{}?field=name&size=30&page={page}&sort=-downloads", urlencoding::encode(q.trim())) };
            let r = http::get_json(&url).await.unwrap_or(json!([]));
            let hits: Vec<Value> = r.as_array().cloned().unwrap_or_default().into_iter().filter(|p| p["premium"] != true).map(|p| {
                let external = p["external"] == true;
                json!({
                    "id": p["id"].to_string(), "title": p["name"], "desc": p["tag"], "downloads": p["downloads"],
                    "icon": p["icon"]["url"].as_str().filter(|u| !u.is_empty()).map(|u| format!("https://www.spigotmc.org/{u}")),
                    "url": format!("https://www.spigotmc.org/resources/{}", p["id"]),
                    "external": external, "note": if external { "файл на внешнем сайте" } else { "" },
                    "versions": p["testedVersions"],
                })
            }).collect();
            let n = hits.len() as u64;
            Ok(json!({ "hits": hits, "total": if n == 30 { offset + 60 } else { offset + n } }))
        }
        _ => {
            let mut facets = vec![json!([format!("project_type:{kind}")])];
            if !mc.is_empty() { facets.push(json!([format!("versions:{mc}")])); }
            let ls = mr_loaders(loader);
            if !ls.is_empty() { facets.push(json!(ls.iter().map(|l| format!("categories:{l}")).collect::<Vec<_>>())); }
            let url = format!("{MR}/search?limit=30&offset={offset}&query={}&facets={}", urlencoding::encode(q), enc(&json!(facets)));
            let r = http::get_json(&url).await?;
            let hits: Vec<Value> = r["hits"].as_array().cloned().unwrap_or_default().into_iter().map(|h| json!({
                "id": h["project_id"], "title": h["title"], "desc": h["description"], "icon": h["icon_url"], "author": h["author"],
                "downloads": h["downloads"], "client": h["client_side"], "server": h["server_side"],
                "url": format!("https://modrinth.com/{}/{}", h["project_type"].as_str().unwrap_or("mod"), h["slug"].as_str().unwrap_or("")),
            })).collect();
            Ok(json!({ "hits": hits, "total": r["total_hits"] }))
        }
    }
}

// ---------- установка ----------
type Done = Arc<Mutex<HashSet<String>>>;
pub async fn install(provider: &str, pid: &str, mc: &str, loader: &str, dir: &Path) -> Res<Vec<String>> {
    tokio::fs::create_dir_all(dir).await.ctx("Создание папки")?;
    let done: Done = Default::default();
    match provider {
        "curseforge" => cf_install(pid.into(), mc.into(), loader.into(), dir.into(), done).await,
        "hangar" => hangar_install(pid, mc, dir).await,
        "spigot" => spigot_install(pid, dir).await,
        _ => mr_install(pid.into(), mc.into(), loader.into(), dir.into(), done).await,
    }
}
async fn download_named(url: &str, dir: &Path, name: &str) -> Res<String> {
    let fname = core::file_name(name);
    if fname.is_empty() { crate::bail!("Пустое имя файла"); }
    http::download(url, &dir.join(&fname), |_| {}).await?;
    Ok(fname)
}

/// Подходящая версия проекта Modrinth (последняя под версию MC и загрузчик).
async fn mr_version(pid: &str, mc: &str, loader: &str) -> Res<Value> {
    let mut q = format!("?game_versions={}", enc(&json!([mc])));
    let ls = mr_loaders(loader);
    if !ls.is_empty() { q += &format!("&loaders={}", enc(&json!(ls))); }
    let vers = http::get_json(&format!("{MR}/project/{pid}/version{q}")).await?;
    vers.get(0).cloned().ok_or(format!("Нет версии для {} {mc}", crate::installer::loader_title(loader)))
}
fn mr_install(pid: String, mc: String, loader: String, dir: PathBuf, done: Done) -> BoxFuture<'static, Res<Vec<String>>> {
    Box::pin(async move {
        if !done.lock().insert(pid.clone()) { return Ok(vec![]); }
        let v = mr_version(&pid, &mc, &loader).await?;
        let files = v["files"].as_array().cloned().unwrap_or_default();
        let f = files.iter().find(|x| x["primary"] == true).or(files.first()).ok_or("Нет файлов")?;
        let mut out = vec![download_named(f["url"].as_str().unwrap_or(""), &dir, f["filename"].as_str().unwrap_or("mod.jar")).await?];
        for d in v["dependencies"].as_array().cloned().unwrap_or_default() {
            if d["dependency_type"] == "required" {
                if let Some(p) = d["project_id"].as_str() {
                    if let Ok(more) = mr_install(p.into(), mc.clone(), loader.clone(), dir.clone(), done.clone()).await { out.extend(more); }
                }
            }
        }
        Ok(out)
    })
}

async fn cf_file(mod_id: &str, mc: &str, loader: &str) -> Res<Value> {
    let mut url = format!("/mods/{mod_id}/files?pageSize=1");
    if !mc.is_empty() && !is_plugin(loader) { url += &format!("&gameVersion={mc}"); }
    if let Some(l) = cf_loader(loader) { url += &format!("&modLoaderType={l}"); }
    let r = cf_get(&url).await?;
    r["data"].get(0).cloned().ok_or("Нет файла под эту версию".into())
}
/// Прямая ссылка: если автор запретил стороннюю загрузку, downloadUrl пуст — берём CDN.
fn cf_url(f: &Value) -> String {
    f["downloadUrl"].as_str().map(String::from).unwrap_or_else(|| {
        let id = f["id"].as_u64().unwrap_or(0);
        format!("https://edge.forgecdn.net/files/{}/{}/{}", id / 1000, id % 1000, urlencoding::encode(f["fileName"].as_str().unwrap_or("")))
    })
}
fn cf_install(pid: String, mc: String, loader: String, dir: PathBuf, done: Done) -> BoxFuture<'static, Res<Vec<String>>> {
    Box::pin(async move {
        if !done.lock().insert(pid.clone()) { return Ok(vec![]); }
        let f = cf_file(&pid, &mc, &loader).await?;
        let mut out = vec![download_named(&cf_url(&f), &dir, f["fileName"].as_str().unwrap_or("mod.jar")).await?];
        for d in f["dependencies"].as_array().cloned().unwrap_or_default() {
            if d["relationType"] == 3 {
                if let Ok(more) = cf_install(d["modId"].to_string(), mc.clone(), loader.clone(), dir.clone(), done.clone()).await { out.extend(more); }
            }
        }
        Ok(out)
    })
}

async fn hangar_version(slug: &str, mc: &str) -> Res<Value> {
    let mut url = format!("{HANGAR}/projects/{slug}/versions?limit=1&platform=PAPER");
    if !mc.is_empty() { url += &format!("&platformVersion={mc}"); }
    let r = http::get_json(&url).await?;
    r["result"].get(0).cloned().ok_or(format!("Нет версии плагина для Paper {mc}"))
}
async fn hangar_install(slug: &str, mc: &str, dir: &Path) -> Res<Vec<String>> {
    let v = hangar_version(slug, mc).await?;
    let d = &v["downloads"]["PAPER"];
    let Some(url) = d["downloadUrl"].as_str() else { crate::bail!("Плагин скачивается только с сайта автора: {}", d["externalUrl"].as_str().unwrap_or("")) };
    let mut out = vec![download_named(url, dir, d["fileInfo"]["name"].as_str().unwrap_or(&format!("{slug}.jar"))).await?];
    // обязательные зависимости, опубликованные на Hangar
    for dep in v["pluginDependencies"]["PAPER"].as_array().cloned().unwrap_or_default() {
        if dep["required"] == true && dep["externalUrl"].is_null() {
            if let Some(n) = dep["name"].as_str() { if let Ok(more) = Box::pin(hangar_install(n, mc, dir)).await { out.extend(more); } }
        }
    }
    Ok(out)
}

async fn spigot_install(id: &str, dir: &Path) -> Res<Vec<String>> {
    let r = http::get_json(&format!("{SPIGET}/resources/{id}")).await?;
    if r["premium"] == true { crate::bail!("Платный плагин — купите и скачайте его на spigotmc.org"); }
    if r["external"] == true || r["file"]["type"] != ".jar" {
        crate::bail!("Автор публикует файл на внешнем сайте: {}", r["file"]["externalUrl"].as_str().unwrap_or("spigotmc.org"));
    }
    let name = format!("{}.jar", core::safe_name(r["name"].as_str().unwrap_or(id)).replace(' ', "_"));
    Ok(vec![download_named(&format!("{SPIGET}/resources/{id}/download"), dir, &name).await.map_err(|e| format!("SpigotMC не отдал файл (защита сайта) — скачайте вручную. {e}"))?])
}

// ---------- обновления ----------
/// CurseForge fingerprint: MurmurHash2 (seed 1) по байтам без пробельных символов.
pub fn murmur2_cf(data: &[u8]) -> u32 {
    let b: Vec<u8> = data.iter().copied().filter(|c| !matches!(c, 9 | 10 | 13 | 32)).collect();
    const M: u32 = 0x5bd1_e995;
    let len = b.len() as u32;
    let mut h: u32 = 1 ^ len;
    let mut chunks = b.chunks_exact(4);
    for c in &mut chunks {
        let mut k = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        k = k.wrapping_mul(M); k ^= k >> 24; k = k.wrapping_mul(M);
        h = h.wrapping_mul(M); h ^= k;
    }
    let r = chunks.remainder();
    if r.len() >= 3 { h ^= (r[2] as u32) << 16; }
    if r.len() >= 2 { h ^= (r[1] as u32) << 8; }
    if !r.is_empty() { h ^= r[0] as u32; h = h.wrapping_mul(M); }
    h ^= h >> 13; h = h.wrapping_mul(M); h ^= h >> 15;
    h
}

/// Проверяет, для каких .jar в папке есть новые версии под версию MC и загрузчик.
pub async fn check_updates(dir: &Path, mc: &str, loader: &str) -> Res<Value> {
    let jars: Vec<String> = core::ls(dir).into_iter().filter(|f| f.to_lowercase().ends_with(".jar")).collect();
    if jars.is_empty() { return Ok(json!({ "updates": [], "checked": 0, "unknown": [] })); }
    let data: HashMap<String, Vec<u8>> = jars.iter().filter_map(|f| std::fs::read(dir.join(f)).ok().map(|b| (f.clone(), b))).collect();
    let by_hash: HashMap<String, String> = data.iter().map(|(f, b)| (sha1_smol::Sha1::from(b).digest().to_string(), f.clone())).collect();
    let mut updates = vec![];
    let mut known: HashSet<String> = HashSet::new();

    // Modrinth: текущие версии по хешу и последние совместимые
    let hashes: Vec<&String> = by_hash.keys().collect();
    let cur = http::post_json(&format!("{MR}/version_files"), &json!({ "hashes": hashes, "algorithm": "sha1" })).await.unwrap_or(json!({}));
    let mut body = json!({ "hashes": hashes, "algorithm": "sha1", "game_versions": [mc] });
    let ls = mr_loaders(loader);
    if !ls.is_empty() { body["loaders"] = json!(ls); }
    let latest = http::post_json(&format!("{MR}/version_files/update"), &body).await.unwrap_or(json!({}));
    let mut pids = vec![];
    if let Value::Object(m) = &cur {
        for (h, v) in m {
            let Some(file) = by_hash.get(h) else { continue };
            known.insert(file.clone());
            let Some(new) = latest.get(h) else { continue };
            if new["id"] != v["id"] {
                let files = new["files"].as_array().cloned().unwrap_or_default();
                let f = files.iter().find(|x| x["primary"] == true).or(files.first()).cloned().unwrap_or(json!({}));
                pids.push(v["project_id"].clone());
                updates.push(json!({ "file": file, "provider": "modrinth", "project": v["project_id"], "current": v["version_number"], "latest": new["version_number"],
                    "newFile": f["filename"], "url": f["url"] }));
            }
        }
    }
    if !pids.is_empty() {
        if let Ok(Value::Array(ps)) = http::get_json(&format!("{MR}/projects?ids={}", enc(&json!(pids)))).await {
            for u in updates.iter_mut() {
                if let Some(p) = ps.iter().find(|p| p["id"] == u["project"]) { u["title"] = p["title"].clone(); u["icon"] = p["icon_url"].clone(); }
            }
        }
    }

    // CurseForge: по отпечаткам файлов, которых нет на Modrinth
    if !cf_key().is_empty() && !is_plugin(loader) {
        let rest: Vec<(&String, u32)> = data.iter().filter(|(f, _)| !known.contains(*f)).map(|(f, b)| (f, murmur2_cf(b))).collect();
        if !rest.is_empty() {
            let fp: Vec<u32> = rest.iter().map(|x| x.1).collect();
            if let Ok(r) = cf_post("/fingerprints/432", &json!({ "fingerprints": fp })).await {
                for m in r["data"]["exactMatches"].as_array().cloned().unwrap_or_default() {
                    let fid = m["file"]["fileFingerprint"].as_u64().unwrap_or(0) as u32;
                    let Some((file, _)) = rest.iter().find(|x| x.1 == fid) else { continue };
                    known.insert((*file).clone());
                    let mod_id = m["id"].to_string();
                    if let Ok(nf) = cf_file(&mod_id, mc, loader).await {
                        if nf["id"] != m["file"]["id"] {
                            updates.push(json!({ "file": file, "provider": "curseforge", "project": mod_id, "title": m["file"]["displayName"],
                                "current": m["file"]["displayName"], "latest": nf["displayName"], "newFile": nf["fileName"], "url": cf_url(&nf) }));
                        }
                    }
                }
            }
        }
    }
    let unknown: Vec<&String> = jars.iter().filter(|f| !known.contains(*f)).collect();
    Ok(json!({ "updates": updates, "checked": jars.len(), "unknown": unknown }))
}

/// Скачивает новую версию и удаляет старый файл.
pub async fn apply_update(dir: &Path, u: &Value) -> Res<String> {
    let old = core::file_name(u["file"].as_str().unwrap_or(""));
    let url = u["url"].as_str().ok_or("Нет ссылки на файл")?;
    let new = download_named(url, dir, u["newFile"].as_str().unwrap_or(&old)).await?;
    if !old.is_empty() && old != new { let _ = tokio::fs::remove_file(dir.join(&old)).await; }
    Ok(new)
}

// ---------- карточка проекта (превью) ----------
/// Картинки из Markdown/HTML-описания (для каталогов без отдельной галереи).
fn images_in(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"!\[[^\]]*\]\((https?://[^)\s]+)|<img[^>]+src="(https?://[^"]+)""#).unwrap();
    let mut v: Vec<String> = re.captures_iter(text).filter_map(|c| c.get(1).or(c.get(2)).map(|m| m.as_str().to_string()))
        .filter(|u| !u.contains("shields.io") && !u.contains("badge") && !u.ends_with(".svg")).collect();
    v.dedup();
    v.truncate(12);
    v
}
/// Описание в простой текст: без HTML-тегов, картинок и разметки ссылок.
pub fn plain_text(text: &str) -> String {
    let t = regex::Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap().replace_all(text, "");
    let t = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap().replace_all(&t, "$1");
    let t = regex::Regex::new(r"(?i)<br\s*/?>|</p>|</li>|</h\d>").unwrap().replace_all(&t, "\n");
    let t = regex::Regex::new(r"<[^>]+>").unwrap().replace_all(&t, "");
    // разметка Markdown: заголовки, жирный/курсив, разделители, код
    let t = regex::Regex::new(r"(?m)^\s*(#{1,6}\s+|[-*_]{3,}\s*$|>\s?)").unwrap().replace_all(&t, "");
    let t = regex::Regex::new(r"\*\*|__|`{1,3}").unwrap().replace_all(&t, "");
    let t = t.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'");
    let t = regex::Regex::new(r"\n\s*\n\s*\n+").unwrap().replace_all(&t, "\n\n");
    t.trim().chars().take(4000).collect()
}
pub async fn details(provider: &str, id: &str) -> Res<Value> {
    match provider {
        "curseforge" => {
            let m = cf_get(&format!("/mods/{id}")).await?["data"].clone();
            let desc = cf_get(&format!("/mods/{id}/description")).await.ok().and_then(|d| d["data"].as_str().map(String::from)).unwrap_or_default();
            let mut gallery: Vec<String> = m["screenshots"].as_array().cloned().unwrap_or_default().iter().filter_map(|s| s["url"].as_str().map(String::from)).collect();
            if gallery.is_empty() { gallery = images_in(&desc); }
            Ok(json!({ "title": m["name"], "icon": m["logo"]["thumbnailUrl"], "author": m["authors"][0]["name"], "downloads": m["downloadCount"],
                "summary": m["summary"], "body": plain_text(&desc), "gallery": gallery, "url": m["links"]["websiteUrl"] }))
        }
        "hangar" => {
            let p = http::get_json(&format!("{HANGAR}/projects/{id}")).await?;
            let page = http::get_text(&format!("{HANGAR}/pages/main/{id}")).await.unwrap_or_default();
            Ok(json!({ "title": p["name"], "icon": p["avatarUrl"], "author": p["namespace"]["owner"], "downloads": p["stats"]["downloads"],
                "summary": p["description"], "body": plain_text(&page), "gallery": images_in(&page),
                "url": format!("https://hangar.papermc.io/{}/{}", p["namespace"]["owner"].as_str().unwrap_or(""), p["namespace"]["slug"].as_str().unwrap_or("")) }))
        }
        "spigot" => {
            let p = http::get_json(&format!("{SPIGET}/resources/{id}")).await?;
            use base64::Engine;
            let html = p["description"].as_str().and_then(|d| base64::engine::general_purpose::STANDARD.decode(d).ok()).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
            Ok(json!({ "title": p["name"], "icon": p["icon"]["url"].as_str().filter(|u| !u.is_empty()).map(|u| format!("https://www.spigotmc.org/{u}")),
                "downloads": p["downloads"], "summary": p["tag"], "body": plain_text(&html), "gallery": images_in(&html), "url": format!("https://www.spigotmc.org/resources/{id}") }))
        }
        _ => {
            let p = http::get_json(&format!("{MR}/project/{id}")).await?;
            let body = p["body"].as_str().unwrap_or("");
            let mut gallery: Vec<Value> = p["gallery"].as_array().cloned().unwrap_or_default();
            gallery.sort_by_key(|g| std::cmp::Reverse(g["featured"].as_bool().unwrap_or(false)));
            let mut g: Vec<String> = gallery.iter().filter_map(|x| x["url"].as_str().map(String::from)).collect();
            if g.is_empty() { g = images_in(body); }
            Ok(json!({ "title": p["title"], "icon": p["icon_url"], "downloads": p["downloads"], "summary": p["description"], "body": plain_text(body), "gallery": g,
                "url": format!("https://modrinth.com/{}/{}", p["project_type"].as_str().unwrap_or("mod"), p["slug"].as_str().unwrap_or("")),
                "client": p["client_side"], "server": p["server_side"], "versions": p["game_versions"].as_array().map(|a| a.iter().rev().take(6).cloned().collect::<Vec<_>>()) }))
        }
    }
}
