//! Версии Minecraft/загрузчиков, Java (Adoptium) и установка ядра сервера.
use crate::core::{self, set_str, Ctx, Job, Res, DATA};
use crate::http;
use crate::servers::{Launch, Server};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};

const MOJANG: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

static CACHE: Lazy<Mutex<HashMap<String, (Instant, Value)>>> = Lazy::new(|| Mutex::new(HashMap::new()));
async fn cached<F: Future<Output = Res<Value>>>(k: &str, ttl: u64, f: impl FnOnce() -> F) -> Res<Value> {
    if let Some((t, v)) = CACHE.lock().get(k) { if t.elapsed() < Duration::from_secs(ttl) { return Ok(v.clone()); } }
    let v = f().await?;
    CACHE.lock().insert(k.into(), (Instant::now(), v.clone()));
    Ok(v)
}
async fn maven_versions(url: &str) -> Res<Value> {
    let t = http::get_text(url).await?;
    let re = regex::Regex::new(r"<version>([^<]+)</version>").unwrap();
    let mut v: Vec<String> = re.captures_iter(&t).map(|c| c[1].to_string()).collect();
    v.reverse();
    Ok(json!(v))
}
fn strs(v: &Value) -> Vec<String> { v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() }

pub async fn mc_versions(snapshots: bool) -> Res<Value> {
    let m = cached("mojang", 3600, || http::get_json(MOJANG)).await?;
    let list: Vec<Value> = m["versions"].as_array().cloned().unwrap_or_default().into_iter()
        .filter(|v| snapshots || v["type"] == "release")
        .map(|v| json!({ "id": v["id"], "type": v["type"], "time": v["releaseTime"] })).collect();
    Ok(json!(list))
}
pub async fn mc_version_info(id: &str) -> Res<Value> {
    let m = cached("mojang", 3600, || http::get_json(MOJANG)).await?;
    let v = m["versions"].as_array().and_then(|a| a.iter().find(|x| x["id"] == id)).cloned().ok_or(format!("Неизвестная версия {id}"))?;
    let url = v["url"].as_str().unwrap_or("").to_string();
    cached(&format!("ver:{id}"), 86400, || async move { http::get_json(&url).await }).await
}

/// 21.1.77 -> 1.21.1, 20.4.x -> 1.20.4, 21.0.x -> 1.21, 26.1.0.x -> 26.1
pub fn neo_to_mc(v: &str) -> String {
    let p: Vec<&str> = v.split(['.', '-']).collect();
    let a = p.first().copied().unwrap_or("0");
    let b = p.get(1).copied().unwrap_or("0");
    if a.parse::<u32>().unwrap_or(0) >= 26 {
        return match p.get(2) { Some(c) if *c != "0" => format!("{a}.{b}.{c}"), _ => format!("{a}.{b}") };
    }
    if b == "0" { format!("1.{a}") } else { format!("1.{a}.{b}") }
}
fn cmp_num(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parse(a).cmp(&parse(b))
}

pub async fn loader_versions(loader: &str, mc: &str) -> Res<Value> {
    Ok(match loader {
        "fabric" => {
            let l = cached("fabric", 3600, || http::get_json("https://meta.fabricmc.net/v2/versions/loader")).await?;
            json!(l.as_array().map(|a| a.iter().filter_map(|x| x["version"].as_str()).collect::<Vec<_>>()).unwrap_or_default())
        }
        "forge" => {
            let all = cached("forge", 3600, || maven_versions("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")).await?;
            let pre = format!("{mc}-");
            json!(strs(&all).into_iter().filter(|v| v.starts_with(&pre)).map(|v| v[pre.len()..].to_string()).collect::<Vec<_>>())
        }
        "neoforge" => {
            let all = cached("neoforge", 3600, || maven_versions("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml")).await?;
            let mut v: Vec<String> = strs(&all).into_iter().filter(|v| neo_to_mc(v) == mc).collect();
            v.sort_by(|a, b| cmp_num(b, a));
            json!(v)
        }
        "paper" => {
            let b = http::get_json(&format!("https://fill.papermc.io/v3/projects/paper/versions/{mc}/builds")).await.unwrap_or(json!([]));
            json!(b.as_array().map(|a| a.iter().map(|x| x["id"].to_string()).collect::<Vec<_>>()).unwrap_or_default())
        }
        _ => json!([]),
    })
}

