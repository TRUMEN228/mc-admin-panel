//! Сборки Modrinth (.mrpack): установка на сервер и создание набора для клиента.
use crate::core::{self, Ctx, Job, Res, DATA};
use crate::http;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MR: &str = "https://api.modrinth.com/v2";

pub struct Pack { pub path: PathBuf, pub index: Value, pub name: String, pub mc: String, pub loader: String, pub loader_version: String }

/// Путь внутри сборки не должен выходить за пределы папки назначения.
pub fn safe_rel(p: &str) -> Option<PathBuf> {
    let pb = PathBuf::from(p.replace('\\', "/"));
    if pb.components().all(|c| matches!(c, Component::Normal(_))) && !p.is_empty() { Some(pb) } else { None }
}

pub fn read(path: &Path) -> Res<Pack> {
    let f = std::fs::File::open(path).ctx("Открытие сборки")?;
    let mut z = zip::ZipArchive::new(f).ctx("Это не архив .mrpack")?;
    let mut s = String::new();
    z.by_name("modrinth.index.json").ctx("В архиве нет modrinth.index.json")?.read_to_string(&mut s).ctx("")?;
    let index: Value = serde_json::from_str(&s).ctx("Неверный modrinth.index.json")?;
    let deps = &index["dependencies"];
    let mc = deps["minecraft"].as_str().ok_or("В сборке не указана версия Minecraft")?.to_string();
    let (loader, lv) = if let Some(v) = deps["fabric-loader"].as_str() { ("fabric", v) }
        else if let Some(v) = deps["neoforge"].as_str() { ("neoforge", v) }
        else if let Some(v) = deps["forge"].as_str() { ("forge", v) }
        else if deps["quilt-loader"].is_string() { crate::bail!("Сборки на Quilt пока не поддерживаются") }
        else { ("vanilla", "") };
    Ok(Pack { path: path.into(), name: index["name"].as_str().unwrap_or("Сборка").to_string(), mc, loader: loader.into(), loader_version: lv.into(), index })
}

pub fn summary(p: &Pack) -> Value {
    let files = p.index["files"].as_array().cloned().unwrap_or_default();
    let server = files.iter().filter(|f| f["env"]["server"] != "unsupported").count();
    let client = files.iter().filter(|f| f["env"]["client"] != "unsupported").count();
    json!({ "name": p.name, "version": p.index["versionId"], "mc": p.mc, "loader": p.loader, "loaderVersion": p.loader_version,
        "serverFiles": server, "clientFiles": client, "summary": p.index["summary"] })
}

/// Скачивает .mrpack проекта Modrinth (последнюю версию или указанную).
pub async fn download(project: &str, version: Option<&str>) -> Res<PathBuf> {
    let v = match version {
        Some(id) if !id.is_empty() => http::get_json(&format!("{MR}/version/{id}")).await?,
        _ => http::get_json(&format!("{MR}/project/{project}/version")).await?.get(0).cloned().ok_or("У сборки нет версий")?,
    };
    let files = v["files"].as_array().cloned().unwrap_or_default();
    let f = files.iter().find(|x| x["primary"] == true).or(files.first()).ok_or("Нет файла сборки")?;
    let dest = DATA.join("tmp").join(core::file_name(f["filename"].as_str().unwrap_or("pack.mrpack")));
    http::download(f["url"].as_str().unwrap_or(""), &dest, |_| {}).await?;
    Ok(dest)
}

/// Скачивает файлы сборки для стороны («server» или «client») в dest. only_mods — только папка mods.
pub async fn install_files(p: &Pack, side: &str, dest: &Path, only_mods: bool, job: &Job, from: f64, to: f64) -> Res<usize> {
    let files: Vec<Value> = p.index["files"].as_array().cloned().unwrap_or_default().into_iter()
        .filter(|f| f["env"][side] != "unsupported").collect();
    let total = files.len().max(1);
    let mut n = 0;
    for (i, f) in files.iter().enumerate() {
        let path = f["path"].as_str().unwrap_or("");
        let Some(rel) = safe_rel(path) else { job.log(&format!("Пропущен небезопасный путь: {path}")); continue };
        let rel = if only_mods {
            match rel.strip_prefix("mods") { Ok(r) => r.to_path_buf(), Err(_) => continue }
        } else { rel };
        let url = f["downloads"][0].as_str().ok_or(format!("Нет ссылки для {path}"))?;
        job.step(Some(&format!("{} ({}/{})", core::file_name(path), i + 1, total)), Some(from + (to - from) * i as f64 / total as f64));
        http::download(url, &dest.join(rel), |_| {}).await.map_err(|e| format!("{path}: {e}"))?;
        n += 1;
    }
    Ok(n)
}

/// Распаковывает overrides/ (и server-overrides/ для сервера) в папку назначения.
pub async fn extract_overrides(p: &Pack, dest: &Path, server: bool) -> Res<usize> {
    let path = p.path.clone();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || -> Res<usize> {
        let mut z = zip::ZipArchive::new(std::fs::File::open(&path).ctx("")?).ctx("")?;
        let prefixes: &[&str] = if server { &["overrides/", "server-overrides/"] } else { &["overrides/", "client-overrides/"] };
        let mut n = 0;
        for i in 0..z.len() {
            let mut e = z.by_index(i).ctx("")?;
            let name = e.name().to_string();
            let Some(rest) = prefixes.iter().find_map(|p| name.strip_prefix(p)) else { continue };
            let Some(rel) = safe_rel(rest) else { continue };
            let out = dest.join(rel);
            if e.is_dir() { std::fs::create_dir_all(&out).ctx("")?; continue; }
            if let Some(par) = out.parent() { std::fs::create_dir_all(par).ctx("")?; }
            let mut f = std::fs::File::create(&out).ctx("Запись файла")?;
            std::io::copy(&mut e, &mut f).ctx("Распаковка")?;
            n += 1;
        }
        Ok(n)
    }).await.ctx("")?
}
