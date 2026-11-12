//! Пути, настройки, события и фоновые задачи сборщика.
use mccore::{Ctx, Res};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0) }
pub fn new_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let mut n = now_ms() * 64 + SEQ.fetch_add(1, Ordering::Relaxed) % 64;
    let d = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut s = Vec::new();
    loop { s.push(d[(n % 36) as usize]); n /= 36; if n == 0 { break; } }
    s.reverse();
    String::from_utf8(s).unwrap()
}

/// Портативный режим: «Portable» в имени exe — данные рядом с программой.
pub static PORTABLE: Lazy<bool> = Lazy::new(|| {
    std::env::current_exe().ok().and_then(|e| e.file_stem().map(|s| s.to_string_lossy().to_lowercase().contains("portable"))).unwrap_or(false)
});
pub static DATA: Lazy<PathBuf> = Lazy::new(|| {
    let d = std::env::var_os("MCCOLLECTOR_DATA").map(PathBuf::from)
        .or_else(|| PORTABLE.then(|| std::env::current_exe().ok()?.parent().map(|p| p.join("MC Mod Collector Data"))).flatten())
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("MC Mod Collector"));
    let _ = std::fs::create_dir_all(&d);
    d
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
    json!({
        "minecraftDir": dirs::config_dir().unwrap_or_default().join(".minecraft"),
        "exportDir": dirs::download_dir().or_else(dirs::desktop_dir).unwrap_or_default(),
        "proxy": "auto", "curseforgeKey": "", "theme": "system", "autoUpdateCheck": true, "author": "",
    }).as_object().unwrap().clone()
}
fn settings_file() -> PathBuf { DATA.join("settings.json") }
pub static SETTINGS: Lazy<RwLock<Map<String, Value>>> = Lazy::new(|| {
    let mut s = defaults();
    if let Some(Value::Object(saved)) = read_json(&settings_file()) { for (k, v) in saved { s.insert(k, v); } }
    // папка игры, выбранная в установщике
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
pub fn save_settings(patch: Value) -> Res<Value> {
    let Value::Object(p) = patch else { mccore::bail!("Неверные настройки") };
    { let mut s = SETTINGS.write(); for (k, v) in p { s.insert(k, v); } write_json(&settings_file(), &*s)?; }
    sync_mccore();
    Ok(settings())
}
pub fn sync_mccore() {
    mccore::configure(mccore::Config {
        proxy: set_str("proxy"), curseforge_key: set_str("curseforgeKey"), tmp_dir: DATA.join("tmp"),
        user_agent: concat!("mc-mod-collector/", env!("CARGO_PKG_VERSION")).into(),
    });
}

// ---------- события и задачи ----------
pub static APP: OnceCell<tauri::AppHandle> = OnceCell::new();
pub fn emit(msg: Value) {
    use tauri::Emitter;
    if let Some(a) = APP.get() { let _ = a.emit("msg", &msg); }
    #[cfg(debug_assertions)]
    crate::dev::emit(&msg);
}

#[derive(Serialize, Clone)]
pub struct JobData { pub id: u64, pub title: String, pub progress: u32, pub status: &'static str, pub log: Vec<String> }
static JOBS: Lazy<Mutex<BTreeMap<u64, JobData>>> = Lazy::new(|| Mutex::new(BTreeMap::new()));
static JOB_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Job { pub id: u64, last: Arc<Mutex<Instant>> }
impl Job {
    pub fn new(title: impl Into<String>) -> Job {
        let id = JOB_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        JOBS.lock().insert(id, JobData { id, title: title.into(), progress: 0, status: "running", log: vec![] });
        let j = Job { id, last: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1))) };
        j.push();
        j
    }
    fn push(&self) { if let Some(d) = JOBS.lock().get(&self.id).cloned() { emit(json!({ "type": "job", "job": d })); } }
    pub fn step(&self, text: Option<&str>, progress: Option<f64>) {
        {
            let mut j = JOBS.lock();
            let Some(d) = j.get_mut(&self.id) else { return };
            if let Some(t) = text { d.log.push(t.into()); if d.log.len() > 100 { d.log.remove(0); } }
            if let Some(p) = progress { d.progress = p.clamp(0.0, 100.0).round() as u32; }
        }
        let mut last = self.last.lock();
        if text.is_some() || last.elapsed() > Duration::from_millis(300) { *last = Instant::now(); drop(last); self.push(); }
    }
    pub fn finish(&self, r: &Res<String>) {
        if let Some(d) = JOBS.lock().get_mut(&self.id) {
            match r { Ok(m) => { d.status = "done"; d.progress = 100; d.log.push(m.clone()); } Err(e) => { d.status = "error"; d.log.push(format!("Ошибка: {e}")); } }
        }
        self.push();
    }
}
pub fn jobs() -> Vec<JobData> { JOBS.lock().values().cloned().collect() }
pub fn arg(a: &Value, k: &str) -> String { match a.get(k) { Some(Value::String(s)) => s.clone(), Some(Value::Null) | None => String::new(), Some(v) => v.to_string() } }

/// Mica доступна в Windows 11 (сборка 22000+).
pub fn mica_supported() -> bool {
    static V: Lazy<bool> = Lazy::new(|| {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            if let Ok(o) = std::process::Command::new("reg").args(["query", r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion", "/v", "CurrentBuildNumber"]).creation_flags(0x0800_0000).output() {
                let s = String::from_utf8_lossy(&o.stdout);
                if let Some(n) = s.split_whitespace().last().and_then(|x| x.parse::<u32>().ok()) { return n >= 22000; }
            }
        }
        false
    });
    *V
}
