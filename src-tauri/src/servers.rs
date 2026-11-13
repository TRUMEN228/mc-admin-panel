//! Серверы: список, процессы, консоль, мониторинг, server.properties.
use crate::core::{self, emit, now_ms, Ctx, Job, Res, DATA};
use crate::{installer, native};
use futures_util::future::BoxFuture;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::{spawn, JoinHandle};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Launch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

fn d_true() -> bool { true }
fn d_port() -> u16 { 25565 }
fn d_ram() -> u32 { 4096 }

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub name: String,
    #[serde(default)] pub mc_version: String,
    #[serde(default)] pub loader: String,
    #[serde(default)] pub loader_version: String,
    pub dir: String,
    #[serde(default = "d_port")] pub port: u16,
    #[serde(default = "d_ram")] pub ram: u32,
    #[serde(default)] pub jvm_args: String,
    #[serde(default)] pub backup_dir: String,
    #[serde(default)] pub max_backups: u32,
    #[serde(default)] pub auto_backup_min: u32,
    #[serde(default = "d_true")] pub auto_restart: bool,
    #[serde(default)] pub auto_start: bool,
    #[serde(default)] pub installed: bool,
    #[serde(default)] pub imported: bool,
    #[serde(default)] pub created: u64,
    #[serde(default)] pub java_major: u32,
    #[serde(default)] pub launch: Launch,
    /// набор флагов JVM: "" | "aikar" | "zgc"
    #[serde(default)] pub jvm_preset: String,
    #[serde(flatten)] pub extra: Map<String, Value>,
}

fn file() -> PathBuf { DATA.join("servers.json") }
pub static LIST: Lazy<Mutex<Vec<Server>>> = Lazy::new(|| {
    let v: Vec<Server> = core::read_json(&file()).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    Mutex::new(v)
});
pub fn save() { let l = LIST.lock().clone(); if let Err(e) = core::write_json(&file(), &l) { eprintln!("servers.json: {e}"); } }
pub fn get(id: &str) -> Res<Server> { LIST.lock().iter().find(|s| s.id == id).cloned().ok_or("Сервер не найден".into()) }
fn with<R>(id: &str, f: impl FnOnce(&mut Server) -> R) -> Res<R> {
    let mut l = LIST.lock();
    let s = l.iter_mut().find(|s| s.id == id).ok_or("Сервер не найден")?;
    Ok(f(s))
}

// ---------- runtime ----------
#[derive(Serialize, Clone, Copy, Default)]
pub struct Stats { pub cpu: f64, pub mem: u64 }

#[derive(Default)]
pub struct Runtime {
    pub status: &'static str,
    pub log: VecDeque<String>,
    pub players: BTreeSet<String>,
    pub started: Option<u64>,
    pub stopping: bool,
    pub stats: Option<Stats>,
    /// история для графиков: [время, cpu, память МБ, игроки]
    pub history: VecDeque<(u64, f64, u64, usize)>,
    pub pid: Option<u32>,
    stdin: Option<Arc<tokio::sync::Mutex<tokio::process::ChildStdin>>>,
    kill_tx: Option<mpsc::UnboundedSender<()>>,
    exit_rx: Option<watch::Receiver<bool>>,
    waiters: Vec<(Regex, oneshot::Sender<String>)>,
    fetching_java: bool,
    /// TPS по последнему замеру; до какого момента ответы на замер скрываются из консоли
    pub tps: Option<f64>,
    probe_until: u64,
    low_tps: u8,
    /// открытые игровые сессии: игрок → время входа
    sessions: HashMap<String, u64>,
}
pub static RT: Lazy<Mutex<HashMap<String, Runtime>>> = Lazy::new(|| Mutex::new(HashMap::new()));
fn rt<R>(id: &str, f: impl FnOnce(&mut Runtime) -> R) -> R {
    let mut m = RT.lock();
    let r = m.entry(id.to_string()).or_insert_with(|| Runtime { status: "stopped", ..Default::default() });
    f(r)
}
/// Папка активного мира сервера (level-name в server.properties, по умолчанию «world»).
pub fn world_dir(s: &Server) -> PathBuf {
    let txt = std::fs::read_to_string(props_path(s)).unwrap_or_default();
    let name = parse_props(&txt).into_iter().find(|(k, _)| k == "level-name").map(|(_, v)| v).filter(|v| !v.trim().is_empty()).unwrap_or_else(|| "world".into());
    Path::new(&s.dir).join(name.trim())
}
pub fn online_players(id: &str) -> Vec<String> { rt(id, |r| r.players.iter().cloned().collect()) }
pub fn is_running(id: &str) -> bool { rt(id, |r| r.pid.is_some()) }
pub fn status(id: &str) -> &'static str { rt(id, |r| r.status) }

