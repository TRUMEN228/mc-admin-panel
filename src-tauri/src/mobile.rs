//! Мобильное приложение: привязка телефона по QR-коду и токены устройств.
//! Телефон подключается к веб-серверу панели по Wi-Fi или через Tailscale.
use crate::core::{self, now_ms, Ctx, Res, DATA};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const CODE_TTL_MS: u64 = 10 * 60_000;
const MAX_ATTEMPTS: u32 = 5;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Device { pub id: String, pub name: String, pub hash: String, pub created: u64, #[serde(default)] pub last_seen: u64 }

fn file() -> PathBuf { DATA.join("devices.json") }
static DEVICES: Lazy<Mutex<Vec<Device>>> = Lazy::new(|| Mutex::new(core::read_json(&file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()));
fn save() { let d = DEVICES.lock().clone(); let _ = core::write_json(&file(), &d); }

struct Pairing { code: String, expires: u64, attempts: u32 }
static PAIRING: Lazy<Mutex<Option<Pairing>>> = Lazy::new(|| Mutex::new(None));

fn random_bytes<const N: usize>() -> [u8; N] { let mut b = [0u8; N]; getrandom::getrandom(&mut b).expect("random"); b }
fn hash(token: &str) -> String { hex::encode(Sha256::digest(token.as_bytes())) }
pub fn enabled() -> bool { core::set_bool("mobileEnabled") }

/// Адреса компьютера для телефона: сначала локальная сеть, затем Tailscale (100.64.0.0/10).
pub fn addresses() -> Vec<Value> {
    let mut lan = vec![];
    let mut ts = vec![];
    for i in if_addrs::get_if_addrs().unwrap_or_default() {
        let std::net::IpAddr::V4(ip) = i.ip() else { continue };
        if ip.is_loopback() || ip.is_link_local() { continue; }
        let n = i.name.to_lowercase();
        if ["vethernet", "wsl", "hyper-v", "virtualbox", "vmware", "docker"].iter().any(|v| n.contains(v)) { continue; }
        let o = ip.octets();
        if o[0] == 100 && (64..128).contains(&o[1]) { ts.push(json!({ "ip": ip.to_string(), "kind": "tailscale" })); }
        else if ip.is_private() { lan.push(json!({ "ip": ip.to_string(), "kind": "lan", "iface": i.name })); }
    }
    lan.extend(ts);
    lan
}

/// Начать привязку: одноразовый код и QR-код с адресами панели.
pub fn pair_start() -> Res<Value> {
    if !enabled() { crate::bail!("Сначала включите доступ для мобильного приложения"); }
    let r = random_bytes::<4>();
    let code = format!("{:06}", u32::from_le_bytes(r) % 1_000_000);
    let expires = now_ms() + CODE_TTL_MS;
    *PAIRING.lock() = Some(Pairing { code: code.clone(), expires, attempts: 0 });
    let addrs = addresses();
    let port = core::set_u64("port").clamp(1, 65535);
    let name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "ПК".into());
    let payload = json!({ "v": 1, "n": name, "p": port, "c": code, "a": addrs.iter().map(|a| a["ip"].clone()).collect::<Vec<_>>() }).to_string();
    let qr = qrcode::QrCode::with_error_correction_level(payload.as_bytes(), qrcode::EcLevel::M).ctx("QR-код")?;
    let svg = qr.render::<qrcode::render::svg::Color>().min_dimensions(240, 240).quiet_zone(true).build();
    Ok(json!({ "code": code, "expires": expires, "port": port, "addresses": addrs, "name": name, "qr": svg, "tailscale": addrs.iter().any(|a| a["kind"] == "tailscale") }))
}
pub fn pair_cancel() { *PAIRING.lock() = None; }

/// Телефон присылает код — получает постоянный токен устройства.
pub fn pair_finish(code: &str, device_name: &str) -> Res<Value> {
    let mut p = PAIRING.lock();
    let Some(cur) = p.as_mut() else { crate::bail!("Привязка не начата — нажмите «Подключить телефон» в панели") };
    if now_ms() > cur.expires { *p = None; crate::bail!("Код устарел — начните привязку заново"); }
    if cur.code != code.trim() {
        cur.attempts += 1;
        if cur.attempts >= MAX_ATTEMPTS { *p = None; crate::bail!("Слишком много неверных попыток — начните привязку заново"); }
        crate::bail!("Неверный код");
    }
    *p = None;
    drop(p);
    let token = hex::encode(random_bytes::<32>());
    let name = if device_name.trim().is_empty() { "Телефон".to_string() } else { device_name.trim().chars().take(60).collect() };
    let d = Device { id: core::new_id(), name: name.clone(), hash: hash(&token), created: now_ms(), last_seen: now_ms() };
    DEVICES.lock().push(d.clone());
    save();
    core::emit(json!({ "type": "devices" }));
    crate::native::notify("Телефон подключён", &format!("«{name}» получил доступ к панели"));
    Ok(json!({ "token": token, "deviceId": d.id, "panel": std::env::var("COMPUTERNAME").unwrap_or_default(), "version": env!("CARGO_PKG_VERSION") }))
}

/// Проверка токена устройства (обновляет «последний вход» не чаще раза в минуту).
pub fn check_token(token: &str) -> bool {
    if token.len() < 32 { return false; }
    let h = hash(token);
    let mut l = DEVICES.lock();
    let Some(d) = l.iter_mut().find(|d| d.hash == h) else { return false };
    if now_ms() - d.last_seen > 60_000 { d.last_seen = now_ms(); drop(l); save(); }
    true
}
pub fn devices() -> Value { json!(DEVICES.lock().iter().map(|d| json!({ "id": d.id, "name": d.name, "created": d.created, "lastSeen": d.last_seen })).collect::<Vec<_>>()) }
pub fn revoke(id: &str) -> Res<()> {
    let before = DEVICES.lock().len();
    DEVICES.lock().retain(|d| d.id != id);
    if DEVICES.lock().len() == before { crate::bail!("Устройство не найдено"); }
    save();
    core::emit(json!({ "type": "devices" }));
    Ok(())
}
