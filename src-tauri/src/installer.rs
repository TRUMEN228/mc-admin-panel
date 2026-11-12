//! Версии Minecraft/загрузчиков, Java (Adoptium) и установка ядра сервера.
use crate::core::{self, set_str, Ctx, Job, Res, DATA};
use crate::http;
use crate::servers::{Launch, Server};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};

pub use mccore::versions::{loader_versions, mc_version_info, mc_versions, neo_to_mc};

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
    // Aikar рекомендует Xms = Xmx: куча выделяется сразу и не дёргается
    a.push(format!("-Xms{}M", if srv.jvm_preset.is_empty() { srv.ram.min(1024) } else { srv.ram }));
    a.push(format!("-Xmx{}M", srv.ram));
    a.extend(jvm_preset(&srv.jvm_preset, srv.ram, srv.java_major));
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

pub use mccore::loader_title;

/// Готовые наборы флагов JVM.
pub fn jvm_preset(name: &str, ram: u32, java: u32) -> Vec<String> {
    let v: Vec<&str> = match name {
        // https://docs.papermc.io/paper/aikars-flags
        "aikar" => {
            let mut f = vec!["-XX:+UseG1GC", "-XX:+ParallelRefProcEnabled", "-XX:MaxGCPauseMillis=200", "-XX:+UnlockExperimentalVMOptions",
                "-XX:+DisableExplicitGC", "-XX:+AlwaysPreTouch", "-XX:G1HeapWastePercent=5", "-XX:G1MixedGCCountTarget=4",
                "-XX:G1MixedGCLiveThresholdPercent=90", "-XX:G1RSetUpdatingPauseTimePercent=5", "-XX:SurvivorRatio=32",
                "-XX:+PerfDisableSharedMem", "-XX:MaxTenuringThreshold=1", "-Dusing.aikars.flags=https://mcflags.emc.gs", "-Daikars.new.flags=true"];
            if ram >= 12288 { f.extend(["-XX:G1NewSizePercent=40", "-XX:G1MaxNewSizePercent=50", "-XX:G1HeapRegionSize=16M", "-XX:G1ReservePercent=15", "-XX:InitiatingHeapOccupancyPercent=20"]); }
            else { f.extend(["-XX:G1NewSizePercent=30", "-XX:G1MaxNewSizePercent=40", "-XX:G1HeapRegionSize=8M", "-XX:G1ReservePercent=20", "-XX:InitiatingHeapOccupancyPercent=15"]); }
            f
        }
        // ZGC: минимальные паузы, нужен Java 21+ (на 21 поколенческий режим включается флагом)
        "zgc" if java >= 21 => {
            let mut f = vec!["-XX:+UseZGC", "-XX:+AlwaysPreTouch", "-XX:+DisableExplicitGC", "-XX:+PerfDisableSharedMem"];
            if java == 21 { f.push("-XX:+ZGenerational"); }
            f
        }
        _ => vec![],
    };
    v.into_iter().map(String::from).collect()
}
