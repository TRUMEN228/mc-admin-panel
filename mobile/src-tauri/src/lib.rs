//! MC Admin Mobile — компаньон MC Admin Panel: управление серверами с телефона.
//! Запросы к панели идут из Rust (не из WebView): так работает обычный HTTP в локальной сети и Tailscale.
use futures_util::StreamExt;
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{Emitter, Manager};

type Res<T> = Result<T, String>;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct Panel { id: String, name: String, addresses: Vec<String>, port: u16, token: String, #[serde(default)] good: String }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Store {
    panels: Vec<Panel>,
    active: String,
    #[serde(default = "yes")] notify_crash: bool,
    #[serde(default = "yes")] notify_players: bool,
    #[serde(default = "yes")] notify_jobs: bool,
    #[serde(default = "yes")] notify_alerts: bool,
    #[serde(default = "yes")] background: bool,
}
fn yes() -> bool { true }
impl Default for Store { fn default() -> Self { Store { panels: vec![], active: String::new(), notify_crash: true, notify_players: true, notify_jobs: true, notify_alerts: true, background: true } } }

static DIR: OnceCell<PathBuf> = OnceCell::new();
static STORE: Lazy<Mutex<Store>> = Lazy::new(|| Mutex::new(read(&dir().join("panels.json")).unwrap_or_default()));
static APP: OnceCell<tauri::AppHandle> = OnceCell::new();
static WS: Lazy<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

fn dir() -> PathBuf { DIR.get().cloned().unwrap_or_else(std::env::temp_dir) }
fn read<T: for<'a> Deserialize<'a>>(p: &PathBuf) -> Option<T> { std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()) }
fn save(s: &Store) {
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(dir().join("panels.json"), serde_json::to_string_pretty(s).unwrap_or_default());
    // конфигурация для фоновой службы уведомлений (Kotlin): активная панель и что показывать
    let p = s.panels.iter().find(|p| p.id == s.active);
    let cfg = match p {
        Some(p) if s.background => {
            let mut addrs = vec![];
            if !p.good.is_empty() { addrs.push(p.good.clone()); }
            addrs.extend(p.addresses.iter().filter(|a| **a != p.good).cloned());
            json!({ "name": p.name, "port": p.port, "token": p.token, "addresses": addrs, "crash": s.notify_crash, "players": s.notify_players, "jobs": s.notify_jobs, "alerts": s.notify_alerts })
        }
        _ => json!({}),
    };
    let _ = std::fs::write(dir().join("notify.json"), cfg.to_string());
    if let Some(app) = APP.get() { let _ = app.emit("store", json!({})); }
}
fn active() -> Res<Panel> { let s = STORE.lock(); s.panels.iter().find(|p| p.id == s.active).cloned().ok_or("Панель не выбрана".into()) }

fn client(timeout: u64) -> reqwest::Client {
    reqwest::Client::builder().connect_timeout(Duration::from_secs(4)).timeout(Duration::from_secs(timeout)).user_agent("mc-admin-mobile").build().unwrap()
}
fn ordered(p: &Panel) -> Vec<String> {
    let mut v = vec![];
    if !p.good.is_empty() { v.push(p.good.clone()); }
    v.extend(p.addresses.iter().filter(|a| **a != p.good).cloned());
    v
}
fn remember_good(id: &str, addr: &str) {
    let mut s = STORE.lock();
    if let Some(p) = s.panels.iter_mut().find(|p| p.id == id) {
        if p.good != addr { p.good = addr.into(); let c = s.clone(); drop(s); save(&c); }
    }
}

/// Запрос к панели: перебирает адреса (Wi-Fi, Tailscale), запоминает рабочий.
async fn request(p: &Panel, path: &str, body: &Value, timeout: u64) -> Res<Value> {
    let mut last = "Нет адресов панели".to_string();
    for a in ordered(p) {
        let url = format!("http://{a}:{}{path}", p.port);
        match client(timeout).post(&url).bearer_auth(&p.token).json(body).send().await {
            Ok(r) => {
                remember_good(&p.id, &a);
                let st = r.status();
                let v: Value = r.json().await.unwrap_or(json!({}));
                if st.as_u16() == 401 { return Err("Доступ отозван — привяжите телефон заново".into()); }
                if !st.is_success() { return Err(v["error"].as_str().unwrap_or("Ошибка панели").to_string()); }
                return Ok(v);
            }
            Err(e) => last = if e.is_timeout() || e.is_connect() { "Панель недоступна: компьютер выключен, панель не запущена или нет связи (Wi-Fi / Tailscale)".into() } else { e.to_string() },
        }
    }
    Err(last)
}

#[tauri::command]
fn state() -> Value { let s = STORE.lock().clone(); json!({ "panels": s.panels.iter().map(|p| json!({ "id": p.id, "name": p.name, "addresses": p.addresses, "port": p.port, "good": p.good })).collect::<Vec<_>>(), "active": s.active, "notifyCrash": s.notify_crash, "notifyPlayers": s.notify_players, "notifyJobs": s.notify_jobs, "notifyAlerts": s.notify_alerts, "background": s.background }) }

