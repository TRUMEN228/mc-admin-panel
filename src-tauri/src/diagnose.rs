//! Разбор сбоев: последний crash-report и подсказки по типичным причинам.
use crate::core::Res;
use crate::servers;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::time::UNIX_EPOCH;

pub struct Hint { pub title: String, pub text: String }

/// Ищет типичные причины по строкам лога и тексту отчёта.
pub fn hints(lines: &[String], report: &str, ram_mb: u32) -> Vec<Hint> {
    let all = format!("{}\n{report}", lines.join("\n"));
    let mut v = vec![];
    let mut add = |t: &str, x: String| { if !v.iter().any(|h: &Hint| h.title == t) { v.push(Hint { title: t.into(), text: x }); } };
    if all.contains("OutOfMemoryError") {
        add("Не хватает памяти", format!("Серверу не хватило выделенной памяти ({ram_mb} МБ). Увеличьте её в «Параметрах» или уберите тяжёлые моды."));
    }
    if all.contains("FAILED TO BIND TO PORT") || all.contains("Address already in use") || all.contains("BindException") {
        add("Порт занят", "Порт уже используется другой программой или другим сервером. Смените порт в «Параметрах».".into());
    }
    if all.contains("UnsupportedClassVersionError") || all.contains("Unsupported class file major version") || all.contains("requires Java") {
        add("Неподходящая версия Java", "Мод или ядро собраны под другую версию Java. Переустановите ядро — панель подберёт нужную Java — или проверьте путь к Java в настройках.".into());
    }
    if all.contains("Unrecognized VM option") || all.contains("Could not create the Java Virtual Machine") || all.contains("Invalid maximum heap size") || all.contains("Unrecognized option") {
        let opt = Regex::new(r"Unrecognized (?:VM )?option '?([^'\s]+)").unwrap().captures(&all).map(|c| format!(" Java не понимает параметр «{}».", &c[1])).unwrap_or_default();
        add("Неверные аргументы JVM", format!("Java не запустилась из-за параметров запуска.{opt} Проверьте «Дополнительные аргументы JVM» и набор флагов в «Параметрах»."));
    }
    if all.contains("You need to agree to the EULA") { add("Не принята EULA", "Откройте eula.txt во вкладке «Файлы» и укажите eula=true.".into()); }
    if all.contains("Could not find or load main class") || all.contains("Unable to access jarfile") || all.contains("Error: Invalid or corrupt jarfile") {
        add("Повреждена установка", "Файл ядра не найден или повреждён. Переустановите ядро в «Параметрах».".into());
    }
    if all.contains("Session lock") || all.contains("session.lock") && all.contains("already") {
        add("Мир занят другим процессом", "Мир уже открыт другим запущенным сервером. Закройте его и попробуйте снова.".into());
    }
    // зависимости модов: Fabric, Forge/NeoForge
    let dep = Regex::new(r"(?i)(?:Mod '([^']+)' \(([\w\-]+)\)[^\n]*requires[^\n]*?(?:mod '?([^'\n]+)'?|version)[^\n]*|Mod ([\w\-]+) requires ([\w\-]+)[^\n]*|Missing or unsupported mandatory dependencies:[^\n]*\n[^\n]*?Mod ID: '([\w\-]+)'[^\n]*Requested by: '([\w\-]+)')").unwrap();
    let mut found = vec![];
    for c in dep.captures_iter(&all).take(6) { found.push(c.get(0).map(|m| m.as_str().trim().chars().take(220).collect::<String>()).unwrap_or_default()); }
    if !found.is_empty() || all.contains("ModResolutionException") || all.contains("Missing or unsupported mandatory dependencies") {
        add("Не хватает зависимостей мода", if found.is_empty() { "Какому-то моду нужна библиотека или другая версия мода. Подробности — в отчёте ниже.".into() } else { found.join("\n") });
    }
    let mixin = Regex::new(r"(?i)Mixin apply(?: for mod ([\w\-]+))? failed|mixin\.[\w.]*json:[\w$.]+ from mod ([\w\-]+)").unwrap();
    if let Some(c) = mixin.captures(&all) {
        let m = c.get(1).or(c.get(2)).map(|m| m.as_str().to_string()).unwrap_or_default();
        add("Конфликт модов", if m.is_empty() { "Один из модов несовместим с другими или с этой версией Minecraft.".into() } else { format!("Мод «{m}» несовместим с другими модами или с этой версией Minecraft. Попробуйте обновить или отключить его.") });
    }
    let suspect = Regex::new(r"(?m)^\s*Suspected Mods?: ([^\n]+)").unwrap();
    if let Some(c) = suspect.captures(&all) {
        if !c[1].contains("NONE") { add("Подозреваемые моды", format!("По отчёту о сбое: {}", c[1].trim())); }
    }
    if v.is_empty() {
        let desc = Regex::new(r"(?m)^Description: (.+)$").unwrap().captures(report).map(|c| c[1].to_string());
        if let Some(d) = desc { v.push(Hint { title: "Причина из отчёта".into(), text: d }); }
    }
    v
}

pub fn crash_reports(dir: &Path) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = std::fs::read_dir(dir.join("crash-reports")).map(|r| r.flatten().filter_map(|e| {
        let t = e.metadata().ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
        Some((e.file_name().to_string_lossy().to_string(), t))
    }).collect()).unwrap_or_default();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

/// Диагностика сервера: отчёт о сбое после `since` (мс) и подсказки.
pub fn diagnose(id: &str, since: u64) -> Res<Value> {
    let s = servers::get(id)?;
    let lines: Vec<String> = servers::log(id).as_array().cloned().unwrap_or_default().into_iter().filter_map(|v| v.as_str().map(String::from)).rev().take(400).collect::<Vec<_>>().into_iter().rev().collect();
    let reports = crash_reports(Path::new(&s.dir));
    let fresh = reports.iter().find(|r| r.1 + 5000 >= since);
    let report_text = fresh.and_then(|r| std::fs::read_to_string(Path::new(&s.dir).join("crash-reports").join(&r.0)).ok()).unwrap_or_default();
    let h = hints(&lines, &report_text, s.ram);
    Ok(json!({
        "hints": h.iter().map(|h| json!({ "title": h.title, "text": h.text })).collect::<Vec<_>>(),
        "report": fresh.map(|r| json!({ "file": format!("crash-reports/{}", r.0), "time": r.1 })),
        "reports": reports.iter().take(10).map(|r| json!({ "file": format!("crash-reports/{}", r.0), "time": r.1 })).collect::<Vec<_>>(),
    }))
}
