//! Общий код MC Admin Panel и MC Mod Collector.
pub mod http;
pub mod modmeta;
pub mod modpack;
pub mod providers;
pub mod versions;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};

pub type Res<T> = Result<T, String>;

/// Превращает любую ошибку в строку для интерфейса.
pub trait Ctx<T> {
    fn ctx(self, what: &str) -> Res<T>;
}
impl<T, E: std::fmt::Display> Ctx<T> for Result<T, E> {
    fn ctx(self, what: &str) -> Res<T> {
        self.map_err(|e| if what.is_empty() { e.to_string() } else { format!("{what}: {e}") })
    }
}
#[macro_export]
macro_rules! bail { ($($t:tt)*) => { return Err(format!($($t)*)) }; }

/// Настройки, которые задаёт приложение.
#[derive(Clone, Default)]
pub struct Config {
    /// «auto» — системный прокси Windows или HTTPS_PROXY; пусто — напрямую; иначе адрес прокси
    pub proxy: String,
    pub curseforge_key: String,
    /// папка для временных загрузок
    pub tmp_dir: PathBuf,
    pub user_agent: String,
}
static CONFIG: Lazy<RwLock<Config>> = Lazy::new(|| RwLock::new(Config { proxy: "auto".into(), tmp_dir: std::env::temp_dir(), user_agent: "mccore".into(), ..Default::default() }));
pub fn configure(c: Config) { *CONFIG.write() = c; }
pub fn config() -> Config { CONFIG.read().clone() }

/// Прогресс длительной операции: (сообщение, процент).
pub type Progress<'a> = &'a (dyn Fn(Option<&str>, Option<f64>) + Send + Sync);

pub fn file_name(p: &str) -> String {
    Path::new(p).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}
pub fn ls(p: &Path) -> Vec<String> {
    std::fs::read_dir(p).map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default()
}
pub fn safe_name(s: &str) -> String {
    let r: String = s.chars().map(|c| if c.is_alphanumeric() || " ._-".contains(c) { c } else { '_' }).collect();
    let r = r.trim().to_string();
    if r.is_empty() { "file".into() } else { r }
}
pub fn loader_title(l: &str) -> &'static str {
    match l { "fabric" => "Fabric", "forge" => "Forge", "neoforge" => "NeoForge", "paper" => "Paper", "quilt" => "Quilt", _ => "Vanilla" }
}

pub fn sha512_hex(b: &[u8]) -> String { use sha2::Digest; hex::encode(sha2::Sha512::digest(b)) }
pub fn sha1_hex(b: &[u8]) -> String { sha1_smol::Sha1::from(b).digest().to_string() }

/// Настройки, выбранные в установщике (installer.json в папке данных, UTF-16LE или UTF-8).
/// Читаются один раз: файл удаляется.
pub fn take_installer_settings(data_dir: &Path) -> Option<serde_json::Map<String, serde_json::Value>> {
    let f = data_dir.join("installer.json");
    let b = std::fs::read(&f).ok()?;
    let _ = std::fs::remove_file(&f);
    let text = if b.starts_with(&[0xFF, 0xFE]) {
        let u: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else {
        String::from_utf8_lossy(&b).trim_start_matches('\u{feff}').to_string()
    };
    match serde_json::from_str::<serde_json::Value>(text.trim_matches('\0').trim()) {
        Ok(serde_json::Value::Object(mut m)) => { m.remove("installer"); Some(m) }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn installer_settings_utf16() {
        let d = std::env::temp_dir().join(format!("mccore-inst-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let json = r#"{"minecraftDir":"C:\\Users\\Денис\\AppData\\Roaming\\.minecraft","launchAtLogin":true,"installer":true}"#;
        let mut b = vec![0xFF, 0xFE];
        for u in json.encode_utf16() { b.extend_from_slice(&u.to_le_bytes()); }
        std::fs::write(d.join("installer.json"), b).unwrap();
        let m = super::take_installer_settings(&d).unwrap();
        assert_eq!(m["minecraftDir"], r"C:\Users\Денис\AppData\Roaming\.minecraft");
        assert_eq!(m["launchAtLogin"], true);
        assert!(!m.contains_key("installer"));
        assert!(!d.join("installer.json").exists(), "файл читается один раз");
        let _ = std::fs::remove_dir_all(&d);
    }
}
