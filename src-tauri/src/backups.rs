//! Резервные копии миров: zip с ротацией слотов, безопасный бэкап работающего сервера.
use crate::core::{self, emit, Ctx, Job, Res};
use crate::servers::{self, Server};
use crate::{installer, native};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn backup_dir(s: &Server) -> PathBuf {
    let base = if s.backup_dir.trim().is_empty() { core::set_str("backupsDir") } else { s.backup_dir.clone() };
    PathBuf::from(base).join(&s.name)
}
fn max_backups(s: &Server) -> usize { (if s.max_backups > 0 { s.max_backups as u64 } else { core::set_u64("maxBackups") }).max(1) as usize }

fn list_files(d: &Path) -> Vec<(String, u64, u64)> {
    let mut v: Vec<(String, u64, u64)> = std::fs::read_dir(d).map(|r| r.flatten().filter_map(|e| {
        let n = e.file_name().to_string_lossy().to_string();
        if !n.to_lowercase().ends_with(".zip") { return None; }
        let m = e.metadata().ok()?;
        let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
        Some((n, m.len(), t))
    }).collect()).unwrap_or_default();
    v.sort_by(|a, b| b.2.cmp(&a.2));
    v
}
pub fn list(id: &str) -> Res<Value> {
    let s = servers::get(id)?;
    let d = backup_dir(&s);
    let l: Vec<Value> = list_files(&d).into_iter().map(|(f, size, time)| json!({ "file": f, "size": size, "time": time })).collect();
    Ok(json!({ "dir": d, "list": l, "max": max_backups(&s) }))
}

fn world_dirs(s: &Server) -> Vec<String> {
    let props = servers::read_props(&s.id).unwrap_or(json!({}));
    let level = props["level-name"].as_str().filter(|x| !x.is_empty()).unwrap_or("world").to_string();
    std::fs::read_dir(&s.dir).map(|r| r.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| *n == level || n.starts_with(&format!("{level}_"))).collect()).unwrap_or_default()
}

pub fn backup(id: &str, note: &str) -> Res<u64> {
    let s = servers::get(id)?;
    let job = Job::new(format!("Бэкап «{}»", s.name));
    let jid = job.id;
    let note = note.to_string();
    tauri::async_runtime::spawn(async move {
        let id = s.id.clone();
        let running = servers::status(&id) == "running";
        if running {
            let _ = servers::command(&id, "save-off").await;
            let _ = servers::command(&id, "save-all flush").await;
            job.step(Some("Сохранение мира..."), Some(5.0));
            servers::wait_log(&id, r"(?i)Saved the game|Saved the world", 20_000).await;
        }
        let j = job.clone();
        let s2 = s.clone();
        let r = tauri::async_runtime::spawn_blocking(move || make_zip(&s2, &note, &j)).await.unwrap_or_else(|e| Err(e.to_string()));
        if running && servers::is_running(&id) { let _ = servers::command(&id, "save-on").await; }
        match &r {
            Ok(f) => job.done(&format!("Создан {f}")),
            Err(e) => { job.fail(e); native::notify("Ошибка бэкапа", &format!("«{}»: {e}", s.name)); }
        }
        emit(json!({ "type": "backups", "id": id }));
    });
    Ok(jid)
}

fn make_zip(s: &Server, note: &str, job: &Job) -> Res<String> {
    let dirs = world_dirs(s);
    if dirs.is_empty() { crate::bail!("Папка мира не найдена (сервер ещё не запускался?)"); }
    let bdir = backup_dir(s);
    std::fs::create_dir_all(&bdir).ctx("Создание папки бэкапов")?;
    // ротация слотов: удаляем самые ранние, пока не освободится место
    let mut existing = list_files(&bdir);
    while existing.len() >= max_backups(s) {
        let old = existing.pop().unwrap();
        let _ = std::fs::remove_file(bdir.join(&old.0));
        job.log(&format!("Перезаписан старый слот: {}", old.0));
    }
    let ts = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S").to_string();
    let name = if note.trim().is_empty() { format!("{ts}.zip") } else { format!("{ts}_{}.zip", core::safe_name(note)) };
    let path = bdir.join(&name);
    let root = Path::new(&s.dir);
    let total: u64 = dirs.iter().map(|d| core::dir_size(&root.join(d))).sum::<u64>().max(1);
    let r = (|| -> Res<()> {
        let f = std::fs::File::create(&path).ctx("Создание архива")?;
        let mut z = zip::ZipWriter::new(std::io::BufWriter::new(f));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).compression_level(Some(6)).large_file(true);
        let mut done = 0u64;
        let mut buf = vec![0u8; 256 * 1024];
        for d in &dirs {
            for e in walkdir::WalkDir::new(root.join(d)).into_iter().flatten() {
                let rel = e.path().strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                if e.file_type().is_dir() { z.add_directory(format!("{rel}/"), opts).ctx("Архив")?; continue; }
                if e.file_name() == "session.lock" { continue; }
                // файлы, занятые сервером, пропускаем, а не обрываем весь бэкап
                let Ok(mut src) = std::fs::File::open(e.path()) else { job.log(&format!("Пропущен (занят): {rel}")); continue };
                z.start_file(rel, opts).ctx("Архив")?;
                loop {
                    let n = src.read(&mut buf).ctx("Чтение файла")?;
                    if n == 0 { break; }
                    z.write_all(&buf[..n]).ctx("Запись архива")?;
                    done += n as u64;
                }
                job.progress(10.0 + done as f64 / total as f64 * 88.0);
            }
        }
        z.finish().ctx("Завершение архива")?;
        Ok(())
    })();
    if let Err(e) = r { let _ = std::fs::remove_file(&path); return Err(e); }
    Ok(name)
}

pub fn restore(id: &str, file: &str) -> Res<u64> {
    let s = servers::get(id)?;
    if servers::is_running(id) { crate::bail!("Остановите сервер перед восстановлением"); }
    let file = core::file_name(file);
    let job = Job::new(format!("Восстановление «{}» из {file}", s.name));
    let jid = job.id;
    tauri::async_runtime::spawn(async move {
        let r: Res<()> = async {
            let arc = backup_dir(&s).join(&file);
            if !arc.exists() { crate::bail!("Файл бэкапа не найден"); }
            job.step(Some("Удаление текущего мира..."), Some(10.0));
            for d in world_dirs(&s) { tokio::fs::remove_dir_all(Path::new(&s.dir).join(d)).await.ctx("Удаление мира")?; }
            job.step(Some("Распаковка..."), Some(30.0));
            installer::unzip(arc, PathBuf::from(&s.dir)).await
        }.await;
        job.finish(&r, "Мир восстановлен");
    });
    Ok(jid)
}
pub fn delete(id: &str, file: &str) -> Res<()> {
    let s = servers::get(id)?;
    std::fs::remove_file(backup_dir(&s).join(core::file_name(file))).ctx("Удаление")?;
    emit(json!({ "type": "backups", "id": id }));
    Ok(())
}
