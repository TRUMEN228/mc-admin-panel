//! Подключение к серверу: адреса, правило брандмауэра Windows, проверка доступности извне.
use crate::core::{Ctx, Res};
use crate::{http, servers};
use serde_json::{json, Value};

/// Основной локальный IPv4 (без отправки пакетов: UDP connect только выбирает маршрут).
fn local_ip() -> Option<String> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}
fn rule_name(port: u16) -> String { format!("MC Admin Panel — порт {port}") }

#[cfg(windows)]
fn netsh(args: &[&str]) -> Option<std::process::Output> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("netsh").args(args).creation_flags(0x0800_0000).output().ok()
}

pub fn firewall_has_rule(port: u16) -> bool {
    #[cfg(windows)]
    { return netsh(&["advfirewall", "firewall", "show", "rule", &format!("name={}", rule_name(port))]).map(|o| o.status.success()).unwrap_or(false); }
    #[cfg(not(windows))]
    { let _ = port; true }
}

pub async fn info(id: &str) -> Res<Value> {
    let s = servers::get(id)?;
    let external = match http::client().get("https://api.ipify.org").send().await { Ok(r) => r.text().await.ok(), Err(_) => None };
    Ok(json!({ "port": s.port, "localIp": local_ip(), "externalIp": external, "firewall": firewall_has_rule(s.port) }))
}

/// Добавляет входящее правило для TCP-порта сервера (Windows запросит права администратора).
pub async fn firewall_add(id: &str) -> Res<Value> {
    let s = servers::get(id)?;
    if firewall_has_rule(s.port) { return Ok(json!({ "ok": true })); }
    #[cfg(windows)]
    {
        let args = format!("advfirewall firewall add rule name=\"{}\" dir=in action=allow protocol=TCP localport={}", rule_name(s.port), s.port);
        let ps = format!("Start-Process -FilePath netsh -ArgumentList '{}' -Verb RunAs -WindowStyle Hidden -Wait", args.replace('\'', "''"));
        let st = crate::core::command("powershell").args(["-NoProfile", "-Command", &ps]).status().await.ctx("Запуск netsh")?;
        if !st.success() { crate::bail!("Правило не добавлено (запрос прав администратора отклонён?)"); }
    }
    Ok(json!({ "ok": firewall_has_rule(s.port) }))
}

/// Проверка извне через mcsrvstat.us: сервис пингует сервер по внешнему адресу.
pub async fn check(id: &str) -> Res<Value> {
    let s = servers::get(id)?;
    if servers::status(id) != "running" { crate::bail!("Сначала запустите сервер"); }
    let ip = http::client().get("https://api.ipify.org").send().await.map_err(|e| format!("Не удалось узнать внешний IP: {e}"))?.text().await.ctx("")?;
    let r = http::get_json(&format!("https://api.mcsrvstat.us/3/{}:{}", ip.trim(), s.port)).await?;
    Ok(json!({ "online": r["online"] == true, "address": format!("{}:{}", ip.trim(), s.port), "motd": r["motd"]["clean"], "players": r["players"], "version": r["version"] }))
}