pub fn view(s: &Server) -> Value {
    let mut v = serde_json::to_value(s).unwrap_or(json!({}));
    rt(&s.id, |r| {
        v["status"] = json!(r.status);
        v["players"] = json!(r.players.iter().collect::<Vec<_>>());
        v["started"] = json!(r.started);
        v["tps"] = json!(r.tps);
        v["stats"] = json!(r.stats);
        v["history"] = json!(r.history.iter().map(|h| json!([h.0, h.1, h.2, h.3])).collect::<Vec<_>>());
    });
    v["icon"] = json!(icon_data(s));
    v
}
/// Иконка сервера (server-icon.png — её же показывает клиент Minecraft) как data-URL.
fn icon_data(s: &Server) -> String {
    use base64::Engine;
    std::fs::read(Path::new(&s.dir).join("server-icon.png")).ok().filter(|b| b.len() < 256 * 1024)
        .map(|b| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b))).unwrap_or_default()
}
/// Сменить иконку: PNG 64×64 (готовит интерфейс) в base64; пустая строка — убрать иконку.
pub fn set_icon(id: &str, data: &str) -> Res<Value> {
    use base64::Engine;
    let s = get(id)?;
    let p = Path::new(&s.dir).join("server-icon.png");
    if data.is_empty() { let _ = std::fs::remove_file(&p); } else {
        let b = base64::engine::general_purpose::STANDARD.decode(data.trim_start_matches("data:image/png;base64,")).map_err(|_| "Не удалось прочитать картинку".to_string())?;
        if !b.starts_with(&[0x89, b'P', b'N', b'G']) || b.len() > 256 * 1024 { return Err("Нужна PNG-картинка до 256 КБ".into()); }
        std::fs::write(&p, &b).ctx("Сохранение иконки")?;
    }
    emit(json!({ "type": "servers" }));
    Ok(json!({ "icon": icon_data(&s) }))
}
pub fn all() -> Value { let l = LIST.lock().clone(); json!(l.iter().map(view).collect::<Vec<_>>()) }
pub fn all_raw() -> Vec<Server> { LIST.lock().clone() }
pub fn log(id: &str) -> Value { rt(id, |r| json!(r.log.iter().collect::<Vec<_>>())) }

fn set_status(id: &str, st: &'static str) {
    let (prev, started) = rt(id, |r| (std::mem::replace(&mut r.status, st), r.started));
    if prev != st { emit(json!({ "type": "status", "id": id, "status": st, "started": started })); }
}

static RE_DONE: Lazy<Regex> = Lazy::new(|| Regex::new(r"Done \([\d.,]+s\)!").unwrap());
static RE_JOIN: Lazy<Regex> = Lazy::new(|| Regex::new(r": (\w{2,16}) joined the game").unwrap());
static RE_LEFT: Lazy<Regex> = Lazy::new(|| Regex::new(r": (\w{2,16}) left the game").unwrap());

static RE_COLOR: Lazy<Regex> = Lazy::new(|| Regex::new(r"§.").unwrap());
static RE_TPS_PAPER: Lazy<Regex> = Lazy::new(|| Regex::new(r"TPS from last 1m, 5m, 15m: \*?([\d.]+)").unwrap());
static RE_TPS_TICK: Lazy<Regex> = Lazy::new(|| Regex::new(r"Average time per tick: ([\d.]+) ?ms").unwrap());
static RE_TPS_FORGE: Lazy<Regex> = Lazy::new(|| Regex::new(r"Overall\s*: Mean tick time: ([\d.]+) ms\. Mean TPS: ([\d.]+)").unwrap());
static RE_TPS_ANY: Lazy<Regex> = Lazy::new(|| Regex::new(r"(TPS from last|The game is running|Target tick rate|Average time per tick|Percentiles|P50|Mean tick time|Current Memory Usage|The game is (frozen|sprinting|stepping))").unwrap());

