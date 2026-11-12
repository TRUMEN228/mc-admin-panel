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
