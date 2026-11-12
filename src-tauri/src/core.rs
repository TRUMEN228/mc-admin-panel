//! Общее ядро: пути, настройки, события, фоновые задачи, утилиты.
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

pub use mccore::{Ctx, Res};

/// Передаёт настройки панели (прокси, ключ CurseForge) общей библиотеке.
pub fn sync_mccore() {
    mccore::configure(mccore::Config {
        proxy: set_str("proxy"),
        curseforge_key: set_str("curseforgeKey"),
        tmp_dir: DATA.join("tmp"),
        user_agent: concat!("mc-admin-panel/", env!("CARGO_PKG_VERSION")).into(),
    });
}
#[macro_export]
macro_rules! bail { ($($t:tt)*) => { return Err(format!($($t)*)) }; }

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
pub fn new_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = now_ms() * 64 + SEQ.fetch_add(1, Ordering::Relaxed) % 64;
    radix36(n)
}
fn radix36(mut n: u64) -> String {
    let d = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut s = Vec::new();
    loop { s.push(d[(n % 36) as usize]); n /= 36; if n == 0 { break; } }
    s.reverse();
    String::from_utf8(s).unwrap()
}

// ---------- пути ----------
// Данные панели — %APPDATA%\MC Admin Panel (совместимо с прежней Electron-версией),
// серверы и бэкапы по умолчанию — Документы\MC Servers.
/// Портативный режим: exe называется «…Portable.exe» (или рядом лежит файл `portable`).
/// Всё — данные, Java, серверы, бэкапы — хранится в папке «MC Admin Panel Data» рядом с exe.
pub static PORTABLE: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    let by_name = exe.file_stem()?.to_string_lossy().to_lowercase().contains("portable");
    (by_name || dir.join("portable").exists()).then(|| dir.join("MC Admin Panel Data"))
});
// MCPANEL_DATA позволяет запустить панель с отдельной папкой данных (тесты).
pub static DATA: Lazy<PathBuf> = Lazy::new(|| {
    let d = std::env::var_os("MCPANEL_DATA").map(PathBuf::from)
        .or_else(|| PORTABLE.as_ref().map(|p| p.join("data")))
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("MC Admin Panel"));
    let _ = std::fs::create_dir_all(&d);
    d
});
pub static HOME: Lazy<PathBuf> = Lazy::new(|| {
    if let Some(p) = PORTABLE.as_ref() { return p.clone(); }
    dirs::document_dir().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from(".")).join("MC Servers")
});

pub fn read_json(f: &Path) -> Option<Value> {
    std::fs::read_to_string(f).ok().and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
}
pub fn write_json<T: Serialize>(f: &Path, v: &T) -> Res<()> {
    if let Some(p) = f.parent() { std::fs::create_dir_all(p).ctx("")?; }
    let tmp = f.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(v).ctx("")?).ctx("Запись файла")?;
    std::fs::rename(&tmp, f).ctx("Запись файла")
}

// ---------- настройки ----------
fn defaults() -> Map<String, Value> {
    let mc = dirs::config_dir().unwrap_or_default().join(".minecraft");
    let v = json!({
        "serversDir": HOME.join("servers"),
        "backupsDir": HOME.join("backups"),
        "maxBackups": 5,
        "javaPath": "auto",
        "minecraftDir": mc,
        "defaultRamMb": 4096,
        "proxy": "auto",
        "launchAtLogin": false,
        "minimizeToTray": true,
        "notifications": true,
        "quickCommands": [],
        "webEnabled": false,
        "port": 7777,
        "webLan": false,
        "webPassword": "",
        "mobileEnabled": false,
        "curseforgeKey": "",
        "autoUpdateCheck": true,
    });
    v.as_object().unwrap().clone()
}
fn settings_file() -> PathBuf { DATA.join("settings.json") }
pub static SETTINGS: Lazy<RwLock<Map<String, Value>>> = Lazy::new(|| {
    let mut s = defaults();
    if let Some(Value::Object(saved)) = read_json(&settings_file()) {
        for (k, v) in saved { s.insert(k, v); }
    }
    // папки и параметры, выбранные в установщике
    if let Some(inst) = mccore::take_installer_settings(&DATA) {
        for (k, v) in inst { s.insert(k, v); }
        let _ = write_json(&settings_file(), &s);
    }
    RwLock::new(s)
});
pub fn settings() -> Value { Value::Object(SETTINGS.read().clone()) }
pub fn set_str(k: &str) -> String {
    match SETTINGS.read().get(k) { Some(Value::String(s)) => s.clone(), Some(v) if !v.is_null() => v.to_string(), _ => String::new() }
}
pub fn set_u64(k: &str) -> u64 {
    match SETTINGS.read().get(k) { Some(Value::Number(n)) => n.as_u64().unwrap_or(0), Some(Value::String(s)) => s.parse().unwrap_or(0), _ => 0 }
}
pub fn set_bool(k: &str) -> bool { SETTINGS.read().get(k).and_then(|v| v.as_bool()).unwrap_or(false) }

