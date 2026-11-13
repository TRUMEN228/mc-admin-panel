//! Инструменты сервера: клонирование, карта мира (BlueMap), прогрузка чанков (Chunky), туннель playit.gg.
use crate::core::{self, emit, now_ms, Ctx, Job, Res, DATA};
use crate::servers::{self, Server};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

// ---------- клонирование ----------
/// Копия сервера: те же ядро, моды, конфиги и настройки; мир — по желанию (без мира получится «шаблон»).
pub fn clone(id: &str, name: &str, with_world: bool) -> Res<Value> {
    let src = servers::get(id)?;
    let name = core::safe_name(if name.trim().is_empty() { &src.name } else { name });
    let dir = PathBuf::from(core::set_str("serversDir")).join(&name);
    if dir.exists() { crate::bail!("Папка «{}» уже существует — выберите другое название", dir.display()); }
    let mut port = 25565u16;
    while servers::all_raw().iter().any(|s| s.port == port) { port += 1; }
    let job = Job::new(format!("Клонирование «{}» → «{name}»", src.name));
    let jid = job.id;
    let name2 = name.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let r = copy_server(&src, &dir, with_world, &job).and_then(|_| {
            let mut s: Server = src.clone();
            s.id = core::new_id(); s.name = name2.clone(); s.dir = dir.to_string_lossy().to_string(); s.port = port;
            s.created = now_ms(); s.auto_start = false; s.backup_dir = String::new();
            servers::add(s)
        });
        match r { Ok(nid) => { let _ = servers::write_props(&nid, &json!({ "server-port": port.to_string() })); job.done(&format!("Готово: «{name2}», порт {port}")); } Err(e) => job.fail(&e) }
        emit(json!({ "type": "servers" }));
    });
    Ok(json!({ "job": jid }))
}
fn copy_server(s: &Server, dst: &Path, with_world: bool, job: &Job) -> Res<()> {
    let root = Path::new(&s.dir);
    let worlds: Vec<PathBuf> = std::fs::read_dir(root).ctx("Чтение папки сервера")?.flatten()
        .filter(|e| e.path().join("level.dat").exists()).map(|e| e.path()).collect();
    let skip = |p: &Path| -> bool {
        let rel = p.strip_prefix(root).unwrap_or(p);
        let first = rel.components().next().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).unwrap_or_default();
        matches!(first.as_str(), "logs" | "crash-reports" | "cache" | "debug")
            || p.file_name().is_some_and(|n| n == "session.lock")
            || (!with_world && worlds.iter().any(|w| p.starts_with(w)))
    };
    let total = core::dir_size(root).max(1);
    let mut done = 0u64;
    for e in walkdir::WalkDir::new(root).into_iter().filter_entry(|e| !skip(e.path())).flatten() {
        let rel = e.path().strip_prefix(root).unwrap();
        let to = dst.join(rel);
        if e.file_type().is_dir() { std::fs::create_dir_all(&to).ctx("Создание папки")?; continue; }
        if std::fs::copy(e.path(), &to).is_err() { job.log(&format!("Пропущен (занят): {}", rel.display())); continue; }
        done += e.metadata().map(|m| m.len()).unwrap_or(0);
        job.progress(done as f64 / total as f64 * 100.0);
    }
    Ok(())
}

// ---------- карта мира и прогрузка ----------
fn find_jar(s: &Server, re: &str) -> bool {
    let re = regex::Regex::new(re).unwrap();
    ["mods", "plugins"].iter().any(|d| std::fs::read_dir(Path::new(&s.dir).join(d)).map(|r| r.flatten().any(|e| {
        let n = e.file_name().to_string_lossy().to_lowercase();
        n.ends_with(".jar") && re.is_match(&n)
    })).unwrap_or(false))
}
fn bluemap_conf(s: &Server, file: &str) -> Option<PathBuf> {
    [Path::new(&s.dir).join("plugins/BlueMap").join(file), Path::new(&s.dir).join("config/bluemap").join(file)].into_iter().find(|p| p.exists())
}
/// Состояние инструментов сервера: установлены ли BlueMap и Chunky, адрес карты.
pub fn status(id: &str) -> Res<Value> {
    let s = servers::get(id)?;
    let map = find_jar(&s, r"^bluemap");
    let accepted = bluemap_conf(&s, "core.conf").and_then(|p| std::fs::read_to_string(p).ok()).map(|t| t.contains("accept-download: true"));
    let port = bluemap_conf(&s, "webserver.conf").and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| regex::Regex::new(r"(?m)^\s*port:\s*(\d+)").unwrap().captures(&t).and_then(|c| c[1].parse::<u16>().ok())).unwrap_or(8100);
    Ok(json!({ "bluemap": map, "accepted": accepted, "mapUrl": format!("http://localhost:{port}"), "mapPort": port, "chunky": find_jar(&s, r"^chunky") }))
}
/// BlueMap просит согласие на загрузку ресурсов Minecraft: ставим accept-download и перезагружаем карту.
pub async fn map_enable(id: &str) -> Res<Value> {
    let s = servers::get(id)?;
    let Some(p) = bluemap_conf(&s, "core.conf") else { crate::bail!("Конфиг BlueMap появится после первого запуска сервера с картой — запустите сервер и нажмите ещё раз") };
    let t = std::fs::read_to_string(&p).ctx("Чтение core.conf")?;
    std::fs::write(&p, t.replace("accept-download: false", "accept-download: true")).ctx("Запись core.conf")?;
    if servers::is_running(id) { let _ = servers::command(id, "bluemap reload").await; }
    status(id)
}

