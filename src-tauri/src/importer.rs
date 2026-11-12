//! Импорт существующих серверов и папок модов (на месте, без копирования).
use crate::core::{self, Res};
use crate::mods::{self, ModSet};
use crate::servers::{self, Launch, Server};
use crate::installer;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Прочитать из zip/jar записи, имена которых проходят фильтр.
fn read_zip(file: &Path, want: impl Fn(&str) -> bool) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(f) = std::fs::File::open(file) else { return out };
    let Ok(mut z) = zip::ZipArchive::new(f) else { return out };
    for i in 0..z.len() {
        let Ok(mut e) = z.by_index(i) else { continue };
        let n = e.name().to_string();
        if !want(&n) { continue; }
        let mut s = String::new();
        if e.read_to_string(&mut s).is_ok() { out.insert(n, s); }
    }
    out
}
fn props(txt: &str) -> HashMap<String, String> { servers::parse_props(txt).into_iter().map(|(k, v)| (k, v.trim().to_string())).collect() }
fn ver_in_name(s: &str) -> Option<String> {
    Regex::new(r"\b(1\.\d+(?:\.\d+)?|2\d\.\d+(?:\.\d+)?)\b").unwrap().captures(s).map(|c| c[1].to_string())
}
fn newest(mut v: Vec<String>) -> Option<String> {
    v.sort_by(|a, b| natural(b, a));
    v.into_iter().next()
}
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    let p = |s: &str| s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    p(a).cmp(&p(b))
}
fn json_id(s: Option<&String>) -> Option<String> {
    s.and_then(|t| serde_json::from_str::<Value>(t).ok()).and_then(|v| v["id"].as_str().map(String::from))
}