/// TPS из строки ответа Paper (`tps`), Forge/NeoForge (`forge tps`) или ванильной `tick query`.
pub fn parse_tps(line: &str) -> Option<f64> {
    let clean = RE_COLOR.replace_all(line, "");
    if let Some(c) = RE_TPS_PAPER.captures(&clean) { return c[1].parse::<f64>().ok(); }
    if let Some(c) = RE_TPS_TICK.captures(&clean) { return c[1].parse::<f64>().ok().map(|ms| if ms <= 50.0 { 20.0 } else { 1000.0 / ms }); }
    if let Some(c) = RE_TPS_FORGE.captures(&clean) { return c[2].parse::<f64>().ok(); }
    None
}
/// Разбор ответа на замер TPS (строки ответа в консоль не попадают).
fn probe_line(id: &str, line: &str) -> bool {
    let now = now_ms();
    let clean = RE_COLOR.replace_all(line, "");
    let tps = parse_tps(&clean);
    let (hide, alert) = rt(id, |r| {
        if now > r.probe_until { return (false, false); }
        let mut alert = false;
        if let Some(t) = tps {
            let t = (t.min(20.0) * 10.0).round() / 10.0;
            r.tps = Some(t);
            if t < 15.0 { r.low_tps = r.low_tps.saturating_add(1); alert = r.low_tps == 2; } else if t >= 18.0 { r.low_tps = 0; }
        }
        (RE_TPS_ANY.is_match(&clean), alert)
    });
    if alert {
        let name = get(id).map(|s| s.name).unwrap_or_default();
        let t = rt(id, |r| r.tps).unwrap_or(0.0);
        crate::alerts::raise(&format!("tps-{id}"), "tps", Some(id), &format!("Лаги на «{name}»"), &format!("TPS упал до {}", t.to_string().replace('.', ",")));
    }
    hide
}
/// Команда замера TPS для ядра и версии (None — ядро не умеет).
fn tps_probe(s: &Server) -> Option<&'static str> {
    let v: Vec<u32> = s.mc_version.split(['.', '-']).filter_map(|x| x.parse().ok()).collect();
    let at_least = |a: u32, b: u32| v.first().copied().unwrap_or(0) > 1 || (v.first() == Some(&1) && (v.get(1).copied().unwrap_or(0), v.get(2).copied().unwrap_or(0)) >= (a, b));
    match s.loader.as_str() {
        "paper" | "purpur" | "folia" | "spigot" => Some("tps"),
        "forge" => Some("forge tps"),
        "neoforge" => Some("neoforge tps"),
        _ if at_least(20, 3) => Some("tick query"),
        _ => None,
    }
}
/// Учёт игровых сессий для статистики игроков.
fn players_path(id: &str) -> PathBuf { DATA.join("players").join(format!("{id}.json")) }
fn players_db(id: &str) -> Map<String, Value> { std::fs::read_to_string(players_path(id)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default() }
fn session_event(id: &str, player: &str, joined: bool) {
    let now = now_ms();
    let since = rt(id, |r| if joined { r.sessions.insert(player.to_string(), now); None } else { r.sessions.remove(player) });
    let mut db = players_db(id);
    let e = db.entry(player.to_string()).or_insert_with(|| json!({ "first": now, "last": now, "total": 0u64, "sessions": 0u64 }));
    e["last"] = json!(now);
    if joined { e["sessions"] = json!(e["sessions"].as_u64().unwrap_or(0) + 1); }
    if let Some(t) = since { e["total"] = json!(e["total"].as_u64().unwrap_or(0) + now.saturating_sub(t)); }
    let _ = std::fs::create_dir_all(DATA.join("players"));
    let _ = std::fs::write(players_path(id), serde_json::to_string(&db).unwrap_or_default());
}
fn close_sessions(id: &str) {
    let open: Vec<String> = rt(id, |r| r.sessions.keys().cloned().collect());
    for p in open { session_event(id, &p, false); }
}
/// Статистика, которую ведёт сам Minecraft (world/stats/<uuid>.json): время в игре, число выходов, даты файлов.
/// Так видны и игроки, заходившие до появления панели.
fn game_stats(s: &Server) -> Vec<(String, u64, u64, u64, u64)> {
    let root = Path::new(&s.dir);
    let names: HashMap<String, String> = std::fs::read_to_string(root.join("usercache.json")).ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok()).unwrap_or_default().into_iter()
        .filter_map(|e| Some((e["uuid"].as_str()?.to_lowercase(), e["name"].as_str()?.to_string()))).collect();
    let mtime = |p: &Path| p.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64);
    let ctime = |p: &Path| p.metadata().ok().and_then(|m| m.created().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as u64);
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(root) else { return out };
    // только активный мир: в папке сервера могут лежать старые миры с теми же игроками
    let _ = rd;
    for w in [world_dir(s)] {
        let Ok(files) = std::fs::read_dir(w.join("stats")) else { continue };
        for f in files.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")) {
            let uuid = f.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            let Some(name) = names.get(&uuid) else { continue };
            let v: Value = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let c = &v["stats"]["minecraft:custom"];
            // play_time с 1.17, раньше play_one_minute (на деле тоже в тиках)
            let ticks = c["minecraft:play_time"].as_u64().or(c["minecraft:play_one_minute"].as_u64()).unwrap_or(0);
            let leaves = c["minecraft:leave_game"].as_u64().unwrap_or(0);
            let pd = w.join("playerdata").join(format!("{uuid}.dat"));
            let last = mtime(&pd).or(mtime(&f)).unwrap_or(0);
            let first = [ctime(&f), ctime(&pd)].into_iter().flatten().min().unwrap_or(last);
            out.push((name.clone(), ticks * 50, leaves, first, last));
        }
    }
    out
}
/// Статистика игроков сервера: время в игре, число заходов, первый и последний визит.
pub fn player_stats(id: &str) -> Value {
    let now = now_ms();
    let open = rt(id, |r| r.sessions.clone());
    let mut db = players_db(id);
    // объединяем с данными самой игры: берём большее время и число заходов, самые ранние и поздние даты
    if let Ok(s) = get(id) {
        for (name, total, leaves, first, last) in game_stats(&s) {
            let e = db.entry(name).or_insert_with(|| json!({ "first": first, "last": last, "total": 0u64, "sessions": 0u64, "game": true }));
            let cur = |k: &str| e[k].as_u64().unwrap_or(0);
            let (t, se, f, l) = (cur("total").max(total), cur("sessions").max(leaves), cur("first").min(first).max(1), cur("last").max(last));
            e["total"] = json!(t); e["sessions"] = json!(se); e["first"] = json!(f); e["last"] = json!(l);
        }
    }
    let mut list: Vec<Value> = db.into_iter().map(|(name, mut e)| {
        let live = open.get(&name).map(|t| now.saturating_sub(*t)).unwrap_or(0);
        e["total"] = json!(e["total"].as_u64().unwrap_or(0) + live);
        e["online"] = json!(open.contains_key(&name));
        e["name"] = json!(name);
        e
    }).collect();
    list.sort_by_key(|e| std::cmp::Reverse(e["last"].as_u64().unwrap_or(0)));
    json!(list)
}