pub fn save_settings(patch: Value) -> Res<Value> {
    let Value::Object(p) = patch else { bail!("Неверные настройки") };
    let defs = defaults();
    {
        let mut s = SETTINGS.write();
        for (k, mut v) in p {
            // числа из полей ввода приходят строками — приводим к типу по умолчанию
            if let (Some(Value::Number(_)), Value::String(st)) = (defs.get(&k), &v) {
                v = st.trim().parse::<u64>().map(Value::from).unwrap_or(Value::from(0));
            }
            s.insert(k, v);
        }
        write_json(&settings_file(), &*s)?;
    }
    emit(json!({ "type": "settings" }));
    for f in SETTINGS_HOOKS.lock().iter() { f(); }
    Ok(settings())
}
pub static SETTINGS_HOOKS: Lazy<Mutex<Vec<Box<dyn Fn() + Send + Sync>>>> = Lazy::new(|| Mutex::new(Vec::new()));

// ---------- события для UI (Tauri + WebSocket веб-режима) ----------
pub static APP: OnceCell<tauri::AppHandle> = OnceCell::new();
pub static BUS: Lazy<broadcast::Sender<Arc<String>>> = Lazy::new(|| broadcast::channel(4096).0);
pub static LISTENERS: Lazy<Mutex<Vec<Box<dyn Fn(&Value) + Send + Sync>>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn emit(msg: Value) {
    use tauri::Emitter;
    if let Some(app) = APP.get() { let _ = app.emit("msg", &msg); }
    if BUS.receiver_count() > 0 { let _ = BUS.send(Arc::new(msg.to_string())); }
    for l in LISTENERS.lock().iter() { l(&msg); }
}
pub fn on_event(f: impl Fn(&Value) + Send + Sync + 'static) { LISTENERS.lock().push(Box::new(f)); }

// ---------- фоновые задачи с прогрессом ----------
#[derive(Serialize, Clone)]
pub struct JobData { pub id: u64, pub title: String, pub progress: u32, pub status: &'static str, pub log: Vec<String>, pub started: u64 }

pub static JOBS: Lazy<Mutex<BTreeMap<u64, JobData>>> = Lazy::new(|| Mutex::new(BTreeMap::new()));
static JOB_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Job { pub id: u64, last: Arc<Mutex<Instant>> }

impl Job {
    pub fn new(title: impl Into<String>) -> Job {
        let id = JOB_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        let d = JobData { id, title: title.into(), progress: 0, status: "running", log: vec![], started: now_ms() };
        {
            let mut j = JOBS.lock();
            j.insert(id, d);
            while j.len() > 30 { let k = *j.keys().next().unwrap(); j.remove(&k); }
        }
        let job = Job { id, last: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))) };
        job.push();
        job
    }
    fn push(&self) {
        let d = JOBS.lock().get(&self.id).cloned();
        if let Some(d) = d { emit(json!({ "type": "job", "job": d })); }
    }
    pub fn step(&self, text: Option<&str>, progress: Option<f64>) {
        {
            let mut j = JOBS.lock();
            let Some(d) = j.get_mut(&self.id) else { return };
            if let Some(t) = text { d.log.push(t.to_string()); if d.log.len() > 200 { d.log.remove(0); } }
            if let Some(p) = progress { d.progress = p.clamp(0.0, 100.0).round() as u32; }
        }
        let mut last = self.last.lock();
        if text.is_some() || last.elapsed() > Duration::from_millis(400) { *last = Instant::now(); drop(last); self.push(); }
    }
    pub fn log(&self, text: &str) { self.step(Some(text), None) }
    pub fn progress(&self, p: f64) { self.step(None, Some(p)) }
    pub fn done(&self, text: &str) {
        if let Some(d) = JOBS.lock().get_mut(&self.id) { d.status = "done"; d.progress = 100; d.log.push(text.into()); }
        self.push();
    }
    pub fn fail(&self, e: &str) {
        if let Some(d) = JOBS.lock().get_mut(&self.id) { d.status = "error"; d.log.push(format!("Ошибка: {e}")); }
        self.push();
    }
    pub fn finish(&self, r: &Res<()>, ok: &str) { match r { Ok(_) => self.done(ok), Err(e) => self.fail(e) } }
}
pub fn jobs() -> Vec<JobData> { JOBS.lock().values().cloned().collect() }

// ---------- файловые утилиты ----------
pub fn safe_name(s: &str) -> String {
    let r: String = s.chars().map(|c| if c.is_alphanumeric() || " ._-".contains(c) { c } else { '_' }).collect();
    let r = r.trim().to_string();
    if r.is_empty() { "server".into() } else { r }
}
pub fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}
pub fn dir_size(p: &Path) -> u64 {
    walkdir::WalkDir::new(p).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}
pub fn ls(p: &Path) -> Vec<String> {
    std::fs::read_dir(p).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default()
}
/// Команда без всплывающего консольного окна.
pub fn command(prog: impl AsRef<std::ffi::OsStr>) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut c = tokio::process::Command::new(prog);
    #[cfg(windows)]
    c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    c
}
pub fn open_path(p: &str) -> Res<()> {
    let path = PathBuf::from(p);
    if !path.exists() { std::fs::create_dir_all(&path).ctx("Папка не найдена")?; }
    tauri_plugin_opener::open_path(path, None::<&str>).ctx("Открытие папки")
}
pub fn arg_str(a: &Value, k: &str) -> String {
    match a.get(k) { Some(Value::String(s)) => s.clone(), Some(Value::Null) | None => String::new(), Some(v) => v.to_string() }
}
