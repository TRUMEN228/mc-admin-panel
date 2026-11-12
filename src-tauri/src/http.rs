//! HTTP-клиент с поддержкой прокси (в т.ч. системного прокси Windows).
use crate::core::{set_str, Ctx, Res};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

const UA: &str = concat!("mc-admin-panel/", env!("CARGO_PKG_VERSION"));

/// Системный прокси Windows (Настройки → Сеть → Прокси): его выставляют v2rayN, Clash и т.п.
fn windows_proxy() -> String {
    static CACHE: Lazy<Mutex<Option<(Instant, String)>>> = Lazy::new(|| Mutex::new(None));
    if let Some((t, v)) = &*CACHE.lock() { if t.elapsed() < Duration::from_secs(30) { return v.clone(); } }
    let mut v = String::new();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(o) = std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings"])
            .creation_flags(0x0800_0000).output()
        {
            let out = String::from_utf8_lossy(&o.stdout);
            let en = regex::Regex::new(r"ProxyEnable\s+REG_DWORD\s+0x1").unwrap().is_match(&out);
            let srv = regex::Regex::new(r"ProxyServer\s+REG_SZ\s+(\S+)").unwrap().captures(&out).map(|c| c[1].to_string());
            if let (true, Some(srv)) = (en, srv) {
                let pick = |k: &str| regex::Regex::new(&format!("{k}=([^;]+)")).unwrap().captures(&srv).map(|c| c[1].to_string());
                let hp = pick("https").or_else(|| pick("http")).unwrap_or_else(|| srv.split(';').next().unwrap_or("").to_string());
                v = if hp.contains("://") { hp } else { format!("http://{hp}") };
            }
        }
    }
    *CACHE.lock() = Some((Instant::now(), v.clone()));
    v
}

pub fn proxy_url() -> String {
    let p = set_str("proxy");
    if p == "auto" {
        for k in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
            if let Ok(v) = std::env::var(k) { if !v.is_empty() { return v; } }
        }
        return windows_proxy();
    }
    p
}

static CLIENT: Lazy<Mutex<Option<(String, reqwest::Client)>>> = Lazy::new(|| Mutex::new(None));
pub fn client() -> reqwest::Client {
    let p = proxy_url();
    let mut c = CLIENT.lock();
    if let Some((k, cl)) = &*c { if *k == p { return cl.clone(); } }
    let mut b = reqwest::Client::builder().user_agent(UA).connect_timeout(Duration::from_secs(20)).no_proxy();
    if !p.is_empty() { if let Ok(px) = reqwest::Proxy::all(&p) { b = b.proxy(px); } }
    let cl = b.build().expect("http client");
    *c = Some((p, cl.clone()));
    cl
}

/// Аргументы JVM, чтобы установщики Forge/NeoForge и сам сервер ходили через тот же прокси.
pub fn java_proxy_args() -> Vec<String> {
    let p = proxy_url();
    let Ok(u) = reqwest::Url::parse(&p) else { return vec![] };
    let Some(host) = u.host_str() else { return vec![] };
    let port = u.port_or_known_default().unwrap_or(80);
    let mut a: Vec<String> = ["http", "https"].iter().flat_map(|s| [format!("-D{s}.proxyHost={host}"), format!("-D{s}.proxyPort={port}")]).collect();
    a.push("-Dhttp.nonProxyHosts=localhost|127.*".into());
    a
}

async fn get(url: &str) -> Res<reqwest::Response> {
    let r = client().get(url).send().await.map_err(|e| format!("Сеть: {e} ({url})"))?;
    if !r.status().is_success() { crate::bail!("HTTP {} {url}", r.status().as_u16()); }
    Ok(r)
}
pub async fn get_json(url: &str) -> Res<Value> { get(url).await?.json().await.ctx("Неверный ответ сервера") }
pub async fn get_text(url: &str) -> Res<String> { get(url).await?.text().await.ctx("Ошибка чтения ответа") }
pub async fn post_json(url: &str, body: &Value) -> Res<Value> {
    let r = client().post(url).json(body).send().await.map_err(|e| format!("Сеть: {e}"))?;
    if !r.status().is_success() { crate::bail!("HTTP {} {url}", r.status().as_u16()); }
    r.json().await.ctx("Неверный ответ сервера")
}

/// Скачивание во временный .part с последующим переименованием; прогресс 0..1.
pub async fn download(url: &str, dest: &Path, mut on_progress: impl FnMut(f64)) -> Res<()> {
    if let Some(p) = dest.parent() { tokio::fs::create_dir_all(p).await.ctx("Создание папки")?; }
    let r = get(url).await?;
    let total = r.content_length().unwrap_or(0);
    let part = dest.with_file_name(format!("{}.part", dest.file_name().unwrap().to_string_lossy()));
    let mut f = tokio::fs::File::create(&part).await.ctx("Создание файла")?;
    let mut got = 0u64;
    let mut last = Instant::now();
    let mut s = r.bytes_stream();
    while let Some(chunk) = s.next().await {
        let chunk = chunk.map_err(|e| format!("Загрузка прервана: {e}"))?;
        got += chunk.len() as u64;
        f.write_all(&chunk).await.ctx("Запись файла")?;
        if total > 0 && last.elapsed() > Duration::from_millis(300) { last = Instant::now(); on_progress(got as f64 / total as f64); }
    }
    f.flush().await.ctx("")?;
    drop(f);
    tokio::fs::rename(&part, dest).await.ctx("Переименование")
}
