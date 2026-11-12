//! Опциональный веб-доступ к панели из браузера и API для мобильного приложения
//! (оба по умолчанию выключены). Те же команды, события — через WebSocket.
use crate::core::{self, APP, BUS};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path, Query, Request};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, post};
use axum::{Json, Router};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::oneshot;

static RUNNING: Lazy<Mutex<Option<(String, oneshot::Sender<()>)>>> = Lazy::new(|| Mutex::new(None));
pub static STATE: Lazy<Mutex<Value>> = Lazy::new(|| Mutex::new(json!({ "running": false })));

#[derive(Debug, Clone)]
struct Cfg { host: String, port: u16, pw: String, web_ui: bool, mobile: bool }

fn config() -> Option<Cfg> {
    let web_ui = core::set_bool("webEnabled");
    let mobile = crate::mobile::enabled();
    if !web_ui && !mobile { return None; }
    let pw = core::set_str("webPassword");
    // из сети — только с паролем (браузер) или токеном устройства (телефон)
    let lan = mobile || (web_ui && core::set_bool("webLan") && !pw.is_empty());
    Some(Cfg { host: if lan { "0.0.0.0" } else { "127.0.0.1" }.into(), port: core::set_u64("port").clamp(1, 65535) as u16, pw, web_ui, mobile })
}

/// Запускает, перезапускает или останавливает веб-сервер по текущим настройкам.
pub fn apply() {
    let cfg = config();
    let key = format!("{cfg:?}");
    let mut r = RUNNING.lock();
    if r.as_ref().map(|x| x.0 == key).unwrap_or(cfg.is_none()) { return; }
    if let Some((_, tx)) = r.take() { let _ = tx.send(()); }
    *STATE.lock() = json!({ "running": false });
    let Some(cfg) = cfg else { return };
    let (host, port) = (cfg.host.clone(), cfg.port);
    let (tx, rx) = oneshot::channel();
    *r = Some((key, tx));
    tauri::async_runtime::spawn(async move {
        let c2 = cfg.clone();
        let app = Router::new()
            .route("/api/call/{cmd}", post(call))
            .route("/api/events", any(events))
            .route("/api/pair", post(pair))
            .route("/api/ping", any(|| async { Json(json!({ "app": "mc-admin-panel", "version": env!("CARGO_PKG_VERSION") })) }))
            .fallback(asset)
            .layer(middleware::from_fn(move |req: Request, next: Next| { let c = c2.clone(); async move { auth(c, req, next).await } }));
        match tokio::net::TcpListener::bind((host.as_str(), port)).await {
            Ok(l) => {
                *STATE.lock() = json!({ "running": true, "url": format!("http://{}:{port}", if host == "0.0.0.0" { "IP-компьютера" } else { "localhost" }) });
                let _ = axum::serve(l, app.into_make_service_with_connect_info::<SocketAddr>()).with_graceful_shutdown(async { let _ = rx.await; }).await;
            }
            Err(e) => { *STATE.lock() = json!({ "running": false, "error": format!("Порт {port} занят или недоступен: {e}") }); }
        }
    });
}

async fn auth(cfg: Cfg, req: Request, next: Next) -> Response {
    let p = req.uri().path().to_string();
    let local = req.extensions().get::<ConnectInfo<SocketAddr>>().map(|c| c.0.ip().is_loopback()).unwrap_or(false);
    // веб-интерфейс отдаётся только при включённом веб-доступе
    if !p.starts_with("/api/") { return if cfg.web_ui { next.run(req).await } else { StatusCode::NOT_FOUND.into_response() }; }
    if p == "/api/pair" || p == "/api/ping" { return next.run(req).await; }
    let hdr = req.headers().get("x-token").and_then(|v| v.to_str().ok()).map(String::from)
        .or_else(|| req.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).map(String::from));
    let q = req.uri().query().unwrap_or("").split('&').find_map(|kv| kv.strip_prefix("token=")).map(|t| urlencoding::decode(t).map(|s| s.into_owned()).unwrap_or_default());
    let token = hdr.or(q).unwrap_or_default();
    let ok = (!cfg.pw.is_empty() && token == cfg.pw)
        || (cfg.mobile && crate::mobile::check_token(&token))
        // браузер на этом же компьютере без пароля — как раньше
        || (cfg.pw.is_empty() && local && cfg.web_ui);
    if ok { next.run(req).await } else { (StatusCode::UNAUTHORIZED, Json(json!({ "error": "auth" }))).into_response() }
}

async fn pair(body: Json<Value>) -> Response {
    let code = body["code"].as_str().unwrap_or("");
    let name = body["name"].as_str().unwrap_or("");
    match crate::mobile::pair_finish(code, name) {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::FORBIDDEN, Json(json!({ "error": e }))).into_response(),
    }
}

async fn call(Path(cmd): Path<String>, body: Option<Json<Value>>) -> Response {
    let args = body.map(|b| b.0).unwrap_or(json!({}));
    match crate::api::dispatch(&cmd, args, true).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    }
}

async fn events(ws: WebSocketUpgrade, q: Query<HashMap<String, String>>) -> Response {
    // ?types=status,crash — только нужные события (телефону не нужен поток статистики)
    let types: Vec<String> = q.get("types").map(|t| t.split(',').filter(|x| !x.is_empty()).map(|x| format!("\"type\":\"{x}\"")).collect()).unwrap_or_default();
    ws.on_upgrade(move |mut sock: WebSocket| async move {
        let mut rx = BUS.subscribe();
        loop {
            tokio::select! {
                m = rx.recv() => match m {
                    Ok(s) => if (types.is_empty() || types.iter().any(|t| s.contains(t.as_str()))) && sock.send(Message::Text(s.as_str().into())).await.is_err() { break },
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                },
                r = sock.recv() => if matches!(r, None | Some(Err(_)) | Some(Ok(Message::Close(_)))) { break },
            }
        }
    })
}

async fn asset(uri: Uri, _h: HeaderMap) -> Response {
    let Some(app) = APP.get() else { return StatusCode::SERVICE_UNAVAILABLE.into_response() };
    let mut p = uri.path().trim_start_matches('/').to_string();
    if p.is_empty() { p = "index.html".into(); }
    // в отладочной сборке отдаём UI прямо с диска — правки видны без пересборки
    #[cfg(debug_assertions)]
    if !p.contains("..") {
        let f = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui").join(&p);
        if let Ok(b) = std::fs::read(&f) {
            let mime = match f.extension().and_then(|e| e.to_str()) { Some("html") => "text/html", Some("css") => "text/css", Some("js") => "text/javascript", Some("svg") => "image/svg+xml", Some("png") => "image/png", _ => "application/octet-stream" };
            return ([(header::CONTENT_TYPE, format!("{mime}; charset=utf-8")), (header::CACHE_CONTROL, "no-cache".into())], b).into_response();
        }
    }
    match app.asset_resolver().get(p) {
        Some(a) => ([(header::CONTENT_TYPE, a.mime_type.clone()), (header::CACHE_CONTROL, "no-cache".into())], a.bytes.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
