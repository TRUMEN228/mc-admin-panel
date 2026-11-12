//! Моды сервера, каталог Modrinth и наборы модов клиента.
use crate::core::{self, emit, Ctx, Res, DATA};
use crate::{http, installer, providers, servers};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const MR: &str = "https://api.modrinth.com/v2";
fn enc(v: &Value) -> String { urlencoding::encode(&v.to_string()).into_owned() }

// ---------- jar-файлы ----------
fn list_jars(dir: &Path) -> Vec<Value> {
    let mut v: Vec<Value> = std::fs::read_dir(dir).map(|r| r.flatten().filter_map(|e| {
        let f = e.file_name().to_string_lossy().to_string();
        let l = f.to_lowercase();
        if !(l.ends_with(".jar") || l.ends_with(".jar.disabled")) { return None; }
        Some(json!({ "file": f, "enabled": l.ends_with(".jar"), "size": e.metadata().map(|m| m.len()).unwrap_or(0) }))
    }).collect()).unwrap_or_default();
    v.sort_by_key(|x| x["file"].as_str().unwrap_or("").to_lowercase());
    v
}
fn toggle(dir: &Path, file: &str) -> Res<()> {
    let f = core::file_name(file);
    let to = match f.strip_suffix(".disabled") { Some(b) => b.to_string(), None => format!("{f}.disabled") };
    std::fs::rename(dir.join(&f), dir.join(to)).ctx("Переименование")
}
fn copy_in(dir: &Path, paths: &[String]) -> Res<Vec<String>> {
    std::fs::create_dir_all(dir).ctx("")?;
    let mut out = vec![];
    for p in paths {
        let n = core::file_name(p);
        if !n.to_lowercase().ends_with(".jar") { continue; }
        std::fs::copy(p, dir.join(&n)).ctx(&format!("Копирование {n}"))?;
        out.push(n);
    }
    if out.is_empty() { crate::bail!("Нет .jar-файлов"); }
    Ok(out)
}
fn write_upload(dir: &Path, name: &str, b64: &str) -> Res<()> {
    use base64::Engine;
    let n = core::file_name(name);
    if !n.to_lowercase().ends_with(".jar") { crate::bail!("Можно загружать только .jar"); }
    std::fs::create_dir_all(dir).ctx("")?;
    let data = base64::engine::general_purpose::STANDARD.decode(b64).ctx("Неверные данные")?;
    std::fs::write(dir.join(n), data).ctx("Запись файла")
}

// ---------- моды сервера ----------
pub fn mods_dir(s: &servers::Server) -> PathBuf { Path::new(&s.dir).join(if s.loader == "paper" { "plugins" } else { "mods" }) }
pub fn server_mods(id: &str) -> Res<Value> { let s = servers::get(id)?; let d = mods_dir(&s); Ok(json!({ "dir": d, "list": list_jars(&d) })) }
pub fn toggle_server_mod(id: &str, f: &str) -> Res<()> { toggle(&mods_dir(&servers::get(id)?), f) }
pub fn delete_server_mod(id: &str, f: &str) -> Res<()> { std::fs::remove_file(mods_dir(&servers::get(id)?).join(core::file_name(f))).ctx("Удаление") }
pub fn add_server_files(id: &str, paths: &[String]) -> Res<Vec<String>> { copy_in(&mods_dir(&servers::get(id)?), paths) }
pub fn upload_server_file(id: &str, name: &str, b64: &str) -> Res<()> { write_upload(&mods_dir(&servers::get(id)?), name, b64) }
pub async fn install_server_mod(id: &str, provider: &str, pid: &str) -> Res<Vec<String>> {
    let s = servers::get(id)?;
    let d = mods_dir(&s);
    tokio::fs::create_dir_all(&d).await.ctx("")?;
    providers::install(provider, pid, &s.mc_version, &s.loader, &d).await
}