pub async fn detect_server(dir: &Path) -> Option<Server> {
    let files = core::ls(dir);
    if !files.iter().any(|f| f == "server.properties" || f == "eula.txt") { return None; }
    let bat = files.iter().filter(|f| { let l = f.to_lowercase(); l.ends_with(".bat") || l.ends_with(".cmd") || l.ends_with(".sh") })
        .filter_map(|f| std::fs::read(dir.join(f)).ok().map(|b| String::from_utf8_lossy(&b).to_string()))
        .find(|t| t.to_lowercase().contains("java")).unwrap_or_default();
    let ram = Regex::new(r"-Xmx(\d+)([GgMm])").unwrap().captures(&bat)
        .map(|c| c[1].parse::<u32>().unwrap_or(4) * if c[2].eq_ignore_ascii_case("g") { 1024 } else { 1 })
        .unwrap_or(core::set_u64("defaultRamMb") as u32);
    let jvm = Regex::new(r#"(?i)java(?:\.exe)?"?\s+(.*?)\s+(?:-jar|@)"#).unwrap().captures(&bat).map(|c| c[1].to_string()).unwrap_or_default();
    let jvm_args = jvm.split_whitespace().filter(|a| a.starts_with('-') && !a.starts_with("-Xms") && !a.starts_with("-Xmx")).collect::<Vec<_>>().join(" ");
    let name = dir.file_name()?.to_string_lossy().to_string();
    let mut srv = Server {
        id: String::new(), name: name.clone(), mc_version: String::new(), loader: "vanilla".into(), loader_version: String::new(),
        dir: dir.to_string_lossy().to_string(), port: 25565, ram, jvm_args, backup_dir: String::new(), max_backups: 0, auto_backup_min: 0,
        auto_restart: true, auto_start: false, installed: true, imported: true, created: 0, java_major: 0, launch: Launch::default(), jvm_preset: String::new(), extra: Map::new(),
    };

    // чем запускается сервер
    let args_file = Regex::new(r"@(libraries[\\/]\S+?_args\.txt)").unwrap().captures(&bat).map(|c| c[1].to_string());
    let jar_in_bat = Regex::new(r#"(?i)-jar\s+"?([^"\s]+\.jar)"?"#).unwrap().captures(&bat).map(|c| c[1].to_string());
    if let Some(a) = &args_file {
        srv.launch = Launch { jar: None, args: Some(a.replace('/', std::path::MAIN_SEPARATOR_STR)) };
    } else {
        let re = Regex::new(r"(?i)^(fabric-server|forge|neoforge|paper|purpur).*\.jar$").unwrap();
        let jar = jar_in_bat.filter(|j| files.contains(j)).or_else(|| {
            let mut cands = vec!["fabric-server-launch.jar".to_string()];
            cands.extend(files.iter().filter(|f| re.is_match(f)).cloned());
            cands.push("server.jar".into());
            cands.into_iter().find(|f| files.contains(f))
        })?;
        srv.launch = Launch { jar: Some(jar), args: None };
    }

    // ядро и версии
    if let Some(a) = &args_file {
        if let Some(m) = Regex::new(r"neoforged[\\/]neoforge[\\/]([^\\/]+)").unwrap().captures(a) { srv.loader = "neoforge".into(); srv.loader_version = m[1].into(); }
        else if let Some(f) = Regex::new(r"minecraftforge[\\/]forge[\\/]([^\\/]+?)-([^\\/]+)").unwrap().captures(a) { srv.loader = "forge".into(); srv.mc_version = f[1].into(); srv.loader_version = f[2].into(); }
    } else {
        let jar = srv.launch.jar.clone().unwrap();
        let meta = read_zip(&dir.join(&jar), |n| matches!(n, "install.properties" | "version.json" | "META-INF/MANIFEST.MF" | "version_list.json"));
        let inst = props(meta.get("install.properties").map(String::as_str).unwrap_or(""));
        let main = meta.get("META-INF/MANIFEST.MF").and_then(|m| Regex::new(r"Main-Class:\s*(\S+)").unwrap().captures(m).map(|c| c[1].to_string())).unwrap_or_default();
        let lj = jar.to_lowercase();
        if inst.contains_key("game-version") || main.contains("fabricmc") {
            srv.loader = "fabric".into();
            srv.mc_version = inst.get("game-version").cloned().unwrap_or_default();
            srv.loader_version = inst.get("fabric-loader-version").cloned().or_else(|| newest(core::ls(&dir.join("libraries/net/fabricmc/fabric-loader")))).unwrap_or_default();
            if srv.mc_version.is_empty() {
                // старый лаунчер: версия — в vanilla-jar, указанном в fabric-server-launcher.properties
                let vj = std::fs::read_to_string(dir.join("fabric-server-launcher.properties")).ok().and_then(|t| props(&t).get("serverJar").cloned()).unwrap_or("server.jar".into());
                srv.mc_version = json_id(read_zip(&dir.join(vj), |n| n == "version.json").get("version.json")).unwrap_or_default();
            }
        } else if lj.contains("paper") || lj.contains("purpur") || meta.contains_key("version_list.json") { srv.loader = "paper".into(); }
        else if lj.contains("forge") { srv.loader = if lj.contains("neoforge") { "neoforge" } else { "forge" }.into(); }
        if srv.mc_version.is_empty() { srv.mc_version = json_id(meta.get("version.json")).unwrap_or_default(); }
        if srv.loader == "forge" && srv.loader_version.is_empty() {
            if let Some(f) = Regex::new(r"(?i)forge-([\d.]+)-([\d.]+)").unwrap().captures(&jar) {
                if srv.mc_version.is_empty() { srv.mc_version = f[1].into(); }
                srv.loader_version = f[2].trim_end_matches('.').into();
            }
        }
    }
    if srv.loader == "neoforge" && srv.mc_version.is_empty() { srv.mc_version = installer::neo_to_mc(&srv.loader_version); }
    if srv.mc_version.is_empty() {
        let v = core::ls(&dir.join("versions"));
        srv.mc_version = if v.len() == 1 { v[0].clone() } else { ver_in_name(&name).unwrap_or_default() };
    }
    srv.port = std::fs::read_to_string(dir.join("server.properties")).ok().and_then(|t| props(&t).get("server-port").and_then(|p| p.parse().ok())).unwrap_or(25565);
    srv.java_major = match installer::mc_version_info(&srv.mc_version).await {
        Ok(i) => i["javaVersion"]["majorVersion"].as_u64().unwrap_or(8) as u32,
        Err(_) => 21,
    };
    Some(srv)
}

fn subdirs(root: &Path) -> Vec<PathBuf> { core::ls(root).into_iter().map(|d| root.join(d)).filter(|d| d.is_dir()).collect() }

pub async fn import_servers(root: &str) -> Res<Value> {
    let root = std::path::absolute(root).unwrap_or_else(|_| PathBuf::from(root));
    let dirs = if detect_server(&root).await.is_some() { vec![root.clone()] } else { subdirs(&root) };
    let (mut added, mut skipped) = (vec![], vec![]);
    for d in dirs {
        let n = d.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let ds = d.to_string_lossy().to_string();
        if servers::all_raw().iter().any(|s| servers::same_path(&s.dir, &ds)) { skipped.push(json!({ "name": n, "reason": "уже в панели" })); continue; }
        match detect_server(&d).await {
            None => skipped.push(json!({ "name": n, "reason": "не похоже на сервер" })),
            Some(s) => {
                let s = servers::add_imported(s);
                added.push(format!("{} — {} {} {}", s.name, s.loader, s.mc_version, s.loader_version).trim().to_string());
            }
        }
    }
    Ok(json!({ "added": added, "skipped": skipped }))
}

fn detect_mods_loader(dir: &Path) -> String {
    let mut count: HashMap<&str, u32> = HashMap::new();
    for j in core::ls(dir).into_iter().filter(|f| f.ends_with(".jar")).take(12) {
        let m = read_zip(&dir.join(j), |n| matches!(n, "fabric.mod.json" | "quilt.mod.json" | "META-INF/mods.toml" | "META-INF/neoforge.mods.toml" | "mcmod.info"));
        let l = if m.contains_key("fabric.mod.json") || m.contains_key("quilt.mod.json") { Some("fabric") }
            else if m.contains_key("META-INF/neoforge.mods.toml") { Some("neoforge") }
            else if m.contains_key("META-INF/mods.toml") || m.contains_key("mcmod.info") { Some("forge") } else { None };
        if let Some(l) = l { *count.entry(l).or_default() += 1; }
    }
    count.into_iter().max_by_key(|x| x.1).map(|x| x.0).unwrap_or("fabric").to_string()
}
pub fn import_sets(root: &str) -> Res<Value> {
    let root = std::path::absolute(root).unwrap_or_else(|_| PathBuf::from(root));
    let is_pack = |d: &Path| { let l = core::ls(d); l.iter().any(|f| f == "mods" || f.ends_with(".jar")) };
    let dirs = if is_pack(&root) { vec![root.clone()] } else { subdirs(&root) };
    let (mut added, mut skipped) = (vec![], vec![]);
    for d in dirs {
        let n = d.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let md = if core::ls(&d).iter().any(|f| f == "mods") { d.join("mods") } else { d.clone() };
        let mds = md.to_string_lossy().to_string();
        if mods::raw_sets().iter().any(|s| s.dir.as_deref().map(|x| servers::same_path(x, &mds)).unwrap_or(false)) { skipped.push(json!({ "name": n, "reason": "уже в панели" })); continue; }
        let cnt = core::ls(&md).iter().filter(|f| f.ends_with(".jar") || f.ends_with(".jar.disabled")).count();
        if cnt == 0 { skipped.push(json!({ "name": n, "reason": "нет модов" })); continue; }
        let loader = detect_mods_loader(&md);
        mods::add_set(ModSet { id: String::new(), name: n.clone(), mc_version: ver_in_name(&n).unwrap_or_default(), loader: loader.clone(), loader_version: String::new(), builtin: false, dir: Some(mds), extra: Map::new() });
        added.push(format!("{n} — {loader}, модов: {cnt}"));
    }
    Ok(json!({ "added": added, "skipped": skipped }))
}
