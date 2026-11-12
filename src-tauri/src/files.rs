//! Файлы сервера: просмотр папок и редактирование текстовых конфигов.
use crate::core::{Ctx, Res};
use crate::servers;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const TEXT_EXT: &[&str] = &["properties", "json", "json5", "jsonc", "toml", "yml", "yaml", "txt", "cfg", "conf", "ini", "mcmeta", "snbt", "log", "md", "csv", "lang", "js", "zs", "sk", "xml", "bat", "sh", "list"];
const MAX: u64 = 2 * 1024 * 1024;

pub fn editable(name: &str) -> bool {
    let l = name.to_lowercase();
    l.rsplit_once('.').map(|(_, e)| TEXT_EXT.contains(&e)).unwrap_or(false) || l == "eula.txt"
}

/// Путь внутри папки сервера; выход за её пределы запрещён.
pub fn resolve(id: &str, rel: &str) -> Res<(PathBuf, PathBuf)> {
    let s = servers::get(id)?;
    let root = Path::new(&s.dir).canonicalize().ctx("Папка сервера не найдена")?;
    let rel = rel.replace('\\', "/");
    let rel = rel.trim_matches('/');
    if rel.split('/').any(|c| c == "..") { crate::bail!("Недопустимый путь"); }
    let p = if rel.is_empty() { root.clone() } else { root.join(rel) };
    let canon = p.canonicalize().unwrap_or(p.clone());
    if !canon.starts_with(&root) { crate::bail!("Недопустимый путь"); }
    Ok((root, p))
}

pub fn list(id: &str, rel: &str) -> Res<Value> {
    let (_, dir) = resolve(id, rel)?;
    let mut v: Vec<Value> = std::fs::read_dir(&dir).ctx("Чтение папки")?.flatten().filter_map(|e| {
        let m = e.metadata().ok()?;
        let name = e.file_name().to_string_lossy().to_string();
        let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64).unwrap_or(0);
        Some(json!({ "name": name, "dir": m.is_dir(), "size": m.len(), "time": t, "editable": !m.is_dir() && editable(&name) && m.len() <= MAX }))
    }).collect();
    v.sort_by(|a, b| b["dir"].as_bool().cmp(&a["dir"].as_bool()).then(a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase())));
    Ok(json!(v))
}

pub fn read(id: &str, rel: &str) -> Res<Value> {
    let (_, p) = resolve(id, rel)?;
    let m = std::fs::metadata(&p).ctx("Файл не найден")?;
    if m.len() > MAX { crate::bail!("Файл слишком большой для редактора (больше 2 МБ)"); }
    let b = std::fs::read(&p).ctx("Чтение файла")?;
    let text = String::from_utf8(b).map_err(|_| "Это не текстовый файл (или не UTF-8)".to_string())?;
    Ok(json!({ "text": text, "size": m.len() }))
}

pub fn write(id: &str, rel: &str, text: &str) -> Res<()> {
    let (_, p) = resolve(id, rel)?;
    if !p.is_file() { crate::bail!("Файл не найден"); }
    if !editable(&p.file_name().unwrap_or_default().to_string_lossy()) { crate::bail!("Этот тип файлов не редактируется"); }
    // резервная копия предыдущей версии на случай ошибки в конфиге
    let _ = std::fs::copy(&p, p.with_extension(format!("{}.bak", p.extension().and_then(|e| e.to_str()).unwrap_or(""))));
    std::fs::write(&p, text).ctx("Запись файла")
}
