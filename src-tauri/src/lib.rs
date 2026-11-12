//! MC Admin Panel — нативное приложение (Tauri 2 + Rust) для управления серверами Minecraft.
pub mod api;
pub mod backups;
pub mod core;
pub mod http;
pub mod importer;
pub mod installer;
pub mod mods;
pub mod native;
pub mod servers;
pub mod web;

use std::sync::atomic::Ordering;
use tauri::{Manager, RunEvent, WindowEvent};

#[tauri::command]
fn web_status() -> serde_json::Value { web::STATE.lock().clone() }

pub fn run() {
    if std::env::args().any(|a| a == "--hidden") { native::START_HIDDEN.store(true, Ordering::Relaxed); }
    // загружаем настройки и список серверов до старта окна
    once_cell::sync::Lazy::force(&core::SETTINGS);
    once_cell::sync::Lazy::force(&servers::LIST);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| native::show_window(None)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .invoke_handler(tauri::generate_handler![api::call, native::ui_ready, web_status])
        .setup(|app| {
            let _ = core::APP.set(app.handle().clone());
            if let Some(w) = app.get_webview_window("main") { native::setup_window(&w); }
            native::init_listeners();
            native::setup_tray(app.handle())?;
            native::apply_login_item();
            servers::init();
            core::SETTINGS_HOOKS.lock().push(Box::new(web::apply));
            web::apply();
            // страховка: если UI не сообщил о готовности, всё равно показываем окно
            tauri::async_runtime::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                native::ui_ready();
            });
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e { native::on_close_requested(w, api); }
        })
        .build(tauri::generate_context!())
        .expect("ошибка запуска MC Admin Panel");

    app.run(|app, e| {
        if let RunEvent::ExitRequested { api, .. } = e {
            // живём в трее, пока пользователь явно не выберет «Выход»
            if !native::QUITTING.load(Ordering::Relaxed) { api.prevent_exit(); native::quit(app); }
        }
    });
}