// ---------- Java ----------
fn java_dir() -> PathBuf { DATA.join("java") }
const EXE: &str = if cfg!(windows) { "java.exe" } else { "java" };
pub fn java_major_norm(m: u32) -> u32 { if m <= 8 { 8 } else if m == 16 { 17 } else { m } }
fn find_java_in(dir: &Path) -> Option<PathBuf> {
    for d in core::ls(dir) {
        let p = dir.join(&d).join("bin").join(EXE);
        if p.exists() { return Some(p); }
        let mac = dir.join(&d).join("Contents/Home/bin").join(EXE);
        if mac.exists() { return Some(mac); }
    }
    None
}
fn custom_java() -> Option<PathBuf> {
    let j = set_str("javaPath");
    (!j.is_empty() && j != "auto").then(|| PathBuf::from(j))
}
pub fn find_java(major: u32) -> Option<PathBuf> {
    custom_java().or_else(|| find_java_in(&java_dir().join(java_major_norm(major).to_string())))
}

pub async fn ensure_java(major: u32, job: Option<&Job>) -> Res<PathBuf> {
    if let Some(j) = custom_java() { return Ok(j); }
    let major = java_major_norm(major);
    let dir = java_dir().join(major.to_string());
    if let Some(f) = find_java_in(&dir) { return Ok(f); }
    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" };
    let url = format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jre/hotspot/normal/eclipse");
    let is_zip = cfg!(windows);
    let arc = java_dir().join(format!("jre{major}{}", if is_zip { ".zip" } else { ".tar.gz" }));
    if let Some(j) = job { j.log(&format!("Скачивание Java {major} (Adoptium)...")); }
    http::download(&url, &arc, |p| if let Some(j) = job { j.step(Some(&format!("Java {major}: {}%", (p * 100.0) as u32)), Some(10.0 + p * 25.0)) }).await?;
    tokio::fs::create_dir_all(&dir).await.ctx("")?;
    if let Some(j) = job { j.log(&format!("Распаковка Java {major}...")); }
    if is_zip { unzip(arc.clone(), dir.clone()).await?; }
    else {
        let st = core::command("tar").arg("xzf").arg(&arc).arg("-C").arg(&dir).status().await.ctx("tar")?;
        if !st.success() { crate::bail!("Не удалось распаковать Java"); }
    }
    let _ = tokio::fs::remove_file(&arc).await;
    find_java_in(&dir).ok_or("Java не найдена после распаковки".into())
}

pub async fn unzip(file: PathBuf, dir: PathBuf) -> Res<()> {
    tokio::task::spawn_blocking(move || -> Res<()> {
        let f = std::fs::File::open(&file).ctx("Открытие архива")?;
        let mut z = zip::ZipArchive::new(f).ctx("Чтение архива")?;
        z.extract(&dir).ctx("Распаковка")
    }).await.ctx("")?
}

// ---------- установка сервера ----------
async fn run_proc(java: &Path, args: &[String], cwd: &Path, job: &Job) -> Res<()> {
    let mut c = core::command(java);
    c.args(args).current_dir(cwd).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).stdin(std::process::Stdio::null());
    let mut child = c.spawn().ctx("Запуск установщика")?;
    let out = child.stdout.take().unwrap();
    let err = child.stderr.take().unwrap();
    let j1 = job.clone();
    let j2 = job.clone();
    let t1 = tokio::spawn(async move { let mut l = BufReader::new(out).lines(); let mut n = 0; while let Ok(Some(s)) = l.next_line().await { n += 1; if n % 20 == 0 { j1.log(&s.chars().take(160).collect::<String>()); } } });
    let t2 = tokio::spawn(async move { let mut l = BufReader::new(err).lines(); while let Ok(Some(s)) = l.next_line().await { if s.contains("ERROR") || s.contains("Exception") { j2.log(&s.chars().take(160).collect::<String>()); } } });
    let st = child.wait().await.ctx("")?;
    let _ = tokio::join!(t1, t2);
    if !st.success() { crate::bail!("Установщик завершился с кодом {}", st.code().unwrap_or(-1)); }
    Ok(())
}
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(dir).into_iter().flatten().find(|e| e.file_type().is_file() && e.file_name().to_string_lossy() == name).map(|e| e.into_path())
}