pub fn push_log(id: &str, line: String) {
    if probe_line(id, &line) { return; }
    let mut became_running = false;
    let mut joined = None;
    rt(id, |r| {
        r.log.push_back(line.clone());
        if r.log.len() > 3000 { r.log.pop_front(); }
        if RE_DONE.is_match(&line) && r.status == "starting" { became_running = true; }
        else if let Some(c) = RE_JOIN.captures(&line) { r.players.insert(c[1].to_string()); joined = Some((c[1].to_string(), true)); }
        else if let Some(c) = RE_LEFT.captures(&line) { r.players.remove(&c[1]); joined = Some((c[1].to_string(), false)); }
        let ws = std::mem::take(&mut r.waiters);
        for (re, tx) in ws { if re.is_match(&line) { let _ = tx.send(line.clone()); } else { r.waiters.push((re, tx)); } }
    });
    emit(json!({ "type": "log", "id": id, "line": line }));
    if became_running { set_status(id, "running"); }
    if let Some((p, j)) = &joined { session_event(id, p, *j); }
    if let Some((p, j)) = joined {
        let players = rt(id, |r| r.players.iter().cloned().collect::<Vec<_>>());
        emit(json!({ "type": "players", "id": id, "name": get(id).map(|s| s.name).unwrap_or_default(), "players": players, "player": p, "joined": j }));
    }
}
/// Ждёт строку лога по регулярному выражению (None — по таймауту).
pub async fn wait_log(id: &str, re: &str, ms: u64) -> Option<String> {
    let (tx, rx) = oneshot::channel();
    rt(id, |r| r.waiters.push((Regex::new(re).unwrap(), tx)));
    tokio::time::timeout(Duration::from_millis(ms), rx).await.ok().and_then(|r| r.ok())
}