/// Привязка по коду из панели (QR-код или ручной ввод).
#[tauri::command]
async fn pair(addresses: Vec<String>, port: u16, code: String, device: String, name: String) -> Res<Value> {
    let mut last = String::from("Укажите адрес компьютера");
    for a in addresses.iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()) {
        let url = format!("http://{a}:{port}/api/pair");
        match client(8).post(&url).json(&json!({ "code": code.replace(' ', ""), "name": device })).send().await {
            Ok(r) => {
                let ok = r.status().is_success();
                let v: Value = r.json().await.unwrap_or(json!({}));
                if !ok { return Err(v["error"].as_str().unwrap_or("Панель отклонила код").into()); }
                let p = Panel {
                    id: v["deviceId"].as_str().unwrap_or("p").into(),
                    name: if name.trim().is_empty() { v["panel"].as_str().filter(|s| !s.is_empty()).unwrap_or("Мой ПК").into() } else { name.trim().into() },
                    addresses: addresses.clone(), port, token: v["token"].as_str().unwrap_or("").into(), good: a.clone(),
                };
                let mut s = STORE.lock();
                s.panels.retain(|x| x.name != p.name || x.addresses != p.addresses);
                s.active = p.id.clone();
                s.panels.push(p.clone());
                let c = s.clone();
                drop(s);
                save(&c);
                return Ok(json!({ "id": p.id, "name": p.name }));
            }
            Err(e) => last = format!("{a}: нет ответа ({})", if e.is_timeout() || e.is_connect() { "проверьте, что телефон в той же сети или включён Tailscale".to_string() } else { e.to_string() }),
        }
    }
    Err(last)
}

#[tauri::command]
fn select(id: String) { let mut s = STORE.lock(); s.active = id; let c = s.clone(); drop(s); save(&c); }
#[tauri::command]
fn remove(id: String) {
    let mut s = STORE.lock();
    s.panels.retain(|p| p.id != id);
    if s.active == id { s.active = s.panels.first().map(|p| p.id.clone()).unwrap_or_default(); }
    let c = s.clone(); drop(s); save(&c);
}
#[tauri::command]
fn settings(patch: Value) -> Value {
    let mut s = STORE.lock();
    if let Some(v) = patch["notifyCrash"].as_bool() { s.notify_crash = v; }
    if let Some(v) = patch["notifyPlayers"].as_bool() { s.notify_players = v; }
    if let Some(v) = patch["notifyJobs"].as_bool() { s.notify_jobs = v; }
    if let Some(v) = patch["notifyAlerts"].as_bool() { s.notify_alerts = v; }
    if let Some(v) = patch["background"].as_bool() { s.background = v; }
    let c = s.clone(); drop(s); save(&c);
    state()
}

/// Команда панели (те же, что у веб-доступа: servers.list, servers.start, …).
#[tauri::command]
async fn call(cmd: String, args: Option<Value>) -> Res<Value> {
    let p = active()?;
    let long = cmd.contains("install") || cmd.contains("create") || cmd.ends_with(".stop") || cmd.ends_with(".restart");
    request(&p, &format!("/api/call/{cmd}"), &args.unwrap_or(json!({})), if long { 90 } else { 15 }).await
}

/// Поток событий панели для открытого приложения (статусы, статистика, консоль).
#[tauri::command]
fn events(types: String) {
    if let Some(h) = WS.lock().take() { h.abort(); }
    let Ok(p) = active() else { return };
    let h = tauri::async_runtime::spawn(async move {
        let mut delay = 1;
        loop {
            for a in ordered(&p) {
                let url = format!("ws://{a}:{}/api/events?token={}&types={}", p.port, p.token, types);
                if let Ok(Ok((mut ws, _))) = tokio::time::timeout(Duration::from_secs(5), tokio_tungstenite::connect_async(&url)).await {
                    delay = 1;
                    remember_good(&p.id, &a);
                    if let Some(app) = APP.get() { let _ = app.emit("conn", json!({ "online": true, "address": a })); }
                    while let Some(Ok(m)) = ws.next().await {
                        if let tokio_tungstenite::tungstenite::Message::Text(t) = m {
                            if let (Some(app), Ok(v)) = (APP.get(), serde_json::from_str::<Value>(&t)) { let _ = app.emit("msg", v); }
                        }
                    }
                    break;
                }
            }
            if let Some(app) = APP.get() { let _ = app.emit("conn", json!({ "online": false })); }
            tokio::time::sleep(Duration::from_secs(delay)).await;
            delay = (delay * 2).min(30);
        }
    });
    *WS.lock() = Some(h);
}
#[tauri::command]
fn events_stop() { if let Some(h) = WS.lock().take() { h.abort(); } }

/// Есть ли новая версия приложения в релизах на GitHub (APK прикладывается к релизу панели).
#[tauri::command]
async fn update_check() -> Res<Value> {
    let cur = env!("CARGO_PKG_VERSION");
    let r: Value = client(15).get("https://api.github.com/repos/TRUMEN228/mc-admin-panel/releases/latest")
        .header("Accept", "application/vnd.github+json").send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    let ver = |v: &str| v.split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    for a in r["assets"].as_array().cloned().unwrap_or_default() {
        let name = a["name"].as_str().unwrap_or("");
        if let Some(v) = name.strip_prefix("MCAdminMobile-").and_then(|x| x.strip_suffix(".apk")) {
            return Ok(json!({ "current": cur, "version": v, "available": ver(v) > ver(cur), "url": a["browser_download_url"], "size": a["size"], "page": r["html_url"] }));
        }
    }
    Ok(json!({ "current": cur, "available": false }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let b = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    #[cfg(mobile)]
    let b = b.plugin(tauri_plugin_barcode_scanner::init());
    b.invoke_handler(tauri::generate_handler![state, pair, select, remove, settings, call, events, events_stop, update_check])
        .setup(|app| {
            let _ = APP.set(app.handle().clone());
            let d = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir());
            let _ = DIR.set(d);
            let s = STORE.lock().clone();
            save(&s);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ошибка запуска MC Admin Mobile");
}