pub async fn install(srv: &mut Server, job: &Job) -> Res<()> {
    let dir = PathBuf::from(&srv.dir);
    let mc = srv.mc_version.clone();
    tokio::fs::create_dir_all(&dir).await.ctx("Создание папки сервера")?;
    job.step(Some(&format!("Получение информации о версии {mc}")), Some(5.0));
    let info = mc_version_info(&mc).await?;
    srv.java_major = info["javaVersion"]["majorVersion"].as_u64().unwrap_or(8) as u32;
    let java = ensure_java(srv.java_major, Some(job)).await?;
    job.step(Some(&format!("Java: {}", java.display())), Some(35.0));
    let dl = |url: String, file: &'static str| {
        let dest = dir.join(file);
        let j = job.clone();
        async move { http::download(&url, &dest, |p| j.step(Some(&format!("{file}: {}%", (p * 100.0) as u32)), Some(35.0 + p * 35.0))).await }
    };
    let lv = srv.loader_version.clone();
    match srv.loader.as_str() {
        "vanilla" => {
            let url = info["downloads"]["server"]["url"].as_str().ok_or("Для этой версии нет серверного jar")?.to_string();
            dl(url, "server.jar").await?;
            srv.launch = Launch { jar: Some("server.jar".into()), args: None };
        }
        "fabric" => {
            let inst = http::get_json("https://meta.fabricmc.net/v2/versions/installer").await?;
            let iv = inst[0]["version"].as_str().unwrap_or("1.0.1").to_string();
            dl(format!("https://meta.fabricmc.net/v2/versions/loader/{mc}/{lv}/{iv}/server/jar"), "fabric-server-launch.jar").await?;
            srv.launch = Launch { jar: Some("fabric-server-launch.jar".into()), args: None };
        }
        "paper" => {
            let b = http::get_json(&format!("https://fill.papermc.io/v3/projects/paper/versions/{mc}/builds/{lv}")).await?;
            let url = b["downloads"]["server:default"]["url"].as_str().ok_or("Сборка Paper не найдена")?.to_string();
            dl(url, "paper.jar").await?;
            srv.launch = Launch { jar: Some("paper.jar".into()), args: None };
        }
        l @ ("forge" | "neoforge") => {
            let url = loader_installer_url(l, &mc, &lv);
            dl(url, "installer.jar").await?;
            job.step(Some(&format!("Запуск установщика {l} (может занять несколько минут)...")), Some(72.0));
            let mut args = http::java_proxy_args();
            args.extend(["-jar".into(), "installer.jar".into(), "--installServer".into()]);
            run_proc(&java, &args, &dir, job).await?;
            let _ = tokio::fs::remove_file(dir.join("installer.jar")).await;
            let _ = tokio::fs::remove_file(dir.join("installer.jar.log")).await;
            let argf = find_file(&dir.join("libraries"), if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" });
            if let Some(a) = argf {
                srv.launch = Launch { jar: None, args: Some(a.strip_prefix(&dir).unwrap_or(&a).to_string_lossy().to_string()) };
            } else {
                let jar = core::ls(&dir).into_iter().find(|f| { let f = f.to_lowercase(); (f.starts_with("forge") || f.starts_with("neoforge")) && f.ends_with(".jar") && !f.contains("installer") })
                    .ok_or("Не найден jar сервера после установки")?;
                srv.launch = Launch { jar: Some(jar), args: None };
            }
        }
        other => crate::bail!("Неизвестное ядро: {other}"),
    }
    job.step(Some("Принятие EULA и базовая настройка"), Some(92.0));
    tokio::fs::write(dir.join("eula.txt"), "eula=true\n").await.ctx("eula.txt")?;
    let props = dir.join("server.properties");
    if !props.exists() {
        tokio::fs::write(&props, format!("server-port={}\nmotd={}\nenable-rcon=false\n", srv.port, srv.name)).await.ctx("server.properties")?;
    }
    srv.installed = true;
    Ok(())
}

pub fn loader_installer_url(loader: &str, mc: &str, lv: &str) -> String {
    if loader == "forge" {
        let full = format!("{mc}-{lv}");
        format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{full}/forge-{full}-installer.jar")
    } else {
        format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{lv}/neoforge-{lv}-installer.jar")
    }
}

pub fn launch_command(srv: &Server) -> Res<(PathBuf, Vec<String>)> {
    let java = find_java(srv.java_major).ok_or("Java не найдена — переустановите сервер или укажите путь в настройках")?;
    let mut a = http::java_proxy_args();
    a.push(format!("-Xms{}M", srv.ram.min(1024)));
    a.push(format!("-Xmx{}M", srv.ram));
    // единая кодировка консоли: кириллица в логах и командах отображается корректно
    for p in ["file", "stdout", "stderr", "stdin"] { a.push(format!("-D{p}.encoding=UTF-8")); }
    a.extend(srv.jvm_args.split_whitespace().map(String::from));
    match (&srv.launch.args, &srv.launch.jar) {
        (Some(args), _) => a.push(format!("@{args}")),
        (None, Some(jar)) => { a.push("-jar".into()); a.push(jar.clone()); }
        _ => crate::bail!("Не задан способ запуска сервера"),
    }
    a.push("nogui".into());
    Ok((java, a))
}