// ---------- создание / изменение ----------
pub fn create(o: &Value) -> Res<Value> { create_with(o, None) }
/// Создание сервера; pack — сборка .mrpack и флаг «создать набор для клиента».
pub fn create_with(o: &Value, pack: Option<(crate::modpack::Pack, bool)>) -> Res<Value> {
    let name = core::safe_name(&core::arg_str(o, "name"));
    let dir = match core::arg_str(o, "dir") { d if d.trim().is_empty() => PathBuf::from(core::set_str("serversDir")).join(&name), d => PathBuf::from(d.trim()) };
    let dir = std::path::absolute(&dir).unwrap_or(dir).to_string_lossy().to_string();
    if LIST.lock().iter().any(|s| same_path(&s.dir, &dir)) { crate::bail!("В этой папке уже есть сервер"); }
    if Path::new(&dir).join("server.properties").exists() { crate::bail!("В этой папке уже есть сервер — используйте импорт"); }
    let num = |k: &str, d: u64| o.get(k).and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(d);
    let srv = Server {
        id: core::new_id(), name: name.clone(), mc_version: core::arg_str(o, "mcVersion"),
        loader: match core::arg_str(o, "loader") { l if l.is_empty() => "vanilla".into(), l => l },
        loader_version: core::arg_str(o, "loaderVersion"), dir,
        port: num("port", 25565) as u16, ram: num("ram", core::set_u64("defaultRamMb").max(512)) as u32, jvm_args: String::new(),
        backup_dir: String::new(), max_backups: 0, auto_backup_min: 0, auto_restart: true, auto_start: false,
        installed: false, imported: false, created: now_ms(), java_major: 0, launch: Launch::default(), jvm_preset: String::new(), extra: Map::new(),
    };
    if srv.mc_version.is_empty() { crate::bail!("Не выбрана версия Minecraft"); }
    LIST.lock().push(srv.clone());
    save();
    emit(json!({ "type": "servers" }));
    let job = Job::new(match &pack { Some((p, _)) => format!("Установка сборки «{}» → «{name}»", p.name), None => format!("Установка «{name}» ({} {})", srv.loader, srv.mc_version) });
    let jid = job.id;
    let mut s = srv.clone();
    spawn(async move {
        let mut r = installer::install(&mut s, &job).await;
        if let (Ok(_), Some((p, client))) = (&r, &pack) { r = apply_pack(&s, p, *client, &job).await; }
        let _ = with(&s.id, |x| { x.java_major = s.java_major; x.launch = s.launch.clone(); x.installed = s.installed; });
        save();
        job.finish(&r, "Сервер установлен");
        native::notify_job(&format!("«{}»", s.name), r.as_ref().map(|_| "Сервер установлен и готов к запуску".to_string()));
        emit(json!({ "type": "servers" }));
    });
    Ok(json!({ "srv": view(&srv), "job": jid }))
}
async fn apply_pack(s: &Server, p: &crate::modpack::Pack, client: bool, job: &Job) -> Res<()> {
    let dir = PathBuf::from(&s.dir);
    job.step(Some("Скачивание модов сборки..."), Some(80.0));
    let n = crate::modpack::install_files(p, "server", &dir, false, &|t, pr| job.step(t, pr), 80.0, 94.0).await?;
    let o = crate::modpack::extract_overrides(p, &dir, true).await?;
    job.log(&format!("Модов и файлов: {n}, файлов настроек: {o}"));
    if client {
        job.step(Some("Набор модов для клиента..."), Some(95.0));
        let set = crate::mods::create_set(&json!({ "name": format!("{} (клиент)", p.name), "mcVersion": p.mc, "loader": p.loader, "loaderVersion": p.loader_version }))?;
        let m = crate::modpack::install_files(p, "client", &crate::mods::set_dir(&set.id), true, &|t, pr| job.step(t, pr), 95.0, 99.0).await?;
        job.log(&format!("Набор для клиента: модов {m}"));
        emit(json!({ "type": "sets" }));
    }
    Ok(())
}
/// Смена версии Minecraft/загрузчика: бэкап мира, затем переустановка ядра.
pub fn change_version(id: &str, mc: &str, lv: &str) -> Res<u64> {
    if is_running(id) { crate::bail!("Сначала остановите сервер"); }
    let mut s = get(id)?;
    if mc.is_empty() { crate::bail!("Не выбрана версия"); }
    if s.loader != "vanilla" && lv.is_empty() { crate::bail!("Не выбрана версия загрузчика"); }
    let job = Job::new(format!("Смена версии «{}»: {} → {mc}", s.name, s.mc_version));
    let jid = job.id;
    let (mc, lv) = (mc.to_string(), lv.to_string());
    spawn(async move {
        let r: Res<()> = async {
            job.step(Some("Бэкап мира перед обновлением..."), Some(2.0));
            let s2 = s.clone();
            let j2 = job.clone();
            let note = format!("before-{}", s.mc_version);
            let b = tauri::async_runtime::spawn_blocking(move || crate::backups::make_zip(&s2, &note, &j2)).await.map_err(|e| e.to_string())?;
            match b { Ok(f) => job.log(&format!("Бэкап: {f}")), Err(e) if e.contains("не найдена") => job.log("Мира ещё нет — бэкап не нужен"), Err(e) => return Err(e) }
            s.mc_version = mc.clone();
            s.loader_version = lv.clone();
            installer::install(&mut s, &job).await?;
            with(&s.id, |x| { x.mc_version = s.mc_version.clone(); x.loader_version = s.loader_version.clone(); x.java_major = s.java_major; x.launch = s.launch.clone(); x.installed = true; })?;
            Ok(())
        }.await;
        save();
        job.finish(&r, "Версия изменена");
        emit(json!({ "type": "servers" }));
    });
    Ok(jid)
}
pub fn same_path(a: &str, b: &str) -> bool {
    let n = |p: &str| p.trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase();
    n(a) == n(b)
}
/// Тесты: сервер только в памяти, без записи servers.json.
#[cfg(test)]
pub fn add_mem(s: Server) { LIST.lock().push(s); }
/// Добавить готовый сервер (клон) в список.
pub fn add(s: Server) -> Res<String> {
    let id = s.id.clone();
    LIST.lock().push(s);
    save();
    Ok(id)
}
pub fn add_imported(mut s: Server) -> Server {
    s.id = core::new_id();
    s.installed = true; s.imported = true; s.created = now_ms(); s.auto_restart = true;
    LIST.lock().push(s.clone());
    save();
    emit(json!({ "type": "servers" }));
    s
}
pub fn reinstall(id: &str) -> Res<u64> {
    if is_running(id) { crate::bail!("Сначала остановите сервер"); }
    let mut s = get(id)?;
    let job = Job::new(format!("Переустановка «{}»", s.name));
    let jid = job.id;
    spawn(async move {
        let r = installer::install(&mut s, &job).await;
        let _ = with(&s.id, |x| { x.java_major = s.java_major; x.launch = s.launch.clone(); x.installed = s.installed; });
        save();
        job.finish(&r, "Ядро переустановлено");
        emit(json!({ "type": "servers" }));
    });
    Ok(jid)
}
pub fn update(id: &str, p: &Value) -> Res<Value> {
    let num = |k: &str| p.get(k).and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())));
    let s = with(id, |s| {
        if let Some(v) = p.get("name").and_then(|v| v.as_str()) { if !v.trim().is_empty() { s.name = core::safe_name(v); } }
        if let Some(v) = num("port") { s.port = v as u16; }
        if let Some(v) = num("ram") { s.ram = (v as u32).max(512); }
        if let Some(v) = p.get("jvmArgs").and_then(|v| v.as_str()) { s.jvm_args = v.trim().into(); }
        if let Some(v) = p.get("backupDir").and_then(|v| v.as_str()) { s.backup_dir = v.trim().into(); }
        if let Some(v) = num("maxBackups") { s.max_backups = v as u32; } else if p.get("maxBackups").is_some() { s.max_backups = 0; }
        if let Some(v) = num("autoBackupMin") { s.auto_backup_min = v as u32; } else if p.get("autoBackupMin").is_some() { s.auto_backup_min = 0; }
        if let Some(v) = p.get("autoRestart").and_then(|v| v.as_bool()) { s.auto_restart = v; }
        if let Some(v) = p.get("autoStart").and_then(|v| v.as_bool()) { s.auto_start = v; }
        if let Some(v) = p.get("loaderVersion").and_then(|v| v.as_str()) { s.loader_version = v.into(); }
        if let Some(v) = p.get("jvmPreset").and_then(|v| v.as_str()) { s.jvm_preset = v.into(); }
        s.clone()
    })?;
    save();
    schedule_auto_backup(&s);
    emit(json!({ "type": "servers" }));
    Ok(view(&s))
}
pub async fn remove(id: &str, delete_files: bool) -> Res<()> {
    if is_running(id) { crate::bail!("Сначала остановите сервер"); }
    let s = get(id)?;
    LIST.lock().retain(|x| x.id != id);
    save();
    if let Some(t) = TIMERS.lock().remove(id) { t.abort(); }
    RT.lock().remove(id);
    if delete_files { tokio::fs::remove_dir_all(&s.dir).await.ctx("Удаление файлов")?; }
    emit(json!({ "type": "servers" }));
    Ok(())
}