// ---------- playit.gg ----------
fn playit_path() -> PathBuf { DATA.join("tools").join("playit.exe") }
/// Скачивает агент playit.gg (если нужно) и запускает его в отдельном окне: там ссылка привязки аккаунта.
pub async fn playit_start() -> Res<Value> {
    let p = playit_path();
    if !p.exists() {
        let r: Value = mccore::http::client().get("https://api.github.com/repos/playit-cloud/playit-agent/releases/latest")
            .header("Accept", "application/vnd.github+json").send().await.map_err(|e| format!("Сеть: {e}"))?.json().await.ctx("GitHub")?;
        let assets = r["assets"].as_array().cloned().unwrap_or_default();
        let pick = |want_signed: bool| assets.iter().find(|a| { let n = a["name"].as_str().unwrap_or("").to_lowercase(); n.contains("windows") && n.contains("x86_64") && n.ends_with(".exe") && (!want_signed || n.contains("signed")) }).cloned();
        let a = pick(true).or_else(|| pick(false)).ok_or("Не найден агент playit.gg для Windows")?;
        let bytes = mccore::http::client().get(a["browser_download_url"].as_str().unwrap_or("")).send().await.map_err(|e| format!("Сеть: {e}"))?.bytes().await.map_err(|e| format!("Загрузка: {e}"))?;
        std::fs::create_dir_all(p.parent().unwrap()).ctx("Папка инструментов")?;
        std::fs::write(&p, &bytes).ctx("Сохранение агента")?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&p).current_dir(p.parent().unwrap()).creation_flags(0x0000_0010 /* CREATE_NEW_CONSOLE */).spawn().ctx("Запуск playit.gg")?;
    }
    Ok(json!({ "path": p }))
}

