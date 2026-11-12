//! Нативная обвязка Windows: окно (Mica), трей, уведомления, прогресс на панели задач, выход.
use crate::core::{self, APP};
use crate::servers;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindow};

pub static QUITTING: AtomicBool = AtomicBool::new(false);
pub static START_HIDDEN: AtomicBool = AtomicBool::new(false);
static SHOWN_ONCE: AtomicBool = AtomicBool::new(false);
static TRAY_HINT: AtomicBool = AtomicBool::new(false);

fn win() -> Option<WebviewWindow> { APP.get()?.get_webview_window("main") }

// ---------- Mica ----------
pub fn mica_supported() -> bool {
    static V: Lazy<bool> = Lazy::new(|| {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let o = std::process::Command::new("reg")
                .args(["query", r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion", "/v", "CurrentBuildNumber"])
                .creation_flags(0x0800_0000).output();
            if let Ok(o) = o {
                let s = String::from_utf8_lossy(&o.stdout);
                if let Some(n) = s.split_whitespace().last().and_then(|x| x.parse::<u32>().ok()) { return n >= 22000; }
            }
            false
        }
        #[cfg(not(windows))]
        false
    });
    *V
}
pub fn setup_window(w: &WebviewWindow) {
    if mica_supported() {
        use tauri::window::{Effect, EffectsBuilder};
        let _ = w.set_effects(EffectsBuilder::new().effect(Effect::Mica).build());
    }
}
pub fn show_window(hash: Option<&str>) {
    let Some(w) = win() else { return };
    if let Some(h) = hash { let _ = w.eval(&format!("location.hash={}", Value::from(h))); }
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
}
/// UI загрузился: показываем окно (без белой вспышки), если не стартовали свёрнутыми в трей.
#[tauri::command]
pub fn ui_ready() {
    if !START_HIDDEN.load(Ordering::Relaxed) && !SHOWN_ONCE.swap(true, Ordering::Relaxed) { show_window(None); }
}

// ---------- уведомления ----------
pub fn notify(title: &str, body: &str) {
    if !core::set_bool("notifications") { return; }
    let Some(app) = APP.get() else { return };
    // если окно на переднем плане — UI сам покажет тост
    if win().map(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false)).unwrap_or(false) { return; }
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}
pub fn notify_job(title: &str, r: Result<String, &String>) {
    match r { Ok(m) => notify(title, &m), Err(e) => notify(&format!("Ошибка: {title}"), e) }
}

// ---------- прогресс фоновых задач на панели задач ----------
fn update_taskbar() {
    use tauri::window::{ProgressBarState, ProgressBarStatus};
    let Some(w) = win() else { return };
    let jobs = core::jobs();
    let running: Vec<_> = jobs.iter().filter(|j| j.status == "running").collect();
    let st = if running.is_empty() {
        ProgressBarState { status: Some(ProgressBarStatus::None), progress: None }
    } else {
        let p = running.iter().map(|j| j.progress as u64).sum::<u64>() / running.len() as u64;
        ProgressBarState { status: Some(if p == 0 { ProgressBarStatus::Indeterminate } else { ProgressBarStatus::Normal }), progress: Some(p) }
    };
    let _ = w.set_progress_bar(st);
}

