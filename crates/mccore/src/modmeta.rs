//! Превью установленных модов и плагинов: название, версия, описание и иконка
//! из самого .jar (fabric/quilt/forge/neoforge/bukkit), иначе — с Modrinth по хешу.
use crate::{self as core, Res};
use crate::http;
use base64::Engine;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

const MR: &str = "https://api.modrinth.com/v2";
const MAX_ICON: u64 = 256 * 1024;
static CACHE: Lazy<Mutex<HashMap<(PathBuf, u64, u64), Value>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn entry(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut e = z.by_name(name.trim_start_matches('/')).ok()?;
    if e.size() > 4 * 1024 * 1024 { return None; }
    let mut b = Vec::new();
    e.read_to_end(&mut b).ok()?;
    Some(b)
}
fn text(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> { entry(z, name).map(|b| String::from_utf8_lossy(&b).to_string()) }
fn icon_data(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let size = z.by_name(name.trim_start_matches('/')).ok()?.size();
    if size > MAX_ICON { return None; }
    let b = entry(z, name)?;
    let mime = if b.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else if b.starts_with(&[0xFF, 0xD8]) { "image/jpeg" } else { return None };
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(b)))
}
/// Значение ключа TOML (упрощённо: строки в "", '' и ''' ''').
fn toml_val(t: &str, key: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"(?ms)^\s*{key}\s*=\s*(?:'''(.*?)'''|"""(.*?)"""|"((?:[^"\\]|\\.)*)"|'([^']*)')"#)).ok()?;
    let c = re.captures(t)?;
    (1..=4).find_map(|i| c.get(i)).map(|m| m.as_str().trim().replace("\\n", "\n").replace("\\\"", "\""))
}
fn yml_val(t: &str, key: &str) -> Option<String> {
    Regex::new(&format!(r#"(?m)^{key}:\s*['"]?(.*?)['"]?\s*$"#)).ok()?.captures(t).map(|c| c[1].to_string()).filter(|s| !s.is_empty())
}

/// Метаданные из самого .jar.
pub fn read_jar(path: &Path) -> Value {
    let Ok(f) = std::fs::File::open(path) else { return json!({}) };
    let Ok(mut z) = zip::ZipArchive::new(f) else { return json!({}) };
    let mut m = Map::new();
    let mut put = |k: &str, v: Option<String>| { if let Some(v) = v.filter(|s| !s.trim().is_empty() && !s.contains("${")) { m.insert(k.into(), json!(v.trim())); } };
    if let Some(j) = text(&mut z, "fabric.mod.json").and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        put("name", j["name"].as_str().map(String::from));
        put("version", j["version"].as_str().map(String::from));
        put("desc", j["description"].as_str().map(String::from));
        // иконка: строка или словарь «размер → путь» (берём самый крупный до 128)
        let icon = match &j["icon"] {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o.iter().filter_map(|(k, v)| Some((k.parse::<u32>().ok()?, v.as_str()?.to_string()))).filter(|x| x.0 <= 128).max_by_key(|x| x.0).or_else(|| o.values().next().and_then(|v| v.as_str()).map(|s| (0, s.to_string()))).map(|x| x.1),
            _ => None,
        };
        put("icon", icon.and_then(|i| icon_data(&mut z, &i)));
    } else if let Some(j) = text(&mut z, "quilt.mod.json").and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        let q = &j["quilt_loader"];
        put("name", q["metadata"]["name"].as_str().map(String::from));
        put("version", q["version"].as_str().map(String::from));
        put("desc", q["metadata"]["description"].as_str().map(String::from));
        put("icon", q["metadata"]["icon"].as_str().and_then(|i| icon_data(&mut z, i)));
    } else if let Some(t) = text(&mut z, "META-INF/neoforge.mods.toml").or_else(|| text(&mut z, "META-INF/mods.toml")) {
        // версия часто подставляется сборкой: ${file.jarVersion} → берём из MANIFEST
        let manifest = text(&mut z, "META-INF/MANIFEST.MF").unwrap_or_default();
        let impl_ver = Regex::new(r"(?m)^Implementation-Version:\s*(\S+)").unwrap().captures(&manifest).map(|c| c[1].to_string());
        put("name", toml_val(&t, "displayName"));
        put("version", toml_val(&t, "version").filter(|v| !v.contains("${")).or(impl_ver));
        put("desc", toml_val(&t, "description"));
        put("icon", toml_val(&t, "logoFile").and_then(|i| icon_data(&mut z, &i)));
    } else if let Some(t) = text(&mut z, "paper-plugin.yml").or_else(|| text(&mut z, "plugin.yml")) {
        put("name", yml_val(&t, "name"));
        put("version", yml_val(&t, "version"));
        put("desc", yml_val(&t, "description"));
    }
    Value::Object(m)
}

fn cached_jar(path: &Path) -> Value {
    let md = std::fs::metadata(path).ok();
    let key = (path.to_path_buf(), md.as_ref().map(|m| m.len()).unwrap_or(0),
        md.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0));
    if let Some(v) = CACHE.lock().get(&key) { return v.clone(); }
    let v = read_jar(path);
    CACHE.lock().insert(key, v.clone());
    v
}

/// Превью всех .jar в папке: { файл: { name, version, desc, icon, url } }.
pub async fn for_dir(dir: &Path) -> Res<Value> {
    let d = dir.to_path_buf();
    let (mut out, missing) = tokio::task::spawn_blocking(move || {
        let mut out = Map::new();
        let mut missing = vec![];
        for f in core::ls(&d).into_iter().filter(|f| { let l = f.to_lowercase(); l.ends_with(".jar") || l.ends_with(".jar.disabled") }) {
            let v = cached_jar(&d.join(&f));
            if v.get("icon").is_none() { missing.push(f.clone()); }
            out.insert(f, v);
        }
        (out, missing)
    }).await.map_err(|e| e.to_string())?;
    // недостающие иконки — с Modrinth по SHA-1 файла
    if !missing.is_empty() {
        let hashes: HashMap<String, String> = missing.iter().filter_map(|f| std::fs::read(dir.join(f)).ok().map(|b| (sha1_smol::Sha1::from(b).digest().to_string(), f.clone()))).collect();
        if let Ok(Value::Object(vers)) = http::post_json(&format!("{MR}/version_files"), &json!({ "hashes": hashes.keys().collect::<Vec<_>>(), "algorithm": "sha1" })).await {
            let by_project: HashMap<String, String> = vers.iter().filter_map(|(h, v)| Some((v["project_id"].as_str()?.to_string(), hashes.get(h)?.clone()))).collect();
            if !by_project.is_empty() {
                let ids: Vec<&String> = by_project.keys().collect();
                if let Ok(Value::Array(ps)) = http::get_json(&format!("{MR}/projects?ids={}", urlencoding::encode(&json!(ids).to_string()))).await {
                    for p in ps {
                        let Some(f) = p["id"].as_str().and_then(|i| by_project.get(i)) else { continue };
                        let Some(Value::Object(m)) = out.get_mut(f) else { continue };
                        if let Some(i) = p["icon_url"].as_str() { m.insert("icon".into(), json!(i)); }
                        m.entry("name").or_insert(p["title"].clone());
                        m.entry("desc").or_insert(p["description"].clone());
                        m.insert("url".into(), json!(format!("https://modrinth.com/{}/{}", p["project_type"].as_str().unwrap_or("mod"), p["slug"].as_str().unwrap_or(""))));
                    }
                }
            }
        }
    }
    Ok(Value::Object(out))
}