// ---------- процессы ----------
pub fn start(id: String) -> BoxFuture<'static, Res<()>> {
    Box::pin(async move {
        let s = get(&id)?;
        if is_running(&id) { crate::bail!("Сервер уже запущен"); }
        if !s.installed { crate::bail!("Сервер не установлен"); }
        let (java, args) = match installer::launch_command(&s) {
            Ok(c) => c,
            Err(e) => {
                // импортированный сервер: нужной Java ещё нет — скачиваем и запускаем
                if !e.contains("Java") || rt(&id, |r| std::mem::replace(&mut r.fetching_java, true)) { return Err(e); }
                let job = Job::new(format!("Java {} для «{}»", installer::java_major_norm(s.java_major.max(8)), s.name));
                let id2 = id.clone();
                spawn(async move {
                    let r = installer::ensure_java(if s.java_major == 0 { 21 } else { s.java_major }, Some(&job)).await.map(|_| ());
                    rt(&id2, |r| r.fetching_java = false);
                    job.finish(&r, "Java установлена");
                    if r.is_ok() { if let Err(e) = start(id2.clone()).await { push_log(&id2, format!("[Панель] {e}")); } }
                });
                return Ok(());
            }
        };
        push_log(&id, format!("[Панель] Запуск: {} {}", java.file_name().unwrap_or_default().to_string_lossy(), args.join(" ")));
        let mut cmd = core::command(&java);
        cmd.args(&args).current_dir(&s.dir)
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => { push_log(&id, format!("[Панель] Ошибка запуска: {e}")); crate::bail!("Ошибка запуска: {e}"); }
        };
        let (kill_tx, mut kill_rx) = mpsc::unbounded_channel::<()>();
        let (exit_tx, exit_rx) = watch::channel(false);
        let stdin = child.stdin.take().map(|s| Arc::new(tokio::sync::Mutex::new(s)));
        let started = now_ms();
        rt(&id, |r| {
            r.pid = child.id(); r.stdin = stdin; r.kill_tx = Some(kill_tx); r.exit_rx = Some(exit_rx);
            r.players.clear(); r.sessions.clear(); r.started = Some(started); r.stopping = false; r.history.clear(); r.tps = None; r.low_tps = 0;
        });
        set_status(&id, "starting");
        let mut readers = vec![];
        for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>), child.stderr.take().map(|s| Box::new(s) as _)].into_iter().flatten() {
            let id = id.clone();
            readers.push(spawn(async move {
                let mut rd = BufReader::new(stream);
                let mut buf = Vec::new();
                loop {
                    buf.clear();
                    match rd.read_until(b'\n', &mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let line = String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string();
                            push_log(&id, line);
                        }
                    }
                }
            }));
        }
        let auto_restart = s.auto_restart;
        let name = s.name.clone();
        spawn(async move {
            let st = tokio::select! {
                r = child.wait() => r,
                Some(_) = kill_rx.recv() => { let _ = child.kill().await; child.wait().await }
            };
            for r in readers { let _ = r.await; }
            close_sessions(&id);
            let code = st.ok().and_then(|s| s.code()).unwrap_or(-1);
            let stopping = rt(&id, |r| {
                r.pid = None; r.stdin = None; r.kill_tx = None; r.exit_rx = None; r.stats = None; r.players.clear(); r.tps = None;
                r.stopping
            });
            set_status(&id, "stopped");
            push_log(&id, format!("[Панель] Процесс завершён (код {code})"));
            let _ = exit_tx.send(true);
            let crashed = !stopping && code != 0;
            if crashed {
                let diag = crate::diagnose::diagnose(&id, started).unwrap_or(json!({}));
                emit(json!({ "type": "crash", "id": id, "name": name, "code": code, "diag": diag }));
                let restart = auto_restart && now_ms() - started > 60_000 && get(&id).map(|s| s.auto_restart).unwrap_or(false);
                native::notify("Сбой сервера", &format!("«{name}» аварийно завершился (код {code}).{}", if restart { " Автоперезапуск через 5 с." } else { "" }));
                if restart {
                    push_log(&id, "[Панель] Сбой — автоперезапуск через 5 с".into());
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if let Err(e) = start(id.clone()).await { push_log(&id, format!("[Панель] {e}")); }
                }
            }
        });
        Ok(())
    })
}