// ---------- наборы модов клиента ----------
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModSet {
    pub id: String,
    pub name: String,
    #[serde(default)] pub mc_version: String,
    #[serde(default)] pub loader: String,
    #[serde(default)] pub loader_version: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")] pub builtin: bool,
    /// у импортированных наборов — внешняя папка mods (файлы остаются на месте)
    #[serde(default, skip_serializing_if = "Option::is_none")] pub dir: Option<String>,
    #[serde(flatten)] pub extra: Map<String, Value>,
}
fn sets_file() -> PathBuf { DATA.join("clientsets.json") }
static SETS: Lazy<Mutex<Vec<ModSet>>> = Lazy::new(|| {
    let v = core::read_json(&sets_file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_else(|| vec![ModSet {
        id: "vanilla".into(), name: "Ванильный Minecraft".into(), mc_version: String::new(), loader: "vanilla".into(),
        loader_version: String::new(), builtin: true, dir: None, extra: Map::new(),
    }]);
    Mutex::new(v)
});
fn save_sets() { let l = SETS.lock().clone(); let _ = core::write_json(&sets_file(), &l); }
fn get_set(id: &str) -> Res<ModSet> { SETS.lock().iter().find(|s| s.id == id).cloned().ok_or("Набор не найден".into()) }
pub fn set_dir(id: &str) -> PathBuf {
    SETS.lock().iter().find(|s| s.id == id).and_then(|s| s.dir.clone()).map(PathBuf::from).unwrap_or_else(|| DATA.join("clientsets").join(id))
}
pub fn raw_sets() -> Vec<ModSet> { SETS.lock().clone() }
pub fn add_set(mut s: ModSet) -> ModSet { s.id = core::new_id(); SETS.lock().push(s.clone()); save_sets(); s }
pub fn list_sets() -> Value {
    json!(raw_sets().into_iter().map(|s| { let mods = list_jars(&set_dir(&s.id)); let mut v = serde_json::to_value(&s).unwrap(); v["mods"] = json!(mods); v }).collect::<Vec<_>>())
}
pub fn create_set(o: &Value) -> Res<ModSet> {
    let name = match core::arg_str(o, "name") { n if n.trim().is_empty() => "Новый набор".to_string(), n => n.trim().to_string() };
    let s = ModSet { id: core::new_id(), name, mc_version: core::arg_str(o, "mcVersion"), loader: match core::arg_str(o, "loader") { l if l.is_empty() => "fabric".into(), l => l },
        loader_version: core::arg_str(o, "loaderVersion"), builtin: false, dir: None, extra: Map::new() };
    SETS.lock().push(s.clone());
    save_sets();
    std::fs::create_dir_all(set_dir(&s.id)).ctx("")?;
    Ok(s)
}
pub fn update_set(id: &str, p: &Value) -> Res<Value> {
    let mut l = SETS.lock();
    let s = l.iter_mut().find(|s| s.id == id).ok_or("Набор не найден")?;
    if s.builtin { crate::bail!("Встроенный набор нельзя изменить"); }
    for (k, f) in [("name", &mut s.name), ("mcVersion", &mut s.mc_version), ("loader", &mut s.loader), ("loaderVersion", &mut s.loader_version)] {
        if let Some(v) = p.get(k).and_then(|v| v.as_str()) { *f = v.trim().to_string(); }
    }
    let v = serde_json::to_value(&*s).unwrap();
    drop(l);
    save_sets();
    Ok(v)
}
pub fn delete_set(id: &str) -> Res<()> {
    let s = get_set(id)?;
    if s.builtin { crate::bail!("Нельзя удалить встроенный набор"); }
    let dir = set_dir(id);
    SETS.lock().retain(|x| x.id != id);
    save_sets();
    if s.dir.is_none() { let _ = std::fs::remove_dir_all(dir); }
    Ok(())
}
pub async fn install_set_mod(id: &str, provider: &str, pid: &str) -> Res<Vec<String>> {
    let s = get_set(id)?;
    if s.mc_version.is_empty() { crate::bail!("Сначала укажите версию Minecraft набора"); }
    providers::install(provider, pid, &s.mc_version, &s.loader, &set_dir(id)).await
}
pub fn delete_set_mod(id: &str, f: &str) -> Res<()> { std::fs::remove_file(set_dir(id).join(core::file_name(f))).ctx("Удаление") }
pub fn toggle_set_mod(id: &str, f: &str) -> Res<()> { toggle(&set_dir(id), f) }
pub fn add_set_files(id: &str, paths: &[String]) -> Res<Vec<String>> { copy_in(&set_dir(id), paths) }
pub fn upload_set_file(id: &str, name: &str, b64: &str) -> Res<()> { write_upload(&set_dir(id), name, b64) }

/// Набор из модов сервера: копирует всё, кроме модов, помеченных на Modrinth как «только сервер».
pub async fn set_from_server(sid: &str) -> Res<Value> {
    let srv = servers::get(sid)?;
    let src = mods_dir(&srv);
    let set = create_set(&json!({ "name": format!("{} (клиент)", srv.name), "mcVersion": srv.mc_version, "loader": srv.loader, "loaderVersion": srv.loader_version }))?;
    let jars: Vec<String> = list_jars(&src).into_iter().filter(|m| m["enabled"] == true).filter_map(|m| m["file"].as_str().map(String::from)).collect();
    let mut server_only = HashSet::new();
    let r: Res<()> = async {
        let hashes: HashMap<String, String> = jars.iter().filter_map(|f| std::fs::read(src.join(f)).ok().map(|b| (sha1_smol::Sha1::from(b).digest().to_string(), f.clone()))).collect();
        let r = http::post_json(&format!("{MR}/version_files"), &json!({ "hashes": hashes.keys().collect::<Vec<_>>(), "algorithm": "sha1" })).await?;
        let mut by_project = HashMap::new();
        if let Value::Object(m) = r { for (h, v) in m { if let (Some(p), Some(f)) = (v["project_id"].as_str(), hashes.get(&h)) { by_project.insert(p.to_string(), f.clone()); } } }
        if by_project.is_empty() { return Ok(()); }
        let ids: Vec<&String> = by_project.keys().collect();
        let projects = http::get_json(&format!("{MR}/projects?ids={}", enc(&json!(ids)))).await?;
        for p in projects.as_array().cloned().unwrap_or_default() {
            if p["client_side"] == "unsupported" { if let Some(f) = p["id"].as_str().and_then(|i| by_project.get(i)) { server_only.insert(f.clone()); } }
        }
        Ok(())
    }.await;
    let _ = r;
    let dst = set_dir(&set.id);
    for f in &jars { if !server_only.contains(f) { let _ = std::fs::copy(src.join(f), dst.join(f)); } }
    Ok(json!({ "set": set, "skipped": server_only.into_iter().collect::<Vec<_>>() }))
}

/// Применить набор: заменяет .minecraft/mods (чужая папка mods сохраняется в mods_backup_*).
const MARK: &str = ".mcpanel-set";
pub async fn apply_set(id: &str) -> Res<String> {
    let s = get_set(id)?;
    let mc = PathBuf::from(core::set_str("minecraftDir"));
    let mods = mc.join("mods");
    if !mc.exists() { crate::bail!("Папка .minecraft не найдена: {}", mc.display()); }
    if mods.exists() {
        if mods.join(MARK).exists() { tokio::fs::remove_dir_all(&mods).await.ctx("Очистка mods")?; }
        else { tokio::fs::rename(&mods, mc.join(format!("mods_backup_{}", core::now_ms()))).await.ctx("Сохранение старой папки mods")?; }
    }
    if s.loader == "vanilla" { return Ok("Моды отключены — запускайте обычную версию в лаунчере.".into()); }
    tokio::fs::create_dir_all(&mods).await.ctx("")?;
    tokio::fs::write(mods.join(MARK), &s.name).await.ctx("")?;
    let src = set_dir(id);
    let mut n = 0;
    for m in list_jars(&src) {
        if m["enabled"] == true { let f = m["file"].as_str().unwrap(); tokio::fs::copy(src.join(f), mods.join(f)).await.ctx("Копирование")?; n += 1; }
    }
    let msg = install_client_loader(&s).await.unwrap_or_else(|e| format!("Загрузчик не установлен: {e}"));
    emit(json!({ "type": "sets" }));
    Ok(format!("Скопировано модов: {n}. {msg}"))
}
/// Установка загрузчика в клиент: Fabric — профиль напрямую, Forge/NeoForge — открывается официальный установщик.
async fn install_client_loader(s: &ModSet) -> Res<String> {
    if s.loader_version.is_empty() { return Ok("Версия загрузчика не указана — профиль лаунчера не создан.".into()); }
    let mc = PathBuf::from(core::set_str("minecraftDir"));
    match s.loader.as_str() {
        "fabric" => {
            let prof = http::get_json(&format!("https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json", s.mc_version, s.loader_version)).await?;
            let pid = prof["id"].as_str().ok_or("Неверный профиль")?.to_string();
            core::write_json(&mc.join("versions").join(&pid).join(format!("{pid}.json")), &prof)?;
            let lpf = mc.join("launcher_profiles.json");
            let mut lp = core::read_json(&lpf).unwrap_or(json!({ "profiles": {} }));
            let now = chrono::Utc::now().to_rfc3339();
            lp["profiles"][format!("mcpanel-{}", s.id)] = json!({ "name": format!("MC Panel: {}", s.name), "type": "custom", "lastVersionId": pid, "icon": "Furnace", "created": now, "lastUsed": now });
            core::write_json(&lpf, &lp)?;
            Ok(format!("Профиль «MC Panel: {}» добавлен в лаунчер.", s.name))
        }
        l @ ("forge" | "neoforge") => {
            let url = installer::loader_installer_url(l, &s.mc_version, &s.loader_version);
            let f = DATA.join("tmp").join(core::file_name(&url));
            http::download(&url, &f, |_| {}).await?;
            let major = installer::mc_version_info(&s.mc_version).await.ok().and_then(|i| i["javaVersion"]["majorVersion"].as_u64()).unwrap_or(17) as u32;
            let java = installer::ensure_java(major, None).await?;
            // javaw — без консольного окна, установщик Forge показывает собственный GUI
            let javaw = java.with_file_name(if cfg!(windows) { "javaw.exe" } else { "java" });
            let mut args = http::java_proxy_args();
            args.extend(["-jar".into(), f.to_string_lossy().to_string()]);
            std::process::Command::new(if javaw.exists() { javaw } else { java }).args(args).current_dir(f.parent().unwrap()).spawn().ctx("Запуск установщика")?;
            Ok(format!("Открыт установщик {l} — выберите «Install client»."))
        }
        _ => Ok(String::new()),
    }
}

// ---------- обновления ----------
fn target(kind: &str, id: &str) -> Res<(PathBuf, String, String)> {
    if kind == "mods" { let s = servers::get(id)?; Ok((mods_dir(&s), s.mc_version, s.loader)) }
    else { let s = get_set(id)?; Ok((set_dir(id), s.mc_version, s.loader)) }
}
pub async fn check_updates(kind: &str, id: &str) -> Res<Value> {
    let (d, mc, l) = target(kind, id)?;
    if mc.is_empty() { crate::bail!("Не указана версия Minecraft"); }
    providers::check_updates(&d, &mc, &l).await
}
pub async fn apply_updates(kind: &str, id: &str, list: &Value) -> Res<Value> {
    let (d, _, _) = target(kind, id)?;
    let (mut ok, mut failed) = (vec![], vec![]);
    for u in list.as_array().cloned().unwrap_or_default() {
        match providers::apply_update(&d, &u).await { Ok(f) => ok.push(f), Err(e) => failed.push(format!("{}: {e}", u["file"].as_str().unwrap_or(""))) }
    }
    Ok(json!({ "updated": ok, "failed": failed }))
}