// ---------- перенос данных игрока на другой ник ----------
/// UUID игрока в офлайн-режиме (online-mode=false): UUID v3 от «OfflinePlayer:<ник>», как считает сам сервер.
pub fn offline_uuid(name: &str) -> String {
    use md5::{Digest, Md5};
    let mut h: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    let x = hex::encode(h);
    format!("{}-{}-{}-{}-{}", &x[0..8], &x[8..12], &x[12..16], &x[16..20], &x[20..32])
}
/// UUID ника: из usercache.json сервера (кто уже заходил), иначе по режиму сервера — офлайн-расчёт или Mojang API.
pub async fn uuid_of(s: &Server, name: &str, online: bool) -> Res<String> {
    let cache: Value = std::fs::read_to_string(Path::new(&s.dir).join("usercache.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!([]));
    if let Some(u) = cache.as_array().and_then(|a| a.iter().find(|e| e["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(name)))).and_then(|e| e["uuid"].as_str()) {
        return Ok(u.to_lowercase());
    }
    if !online { return Ok(offline_uuid(name)); }
    let r: Value = mccore::http::client().get(format!("https://api.mojang.com/users/profiles/minecraft/{}", urlencoding::encode(name)))
        .send().await.map_err(|e| format!("Mojang: {e}"))?.json().await.map_err(|_| format!("Ник «{name}» не найден в Mojang"))?;
    let id = r["id"].as_str().ok_or(format!("Ник «{name}» не найден в Mojang"))?;
    Ok(format!("{}-{}-{}-{}-{}", &id[0..8], &id[8..12], &id[12..16], &id[16..20], &id[20..32]).to_lowercase())
}
/// Переносит инвентарь, позицию, достижения, статистику и данные модов со старого ника на новый.
/// Данные нового ника сохраняются в папку player-transfer-backup-<время> в папке мира.
pub async fn transfer_player(id: &str, from: &str, to: &str) -> Res<Value> {
    let s = servers::get(id)?;
    if servers::is_running(id) { crate::bail!("Остановите сервер: пока он работает, он перезапишет данные игроков"); }
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() || to.is_empty() || from.eq_ignore_ascii_case(to) { crate::bail!("Укажите два разных ника"); }
    let props = servers::read_props(id)?;
    let online = props["online-mode"].as_str() != Some("false");
    let (old, new) = (uuid_of(&s, from, online).await?, uuid_of(&s, to, online).await?);
    let root = Path::new(&s.dir);
    let worlds: Vec<PathBuf> = std::fs::read_dir(root).ctx("Папка сервера")?.flatten().map(|e| e.path()).filter(|p| p.join("level.dat").exists()).collect();
    if worlds.is_empty() { crate::bail!("Мир не найден — сервер ещё не запускался"); }
    let ts = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
    let mut moved = vec![];
    for w in &worlds {
        // все файлы мира, в имени которых есть старый UUID (playerdata, advancements, stats, данные модов)
        let files: Vec<PathBuf> = walkdir::WalkDir::new(w).into_iter().flatten().filter(|e| e.file_type().is_file())
            .filter(|e| e.file_name().to_string_lossy().to_lowercase().contains(&old)).map(|e| e.into_path()).collect();
        for f in files {
            let name = f.file_name().unwrap().to_string_lossy().to_lowercase().replace(&old, &new);
            let dst = f.with_file_name(&name);
            if dst.exists() {
                let bak = w.join(format!("player-transfer-backup-{ts}")).join(dst.strip_prefix(w).unwrap());
                std::fs::create_dir_all(bak.parent().unwrap()).ctx("Резервная копия")?;
                std::fs::copy(&dst, &bak).ctx("Резервная копия")?;
            }
            std::fs::copy(&f, &dst).ctx("Копирование данных игрока")?;
            moved.push(dst.strip_prefix(root).unwrap_or(&dst).to_string_lossy().replace('\\', "/"));
        }
    }
    if moved.is_empty() { crate::bail!("У ника «{from}» нет сохранённых данных (UUID {old}). Проверьте ник — он должен совпадать с тем, под которым играли"); }
    // операторы и белый список: та же запись под новым ником
    for list in ["ops.json", "whitelist.json"] {
        let p = root.join(list);
        let Some(mut v) = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { continue };
        let Some(a) = v.as_array_mut() else { continue };
        if let Some(e) = a.iter().find(|e| e["uuid"].as_str().is_some_and(|u| u.eq_ignore_ascii_case(&old))).cloned() {
            if !a.iter().any(|x| x["uuid"].as_str().is_some_and(|u| u.eq_ignore_ascii_case(&new))) {
                let mut n = e; n["uuid"] = json!(new); n["name"] = json!(to); a.push(n);
                let _ = std::fs::write(&p, serde_json::to_string_pretty(&v).unwrap_or_default());
            }
        }
    }
    // питомец на плече и животное, на котором сидит игрок, хранятся в его файле — там тоже меняем хозяина
    let (ob, nb) = (crate::pets::uuid_bytes(&old).ok_or("Неверный UUID")?, crate::pets::uuid_bytes(&new).ok_or("Неверный UUID")?);
    for w in &worlds {
        let p = w.join("playerdata").join(format!("{new}.dat"));
        if let Ok(mut v) = crate::playerdata::load(&p) {
            if crate::pets::replace_uuid(&mut v, &ob, &nb, &old, &new) > 0 { crate::playerdata::store(&p, &v)?; }
        }
    }
    // питомцы в мире: UUID хозяина в файлах сущностей (может занять несколько минут на большом мире)
    let regions: Vec<(PathBuf, PathBuf)> = worlds.iter().flat_map(|w| {
        let bak = w.join(format!("player-transfer-backup-{ts}"));
        crate::pets::entity_regions(w).into_iter().map(move |r| { let b = bak.join(r.strip_prefix(w).unwrap_or(&r)); (r, b) })
    }).collect();
    let job = Job::new(format!("Перенос питомцев {from} → {to}"));
    let (o2, n2, total) = (old.clone(), new.clone(), regions.len().max(1));
    let pets = tauri::async_runtime::spawn_blocking(move || {
        let (mut changed, mut files) = (0usize, 0usize);
        for (i, (r, b)) in regions.iter().enumerate() {
            match crate::pets::process_region(r, &o2, &n2, b) {
                Ok(n) if n > 0 => { changed += n; files += 1; }
                Ok(_) => {}
                Err(e) => job.log(&format!("{}: {e}", r.display())),
            }
            job.progress((i + 1) as f64 / total as f64 * 100.0);
        }
        job.done(&format!("Питомцев и привязок перенесено: {changed}"));
        (changed, files)
    }).await.map_err(|e| e.to_string())?;
    Ok(json!({ "from": old, "to": new, "online": online, "files": moved, "pets": pets.0, "petFiles": pets.1 }))
}
