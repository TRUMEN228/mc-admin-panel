//! Версии Minecraft и загрузчиков (Fabric, Forge, NeoForge, Paper).
use crate::{http, Res};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};

const MOJANG: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

static CACHE: Lazy<Mutex<HashMap<String, (Instant, Value)>>> = Lazy::new(|| Mutex::new(HashMap::new()));
async fn cached<F: Future<Output = Res<Value>>>(k: &str, ttl: u64, f: impl FnOnce() -> F) -> Res<Value> {
    if let Some((t, v)) = CACHE.lock().get(k) { if t.elapsed() < Duration::from_secs(ttl) { return Ok(v.clone()); } }
    let v = f().await?;
    CACHE.lock().insert(k.into(), (Instant::now(), v.clone()));
    Ok(v)
}
async fn maven_versions(url: &str) -> Res<Value> {
    let t = http::get_text(url).await?;
    let re = regex::Regex::new(r"<version>([^<]+)</version>").unwrap();
    let mut v: Vec<String> = re.captures_iter(&t).map(|c| c[1].to_string()).collect();
    v.reverse();
    Ok(json!(v))
}
fn strs(v: &Value) -> Vec<String> { v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() }

pub async fn mc_versions(snapshots: bool) -> Res<Value> {
    let m = cached("mojang", 3600, || http::get_json(MOJANG)).await?;
    let list: Vec<Value> = m["versions"].as_array().cloned().unwrap_or_default().into_iter()
        .filter(|v| snapshots || v["type"] == "release")
        .map(|v| json!({ "id": v["id"], "type": v["type"], "time": v["releaseTime"] })).collect();
    Ok(json!(list))
}
pub async fn mc_version_info(id: &str) -> Res<Value> {
    let m = cached("mojang", 3600, || http::get_json(MOJANG)).await?;
    let v = m["versions"].as_array().and_then(|a| a.iter().find(|x| x["id"] == id)).cloned().ok_or(format!("Неизвестная версия {id}"))?;
    let url = v["url"].as_str().unwrap_or("").to_string();
    cached(&format!("ver:{id}"), 86400, || async move { http::get_json(&url).await }).await
}

/// 21.1.77 -> 1.21.1, 20.4.x -> 1.20.4, 21.0.x -> 1.21, 26.1.0.x -> 26.1
pub fn neo_to_mc(v: &str) -> String {
    let p: Vec<&str> = v.split(['.', '-']).collect();
    let a = p.first().copied().unwrap_or("0");
    let b = p.get(1).copied().unwrap_or("0");
    if a.parse::<u32>().unwrap_or(0) >= 26 {
        return match p.get(2) { Some(c) if *c != "0" => format!("{a}.{b}.{c}"), _ => format!("{a}.{b}") };
    }
    if b == "0" { format!("1.{a}") } else { format!("1.{a}.{b}") }
}
fn cmp_num(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| s.split(|c: char| !c.is_ascii_digit()).filter(|x| !x.is_empty()).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parse(a).cmp(&parse(b))
}

pub async fn loader_versions(loader: &str, mc: &str) -> Res<Value> {
    Ok(match loader {
        "fabric" => {
            let l = cached("fabric", 3600, || http::get_json("https://meta.fabricmc.net/v2/versions/loader")).await?;
            json!(l.as_array().map(|a| a.iter().filter_map(|x| x["version"].as_str()).collect::<Vec<_>>()).unwrap_or_default())
        }
        "forge" => {
            let all = cached("forge", 3600, || maven_versions("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")).await?;
            let pre = format!("{mc}-");
            json!(strs(&all).into_iter().filter(|v| v.starts_with(&pre)).map(|v| v[pre.len()..].to_string()).collect::<Vec<_>>())
        }
        "neoforge" => {
            let all = cached("neoforge", 3600, || maven_versions("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml")).await?;
            let mut v: Vec<String> = strs(&all).into_iter().filter(|v| neo_to_mc(v) == mc).collect();
            v.sort_by(|a, b| cmp_num(b, a));
            json!(v)
        }
        "paper" => {
            let b = http::get_json(&format!("https://fill.papermc.io/v3/projects/paper/versions/{mc}/builds")).await.unwrap_or(json!([]));
            json!(b.as_array().map(|a| a.iter().map(|x| x["id"].to_string()).collect::<Vec<_>>()).unwrap_or_default())
        }
        _ => json!([]),
    })
}