/// Команда без записи в консоль (служебные замеры).
async fn command_silent(id: &str, cmd: &str) -> Res<()> {
    let stdin = rt(id, |r| r.stdin.clone()).ok_or("Сервер не запущен")?;
    let mut s = stdin.lock().await;
    s.write_all(format!("{cmd}\n").as_bytes()).await.ctx("Отправка команды")?;
    s.flush().await.ctx("Отправка команды")
}
pub async fn command(id: &str, cmd: &str) -> Res<()> {
    let stdin = rt(id, |r| r.stdin.clone()).ok_or("Сервер не запущен")?;
    let cmd = cmd.trim();
    let cmd = cmd.strip_prefix('/').unwrap_or(cmd);
    push_log(id, format!("> {cmd}"));
    let mut s = stdin.lock().await;
    s.write_all(format!("{cmd}\n").as_bytes()).await.ctx("Отправка команды")?;
    s.flush().await.ctx("Отправка команды")
}
pub async fn stop(id: &str) -> Res<()> {
    let Some(mut exit) = rt(id, |r| { if r.pid.is_some() { r.stopping = true; r.exit_rx.clone() } else { None } }) else { return Ok(()) };
    set_status(id, "stopping");
    let _ = command(id, "stop").await;
    if tokio::time::timeout(Duration::from_secs(60), exit.wait_for(|v| *v)).await.is_err() {
        push_log(id, "[Панель] Сервер не остановился за 60 с — принудительное завершение".into());
        kill(id);
        let _ = tokio::time::timeout(Duration::from_secs(10), exit.wait_for(|v| *v)).await;
    }
    Ok(())
}
pub fn kill(id: &str) { rt(id, |r| { if let Some(k) = &r.kill_tx { r.stopping = true; let _ = k.send(()); } }); }
pub async fn restart(id: &str) -> Res<()> { stop(id).await?; start(id.to_string()).await }
pub fn running_ids() -> Vec<String> { RT.lock().iter().filter(|(_, r)| r.pid.is_some()).map(|(k, _)| k.clone()).collect() }
pub async fn stop_all() { futures_util::future::join_all(running_ids().iter().map(|id| stop(id))).await; }

