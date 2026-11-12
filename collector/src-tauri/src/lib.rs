//! MC Mod Collector — подбор модов в сборку и обмен сборками (.mrpack) с MC Admin Panel.
pub mod client;
#[cfg(debug_assertions)]
pub mod dev;
pub mod core;
pub mod packs;

use mccore::{modmeta, providers, versions, Res};
use serde_json::{json, Value};
use tauri::Manager;

pub async fn dispatch(cmd: &str, a: Value) -> Res<Value> {
    let s = |k: &str| core::arg(&a, k);
    let id = s("id");
    let ok = || Ok(json!({ "ok": true }));
    match cmd {
        "settings.get" => Ok(core::settings()),
        "settings.save" => core::save_settings(a.clone()),
        "system.info" => Ok(json!({ "version": env!("CARGO_PKG_VERSION"), "portable": *core::PORTABLE, "dataDir": core::DATA.to_string_lossy(), "debug": cfg!(debug_assertions), "mica": core::mica_supported() })),
        "jobs" => Ok(json!(core::jobs())),
        "open" => { tauri_plugin_opener::open_path(s("path"), None::<&str>).map_err(|e| e.to_string())?; ok() }
        "versions" => versions::mc_versions(a["snapshots"].as_bool().unwrap_or(false)).await,
        "loaderVersions" => versions::loader_versions(&s("loader"), &s("mc")).await,
        "catalog.providers" => Ok(providers::list(&s("loader")).as_array().cloned().map(|v| json!(v.into_iter().filter(|p| p["id"] == "modrinth" || p["id"] == "curseforge").collect::<Vec<_>>())).unwrap_or(json!([]))),
        "catalog.search" => providers::search(&match s("provider") { p if p.is_empty() => "modrinth".into(), p => p }, &s("q"), &s("mc"), &s("loader"), "mod", a["offset"].as_u64().unwrap_or(0)).await,
        "catalog.details" => providers::details(&s("provider"), &s("projectId")).await,
        "jar.meta" => Ok(modmeta::read_jar(std::path::Path::new(&s("path")))),

        "packs.list" => Ok(packs::list()),
        "packs.get" => packs::get(&id),
        "packs.create" => packs::create(&a),
        "packs.update" => packs::update(&id, &a["patch"]),
        "packs.delete" => { packs::delete(&id)?; ok() }
        "packs.duplicate" => packs::duplicate(&id),
        "packs.add" => packs::add(&id, &s("provider"), &s("project")).await,
        "packs.addFiles" => packs::add_files(&id, &a["paths"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>()).await,
        "packs.addFolder" => packs::add_files(&id, &packs::folder_jars(&s("path"))).await,
        "packs.addUrl" => packs::add_url(&id, &s("url")).await,
        "packs.remove" => { packs::remove(&id, &s("key"))?; ok() }
        "packs.setSide" => { packs::set_side(&id, &s("key"), &s("client"), &s("server"))?; ok() }
        "packs.check" => packs::check(&id, None, None).await,
        "packs.applyUpdates" => Ok(json!({ "updated": packs::apply_updates(&id, &a["entries"])? })),
        "packs.changeVersion" => packs::change_version(&id, &s("mc"), &s("loader"), &s("loaderVersion")).await,
        "packs.export" => Ok(json!({ "job": packs::export(&id, &s("path"))? })),
        "packs.importMrpack" => packs::import_mrpack(&s("path")).await,
        "packs.detectFolder" => packs::detect_folder(&s("path")).await,
        "packs.installClient" => Ok(json!({ "job": client::install(&id)? })),
        _ => Err(format!("Неизвестная команда: {cmd}")),
    }
}

#[tauri::command]
async fn call(cmd: String, args: Option<Value>) -> Result<Value, String> { dispatch(&cmd, args.unwrap_or(json!({}))).await }

pub fn run() {
    once_cell::sync::Lazy::force(&core::SETTINGS);
    core::sync_mccore();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") { let _ = w.unminimize(); let _ = w.show(); let _ = w.set_focus(); }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![call])
        .setup(|app| {
            let _ = core::APP.set(app.handle().clone());
            #[cfg(debug_assertions)]
            dev::start();
            if let Some(w) = app.get_webview_window("main") {
                // Mica в Windows 11: фон окна рисует система
                use tauri::window::{Effect, EffectsBuilder};
                if core::mica_supported() { let _ = w.set_effects(EffectsBuilder::new().effect(Effect::Mica).build()); }
                let _ = w.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ошибка запуска MC Mod Collector");
}
