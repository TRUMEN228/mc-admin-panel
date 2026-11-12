//! Установка сборки в свой Minecraft: папка mods и профиль загрузчика в лаунчере.
use crate::core::{self, emit, Job, DATA};
use crate::packs;
use mccore::{http, Ctx, Res};
use serde_json::json;
use std::path::PathBuf;

/// Метка папки mods, созданной сборщиком: такую можно заменять без резервной копии.
const MARK: &str = ".mccollector-pack";

pub fn install(id: &str) -> Res<u64> {
    let p = packs::load(id)?;
    let job = Job::new(format!("Установка «{}» в Minecraft", p.name));
    let jid = job.id;
    tauri::async_runtime::spawn(async move {
        let r: Res<String> = async {
            let mc = PathBuf::from(core::set_str("minecraftDir"));
            if !mc.exists() { mccore::bail!("Папка Minecraft не найдена: {}. Укажите её в настройках.", mc.display()); }
            let mods = mc.join("mods");
            if mods.exists() {
                if mods.join(MARK).exists() || mods.join(".mcpanel-set").exists() { tokio::fs::remove_dir_all(&mods).await.ctx("Очистка mods")?; }
                else {
                    let b = format!("mods_backup_{}", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
                    tokio::fs::rename(&mods, mc.join(&b)).await.ctx("Сохранение старой папки mods")?;
                    job.step(Some(&format!("Прежняя папка mods сохранена как {b}")), None);
                }
            }
            tokio::fs::create_dir_all(&mods).await.ctx("")?;
            tokio::fs::write(mods.join(MARK), &p.name).await.ctx("")?;
            let list: Vec<_> = p.mods.iter().filter(|m| m["client"] != "unsupported").collect();
            let total = list.len().max(1);
            for (i, m) in list.iter().enumerate() {
                let name = mccore::file_name(m["file"]["name"].as_str().unwrap_or(""));
                job.step(Some(&name), Some(i as f64 / total as f64 * 90.0));
                if m["source"] == "file" {
                    tokio::fs::copy(DATA.join("packs").join(&p.id).join("files").join(&name), mods.join(&name)).await.ctx(&name)?;
                } else {
                    http::download(m["file"]["url"].as_str().unwrap_or(""), &mods.join(&name), |_| {}).await.map_err(|e| format!("{name}: {e}"))?;
                }
            }
            job.step(Some("Профиль загрузчика..."), Some(95.0));
            let note = loader_profile(&p, &mc).await.unwrap_or_else(|e| format!("Загрузчик не установлен: {e}"));
            Ok(format!("Установлено модов: {}. {note}", list.len()))
        }.await;
        job.finish(&r);
        emit(json!({ "type": "installed", "id": p.id, "ok": r.is_ok(), "message": r.unwrap_or_else(|e| e) }));
    });
    Ok(jid)
}

/// Fabric — профиль в лаунчере напрямую; Forge/NeoForge — открываем официальный установщик.
async fn loader_profile(p: &packs::Pack, mc: &std::path::Path) -> Res<String> {
    let lv = packs::loader_version_or_latest(p).await?;
    match p.loader.as_str() {
        "fabric" | "quilt" => {
            let url = if p.loader == "fabric" { format!("https://meta.fabricmc.net/v2/versions/loader/{}/{lv}/profile/json", p.mc) }
                else { format!("https://meta.quiltmc.org/v3/versions/loader/{}/{lv}/profile/json", p.mc) };
            let prof = http::get_json(&url).await?;
            let vid = prof["id"].as_str().ok_or("Неверный профиль")?.to_string();
            core::write_json(&mc.join("versions").join(&vid).join(format!("{vid}.json")), &prof)?;
            let lpf = mc.join("launcher_profiles.json");
            let mut lp = core::read_json(&lpf).unwrap_or(json!({ "profiles": {} }));
            let now = chrono::Utc::now().to_rfc3339();
            lp["profiles"][format!("mccollector-{}", p.id)] = json!({ "name": p.name, "type": "custom", "lastVersionId": vid, "icon": "Crafting_Table", "created": now, "lastUsed": now });
            core::write_json(&lpf, &lp)?;
            Ok(format!("В лаунчере появился профиль «{}» — выберите его и нажмите «Играть».", p.name))
        }
        l @ ("forge" | "neoforge") => {
            let url = if l == "forge" { let f = format!("{}-{lv}", p.mc); format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{f}/forge-{f}-installer.jar") }
                else { format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{lv}/neoforge-{lv}-installer.jar") };
            let f = DATA.join("tmp").join(mccore::file_name(&url));
            http::download(&url, &f, |_| {}).await?;
            // установщику нужна Java; если её нет, открываем папку с файлом
            let started = std::process::Command::new("javaw").arg("-jar").arg(&f).current_dir(f.parent().unwrap()).spawn().is_ok();
            if started { Ok(format!("Открыт установщик {} — выберите «Install client» и нажмите OK.", mccore::loader_title(l))) }
            else {
                let _ = tauri_plugin_opener::reveal_item_in_dir(&f);
                Ok(format!("Java не найдена: запустите {} двойным щелчком (папка открыта) и выберите «Install client».", mccore::file_name(&url)))
            }
        }
        _ => Ok(String::new()),
    }
}