/// Мониторинг: CPU/RAM процессов серверов и системы раз в 2 секунды.
pub fn start_monitor() {
    spawn(async {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
        let mut sys = System::new();
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f32;
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        let mut n: u64 = 0;
        let mut disks = sysinfo::Disks::new_with_refreshed_list();
        loop {
            tick.tick().await;
            n += 1;
            if n % 15 == 0 {
                let probes: Vec<(String, &'static str)> = LIST.lock().iter().filter(|s| rt(&s.id, |r| r.status == "running")).filter_map(|s| tps_probe(s).map(|c| (s.id.clone(), c))).collect();
                for (id, cmd) in probes {
                    rt(&id, |r| r.probe_until = now_ms() + 4000);
                    spawn(async move { let _ = command_silent(&id, cmd).await; });
                }
            }
            if n % 30 == 0 {
                disks.refresh(true);
                let dir = core::set_str("serversDir");
                if let Some(d) = disks.list().iter().filter(|d| dir.to_lowercase().starts_with(&d.mount_point().to_string_lossy().to_lowercase())).max_by_key(|d| d.mount_point().as_os_str().len()) {
                    let free = d.available_space();
                    if free < 5 * 1_073_741_824 { crate::alerts::raise("disk", "disk", None, "Мало места на диске", &format!("Свободно {:.1} ГБ на {} — бэкапы и миры могут не поместиться", free as f64 / 1_073_741_824.0, d.mount_point().to_string_lossy())); }
                }
            }
            let pids: Vec<(String, u32)> = RT.lock().iter().filter_map(|(k, r)| r.pid.map(|p| (k.clone(), p))).collect();
            let list: Vec<Pid> = pids.iter().map(|(_, p)| Pid::from_u32(*p)).collect();
            sys.refresh_processes_specifics(ProcessesToUpdate::Some(&list), true, ProcessRefreshKind::nothing().with_cpu().with_memory());
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            let now = now_ms();
            for (id, pid) in &pids {
                let st = sys.process(Pid::from_u32(*pid)).map(|p| Stats { cpu: ((p.cpu_usage() / cores) as f64 * 10.0).round() / 10.0, mem: p.memory() });
                let (players, status) = rt(id, |r| {
                    r.stats = st;
                    if let Some(s) = st { r.history.push_back((now, s.cpu, s.mem / 1_048_576, r.players.len())); if r.history.len() > 180 { r.history.pop_front(); } }
                    (r.players.iter().cloned().collect::<Vec<_>>(), r.status)
                });
                let tps = rt(id, |r| r.tps);
                emit(json!({ "type": "stats", "id": id, "stats": st, "players": players, "status": status, "t": now, "tps": tps }));
            }
            emit(json!({ "type": "sys", "cpu": (sys.global_cpu_usage() as f64 * 10.0).round() / 10.0, "memUsed": sys.used_memory(), "memTotal": sys.total_memory() }));
            if sys.total_memory() > 0 && sys.used_memory() as f64 / sys.total_memory() as f64 > 0.93 && !pids.is_empty() {
                crate::alerts::raise("mem", "mem", None, "Мало памяти на ПК", &format!("Занято {:.1} из {:.0} ГБ — серверы могут начать тормозить или упасть", sys.used_memory() as f64 / 1_073_741_824.0, sys.total_memory() as f64 / 1_073_741_824.0));
            }
        }
    });
}
pub fn system_info() -> Value {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    json!({
        "memTotal": sys.total_memory(),
        "cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        "version": env!("CARGO_PKG_VERSION"),
        "dataDir": crate::core::DATA.to_string_lossy(),
        "mica": native::mica_supported(),
        "debug": cfg!(debug_assertions),
        "portable": crate::core::PORTABLE.is_some(),
    })
}

// ---------- server.properties ----------
fn props_path(s: &Server) -> PathBuf { Path::new(&s.dir).join("server.properties") }
pub fn parse_props(txt: &str) -> Vec<(String, String)> {
    txt.lines().filter(|l| !l.trim_start().starts_with('#') && l.contains('='))
        .map(|l| { let i = l.find('=').unwrap(); (l[..i].trim().to_string(), l[i + 1..].to_string()) }).collect()
}
pub fn read_props(id: &str) -> Res<Value> {
    let s = get(id)?;
    let txt = std::fs::read_to_string(props_path(&s)).unwrap_or_default();
    let m: Map<String, Value> = parse_props(&txt).into_iter().map(|(k, v)| (k, Value::String(v))).collect();
    Ok(Value::Object(m))
}
/// Обновляет значения на месте, сохраняя порядок строк и комментарии.
pub fn write_props(id: &str, obj: &Value) -> Res<()> {
    let s = get(id)?;
    let Value::Object(o) = obj else { crate::bail!("Неверные данные") };
    let path = props_path(&s);
    let txt = std::fs::read_to_string(&path).unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = txt.lines().map(|l| {
        if l.trim_start().starts_with('#') || !l.contains('=') { return l.to_string(); }
        let k = l[..l.find('=').unwrap()].trim().to_string();
        match o.get(&k) { Some(v) => { seen.insert(k.clone()); format!("{k}={}", core::arg_str(&json!({ "v": v }), "v")) } None => l.to_string() }
    }).collect();
    for (k, v) in o { if !seen.contains(k) { out.push(format!("{k}={}", core::arg_str(&json!({ "v": v }), "v"))); } }
    std::fs::write(&path, out.join("\n") + "\n").ctx("Запись server.properties")?;
    if let Some(p) = o.get("server-port").and_then(|v| v.as_str()).and_then(|v| v.trim().parse::<u16>().ok()) {
        with(id, |s| s.port = p)?;
        save();
        emit(json!({ "type": "servers" }));
    }
    Ok(())
}
pub fn lists(id: &str) -> Res<Value> {
    let s = get(id)?;
    let mut m = Map::new();
    for f in ["ops", "whitelist", "banned-players", "banned-ips"] {
        m.insert(f.into(), core::read_json(&Path::new(&s.dir).join(format!("{f}.json"))).unwrap_or(json!([])));
    }
    Ok(Value::Object(m))
}

// ---------- автобэкапы и автозапуск ----------
static TIMERS: Lazy<Mutex<HashMap<String, JoinHandle<()>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub fn schedule_auto_backup(s: &Server) {
    if let Some(t) = TIMERS.lock().remove(&s.id) { t.abort(); }
    if s.auto_backup_min == 0 { return; }
    let id = s.id.clone();
    let every = Duration::from_secs(s.auto_backup_min as u64 * 60);
    let h = spawn(async move {
        let mut t = tokio::time::interval_at(tokio::time::Instant::now() + every, every);
        loop { t.tick().await; if status(&id) == "running" { let _ = crate::backups::backup(&id, "auto"); } }
    });
    TIMERS.lock().insert(s.id.clone(), h);
}
pub fn init() {
    for s in all_raw() { schedule_auto_backup(&s); }
    start_monitor();
    // серверы с флагом «Запускать при старте панели»
    let auto: Vec<String> = all_raw().into_iter().filter(|s| s.auto_start && s.installed).map(|s| s.id).collect();
    spawn(async move {
        for (i, id) in auto.into_iter().enumerate() {
            tokio::time::sleep(Duration::from_millis(if i == 0 { 1500 } else { 3000 })).await;
            if let Err(e) = start(id.clone()).await { push_log(&id, format!("[Панель] Автозапуск: {e}")); }
        }
    });
}