// ---------- трей ----------
const STATUS: [(&str, &str, &str); 4] = [("running", "работает", "●"), ("starting", "запуск…", "◐"), ("stopping", "остановка…", "◐"), ("stopped", "остановлен", "○")];
fn st_label(s: &str) -> (&'static str, &'static str) { STATUS.iter().find(|x| x.0 == s).map(|x| (x.1, x.2)).unwrap_or(("остановлен", "○")) }

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let list = servers::all_raw();
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, "show", "Открыть панель", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    if list.is_empty() { menu.append(&MenuItem::with_id(app, "none", "Серверов нет", false, None::<&str>)?)?; }
    for s in &list {
        let st = servers::status(&s.id);
        let players = servers::RT.lock().get(&s.id).map(|r| r.players.len()).unwrap_or(0);
        let (label, mark) = st_label(st);
        let title = format!("{mark}  {} — {label}{}", s.name, if players > 0 { format!(" ({players} игр.)") } else { String::new() });
        let id = |a: &str| format!("s|{}|{a}", s.id);
        let sub = Submenu::with_items(app, title, true, &[
            &MenuItem::with_id(app, id("start"), "Запустить", st == "stopped" && s.installed, None::<&str>)?,
            &MenuItem::with_id(app, id("restart"), "Перезапустить", st == "running", None::<&str>)?,
            &MenuItem::with_id(app, id("stop"), "Остановить", st == "running" || st == "starting", None::<&str>)?,
            &MenuItem::with_id(app, id("backup"), "Сделать бэкап", s.installed, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, id("console"), "Консоль", true, None::<&str>)?,
            &MenuItem::with_id(app, id("folder"), "Открыть папку", true, None::<&str>)?,
        ])?;
        menu.append(&sub)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&CheckMenuItem::with_id(app, "autostart", "Запускать вместе с Windows", !cfg!(debug_assertions), core::set_bool("launchAtLogin"), None::<&str>)?)?;
    menu.append(&CheckMenuItem::with_id(app, "totray", "Сворачивать в трей при закрытии", true, core::set_bool("minimizeToTray"), None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?)?;
    Ok(menu)
}
fn tray_tooltip() -> String {
    let list = servers::all_raw();
    let on = list.iter().filter(|s| servers::status(&s.id) != "stopped").count();
    format!("MC Admin Panel — запущено серверов: {on} из {}", list.len())
}
pub fn refresh_tray() {
    static PENDING: AtomicBool = AtomicBool::new(false);
    if PENDING.swap(true, Ordering::Relaxed) { return; }
    tauri::async_runtime::spawn(async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        PENDING.store(false, Ordering::Relaxed);
        let Some(app) = APP.get().cloned() else { return };
        let a2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(t) = a2.tray_by_id("main") {
                if let Ok(m) = build_menu(&a2) { let _ = t.set_menu(Some(m)); }
                let _ = t.set_tooltip(Some(tray_tooltip()));
            }
        });
    });
}
fn run_bg(f: impl std::future::Future<Output = Result<(), String>> + Send + 'static) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = f.await {
            if let Some(app) = APP.get() {
                use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                app.dialog().message(e).title("Ошибка").kind(MessageDialogKind::Error).show(|_| {});
            }
        }
    });
}
fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "show" => show_window(None),
        "quit" => quit(app),
        "autostart" => { let v = !core::set_bool("launchAtLogin"); let _ = core::save_settings(serde_json::json!({ "launchAtLogin": v })); }
        "totray" => { let v = !core::set_bool("minimizeToTray"); let _ = core::save_settings(serde_json::json!({ "minimizeToTray": v })); }
        _ => {
            let p: Vec<&str> = id.split('|').collect();
            if p.len() != 3 { return; }
            let (sid, act) = (p[1].to_string(), p[2]);
            match act {
                "start" => run_bg(servers::start(sid)),
                "stop" => run_bg(async move { servers::stop(&sid).await }),
                "restart" => run_bg(async move { servers::restart(&sid).await }),
                "backup" => run_bg(async move { crate::backups::backup(&sid, "tray").map(|_| ()) }),
                "console" => show_window(Some(&format!("server/{sid}/console"))),
                "folder" => { if let Ok(s) = servers::get(&sid) { let _ = core::open_path(&s.dir); } }
                _ => {}
            }
        }
    }
}
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("icon"))
        .tooltip(tray_tooltip())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| on_menu(app, e.id().as_ref()))
        .on_tray_icon_event(|t, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e { let _ = t; show_window(None); }
        })
        .build(app)?;
    Ok(())
}

// ---------- автозапуск с Windows ----------
pub fn apply_login_item() {
    if cfg!(debug_assertions) { return; } // в режиме разработки не регистрируем отладочный exe
    let Some(app) = APP.get() else { return };
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let want = core::set_bool("launchAtLogin");
    if al.is_enabled().unwrap_or(false) != want { let _ = if want { al.enable() } else { al.disable() }; }
}

// ---------- закрытие окна и выход ----------
pub fn on_close_requested(w: &tauri::Window, api: &tauri::CloseRequestApi) {
    if QUITTING.load(Ordering::Relaxed) { return; }
    api.prevent_close();
    if core::set_bool("minimizeToTray") {
        let _ = w.hide();
        if !TRAY_HINT.swap(true, Ordering::Relaxed) {
            if let Some(app) = APP.get() {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().builder().title("MC Admin Panel").body("Панель работает в трее. Серверы продолжают работать.").show();
            }
        }
    } else if let Some(app) = APP.get() { quit(app); }
}
static QUIT_DIALOG: Mutex<bool> = Mutex::new(false);
pub fn quit(app: &AppHandle) {
    let running: Vec<String> = servers::running_ids();
    if running.is_empty() { QUITTING.store(true, Ordering::Relaxed); app.exit(0); return; }
    {
        let mut d = QUIT_DIALOG.lock();
        if *d { return; }
        *d = true;
    }
    let names: Vec<String> = running.iter().filter_map(|id| servers::get(id).ok().map(|s| s.name)).collect();
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    let app2 = app.clone();
    let mut dlg = app.dialog()
        .message(format!("Запущено серверов: {} ({}).\n\nОни будут корректно остановлены с сохранением мира.", names.len(), names.join(", ")))
        .title("Выход из MC Admin Panel")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom("Остановить серверы и выйти".into(), "Отмена".into()));
    if let Some(w) = win().filter(|w| w.is_visible().unwrap_or(false)) { dlg = dlg.parent(&w); }
    dlg.show(move |ok| {
        *QUIT_DIALOG.lock() = false;
        if !ok { return; }
        tauri::async_runtime::spawn(async move {
            if let Some(t) = app2.tray_by_id("main") { let _ = t.set_tooltip(Some("MC Admin Panel — остановка серверов…")); }
            if let Some(w) = win() { let _ = w.set_title("MC Admin Panel — остановка серверов…"); }
            core::emit(serde_json::json!({ "type": "quitting" }));
            servers::stop_all().await;
            QUITTING.store(true, Ordering::Relaxed);
            app2.exit(0);
        });
    });
}

pub fn init_listeners() {
    core::on_event(|m| {
        match m["type"].as_str().unwrap_or("") {
            "status" | "servers" | "players" => refresh_tray(),
            "job" => update_taskbar(),
            _ => {}
        }
    });
    core::SETTINGS_HOOKS.lock().push(Box::new(|| { apply_login_item(); refresh_tray(); }));
}
