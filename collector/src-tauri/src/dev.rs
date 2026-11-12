//! Только для отладочной сборки: интерфейс в обычном браузере на http://127.0.0.1:7791
//! (те же команды, события через WebSocket). В релиз не попадает.
use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::Path;
use axum::http::{header, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{any, post};
use axum::{Json, Router};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

pub static BUS: Lazy<broadcast::Sender<Arc<String>>> = Lazy::new(|| broadcast::channel(1024).0);

pub fn start() {
    tauri::async_runtime::spawn(async {
        let app = Router::new()
            .route("/api/call/{cmd}", post(|Path(cmd): Path<String>, body: Option<Json<Value>>| async move {
                match crate::dispatch(&cmd, body.map(|b| b.0).unwrap_or(json!({}))).await {
                    Ok(v) => Json(v).into_response(),
                    Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
                }
            }))
            .route("/api/events", any(|ws: WebSocketUpgrade| async move {
                ws.on_upgrade(|mut s| async move {
                    let mut rx = BUS.subscribe();
                    while let Ok(m) = rx.recv().await { if s.send(Message::Text(m.as_str().into())).await.is_err() { break; } }
                })
            }))
            .fallback(|uri: Uri| async move {
                let p = match uri.path().trim_start_matches('/') { "" => "index.html", p => p }.to_string();
                if p.contains("..") { return StatusCode::NOT_FOUND.into_response(); }
                let f = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui").join(&p);
                let mime = match f.extension().and_then(|e| e.to_str()) { Some("html") => "text/html", Some("css") => "text/css", Some("js") => "text/javascript", _ => "application/octet-stream" };
                match std::fs::read(&f) { Ok(b) => ([(header::CONTENT_TYPE, format!("{mime}; charset=utf-8")), (header::CACHE_CONTROL, "no-cache".into())], b).into_response(), Err(_) => StatusCode::NOT_FOUND.into_response() }
            });
        if let Ok(l) = tokio::net::TcpListener::bind("127.0.0.1:7791").await { let _ = axum::serve(l, app).await; }
    });
}
pub fn emit(msg: &Value) { if BUS.receiver_count() > 0 { let _ = BUS.send(Arc::new(msg.to_string())); } }
