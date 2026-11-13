//! Модульные тесты чистой логики (без сети и процессов).
use crate::{diagnose, files, installer, modpack, playerdata, providers, servers, tools};

#[test]
fn neoforge_version_to_minecraft() {
    assert_eq!(installer::neo_to_mc("21.1.77"), "1.21.1");
    assert_eq!(installer::neo_to_mc("20.4.237"), "1.20.4");
    assert_eq!(installer::neo_to_mc("21.0.167"), "1.21");
    assert_eq!(installer::neo_to_mc("26.1.0.5-beta"), "26.1");
    assert_eq!(installer::neo_to_mc("26.1.2.3"), "26.1.2");
}

#[test]
fn jvm_presets() {
    let a = installer::jvm_preset("aikar", 4096, 21);
    assert!(a.contains(&"-XX:+UseG1GC".to_string()));
    assert!(a.contains(&"-XX:G1HeapRegionSize=8M".to_string()));
    assert!(installer::jvm_preset("aikar", 16384, 21).contains(&"-XX:G1HeapRegionSize=16M".to_string()));
    assert!(installer::jvm_preset("zgc", 8192, 17).is_empty(), "ZGC только с Java 21+");
    assert!(installer::jvm_preset("zgc", 8192, 21).contains(&"-XX:+ZGenerational".to_string()));
    assert!(!installer::jvm_preset("zgc", 8192, 25).contains(&"-XX:+ZGenerational".to_string()));
    assert!(installer::jvm_preset("", 8192, 21).is_empty());
}

#[test]
fn curseforge_fingerprint() {
    // эталон — независимая реализация MurmurHash2 (seed 1, без пробельных символов)
    assert_eq!(providers::murmur2_cf(b"hello world"), 2824650221);
    assert_eq!(providers::murmur2_cf(b"hello\tworld\n"), 2824650221);
    assert_eq!(providers::murmur2_cf(b""), 1540447798);
    assert_eq!(providers::murmur2_cf(b"abc"), 1621425345);
}

#[test]
fn server_properties_parsing() {
    let p = servers::parse_props("#comment\nmotd=Привет=мир\nserver-port=25565\n\nbroken line\n");
    assert_eq!(p, vec![("motd".to_string(), "Привет=мир".to_string()), ("server-port".to_string(), "25565".to_string())]);
}

#[test]
fn modpack_paths_are_contained() {
    assert!(modpack::safe_rel("mods/sodium.jar").is_some());
    assert!(modpack::safe_rel("config\\a.toml").is_some());
    assert!(modpack::safe_rel("../evil.jar").is_none());
    assert!(modpack::safe_rel("mods/../../evil.jar").is_none());
    assert!(modpack::safe_rel("/etc/passwd").is_none());
    assert!(modpack::safe_rel("C:/Windows/x.dll").is_none());
    assert!(modpack::safe_rel("").is_none());
}

#[test]
fn description_to_plain_text() {
    let t = providers::plain_text("## Title\n**Bold** [link](http://x) ![img](http://y.png)\n---\n<p>Hi&amp;bye</p>");
    assert_eq!(t, "Title\nBold link \n\nHi&bye");
}

#[test]
fn editable_files() {
    assert!(files::editable("server.properties"));
    assert!(files::editable("config.TOML"));
    assert!(files::editable("ops.json"));
    assert!(!files::editable("server.jar"));
    assert!(!files::editable("level.dat"));
}

#[test]
fn crash_hints() {
    let l = |s: &str| s.lines().map(String::from).collect::<Vec<_>>();
    let h = diagnose::hints(&l("java.lang.OutOfMemoryError: Java heap space"), "", 2048);
    assert_eq!(h[0].title, "Не хватает памяти");
    assert!(h[0].text.contains("2048"));
    let h = diagnose::hints(&l("**** FAILED TO BIND TO PORT!"), "", 4096);
    assert_eq!(h[0].title, "Порт занят");
    let h = diagnose::hints(&l("java.lang.UnsupportedClassVersionError: has been compiled by a more recent version"), "", 4096);
    assert_eq!(h[0].title, "Неподходящая версия Java");
    let h = diagnose::hints(&[], "Description: Exception in server tick loop\n\tSuspected Mods: Create (create)", 4096);
    assert!(h.iter().any(|x| x.title == "Подозреваемые моды" && x.text.contains("Create")));
    let h = diagnose::hints(&l("Mixin apply for mod badmod failed"), "", 4096);
    assert!(h.iter().any(|x| x.title == "Конфликт модов" && x.text.contains("badmod")));
    let h = diagnose::hints(&l("Unrecognized VM option 'ThisFlagDoesNotExist'\nError: Could not create the Java Virtual Machine."), "", 4096);
    assert!(h[0].title == "Неверные аргументы JVM" && h[0].text.contains("ThisFlagDoesNotExist"));
    assert!(diagnose::hints(&l("[Server thread/INFO]: Done (3.2s)!"), "", 4096).is_empty());
}

#[test]
fn tps_from_server_output() {
    // Paper/Purpur: «tps» (с цветовыми кодами)
    assert_eq!(servers::parse_tps("[12:00:00 INFO]: §6TPS from last 1m, 5m, 15m: §a*20.0, §a19.97, §a19.99"), Some(20.0));
    assert_eq!(servers::parse_tps("[12:00:00 INFO]: TPS from last 1m, 5m, 15m: 14.21, 18.0, 19.5"), Some(14.21));
    // Vanilla/Fabric 1.20.3+: «tick query» — среднее время тика в мс
    assert_eq!(servers::parse_tps("[Server thread/INFO]: Average time per tick: 3.4ms (Target: 50.0ms)"), Some(20.0));
    assert_eq!(servers::parse_tps("[Server thread/INFO]: Average time per tick: 100.0ms (Target: 50.0ms)"), Some(10.0));
    // Forge/NeoForge: «forge tps»
    assert_eq!(servers::parse_tps("[Server thread/INFO]: Overall: Mean tick time: 4.123 ms. Mean TPS: 20.000"), Some(20.0));
    assert_eq!(servers::parse_tps("[Server thread/INFO]: Steve joined the game"), None);
}

#[test]
fn offline_player_uuid() {
    // так сервер с online-mode=false вычисляет UUID игрока по нику
    assert_eq!(tools::offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
}

#[tokio::test]
async fn transfer_player_between_nicks() {
    let dir = std::env::temp_dir().join(format!("mcpanel-transfer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (old, new) = (tools::offline_uuid("OldNick"), tools::offline_uuid("NewNick"));
    let w = dir.join("world");
    for d in ["playerdata", "stats", "advancements", "somemod"] { std::fs::create_dir_all(w.join(d)).unwrap(); }
    std::fs::write(w.join("level.dat"), b"x").unwrap();
    std::fs::write(w.join(format!("playerdata/{old}.dat")), b"old-inventory").unwrap();
    std::fs::write(w.join(format!("playerdata/{new}.dat")), b"new-empty").unwrap();
    std::fs::write(w.join(format!("stats/{old}.json")), b"{}").unwrap();
    std::fs::write(w.join(format!("advancements/{old}.json")), b"{}").unwrap();
    std::fs::write(w.join(format!("somemod/{old}.dat")), b"mod").unwrap();
    std::fs::write(dir.join("server.properties"), "online-mode=false\n").unwrap();
    std::fs::write(dir.join("ops.json"), format!(r#"[{{"uuid":"{old}","name":"OldNick","level":4}}]"#)).unwrap();
    let s: servers::Server = serde_json::from_value(serde_json::json!({ "id": "tt1", "name": "T", "dir": dir.to_string_lossy() })).unwrap();
    servers::add_mem(s);
    let r = tools::transfer_player("tt1", "OldNick", "NewNick").await.unwrap();
    assert_eq!(r["files"].as_array().unwrap().len(), 4);
    assert_eq!(std::fs::read(w.join(format!("playerdata/{new}.dat"))).unwrap(), b"old-inventory");
    assert!(w.join(format!("somemod/{new}.dat")).exists());
    // данные нового ника сохранены в резервную папку
    let bak = std::fs::read_dir(&w).unwrap().flatten().find(|e| e.file_name().to_string_lossy().starts_with("player-transfer-backup-")).unwrap();
    assert_eq!(std::fs::read(bak.path().join(format!("playerdata/{new}.dat"))).unwrap(), b"new-empty");
    assert!(std::fs::read_to_string(dir.join("ops.json")).unwrap().contains("NewNick"));
    assert!(tools::transfer_player("tt1", "Nobody", "NewNick").await.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn edit_player_inventory() {
    use fastnbt::Value as N;
    use std::collections::HashMap;
    let c = |v: Vec<(&str, N)>| N::Compound(v.into_iter().map(|(k, v)| (k.to_string(), v)).collect::<HashMap<_, _>>());
    let dir = std::env::temp_dir().join(format!("mcpanel-inv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let uuid = tools::offline_uuid("Steve");
    std::fs::create_dir_all(dir.join("world/playerdata")).unwrap();
    std::fs::write(dir.join("world/level.dat"), b"x").unwrap();
    std::fs::write(dir.join("server.properties"), "online-mode=false\n").unwrap();
    let sword = c(vec![("Slot", N::Byte(0)), ("id", N::String("minecraft:diamond_sword".into())), ("count", N::Int(1)),
        ("components", c(vec![("minecraft:enchantments", c(vec![("levels", c(vec![("minecraft:sharpness", N::Int(5))]))]))]))]);
    let stone = c(vec![("Slot", N::Byte(1)), ("id", N::String("minecraft:stone".into())), ("count", N::Int(64))]);
    let helmet = c(vec![("Slot", N::Byte(103)), ("id", N::String("minecraft:diamond_helmet".into())), ("count", N::Int(1))]);
    let player = c(vec![("DataVersion", N::Int(4189)), ("Inventory", N::List(vec![sword, stone, helmet])), ("EnderItems", N::List(vec![])),
        ("Health", N::Float(20.0)), ("foodLevel", N::Int(20)), ("XpLevel", N::Int(5)), ("playerGameType", N::Int(0))]);
    let p = dir.join(format!("world/playerdata/{uuid}.dat"));
    playerdata::store(&p, &player).unwrap();
    let s: servers::Server = serde_json::from_value(serde_json::json!({ "id": "inv1", "name": "I", "dir": dir.to_string_lossy() })).unwrap();
    servers::add_mem(s);

    let d = playerdata::get("inv1", "Steve").await.unwrap();
    assert_eq!(d["inventory"].as_array().unwrap().len(), 2);
    assert_eq!(d["equipment"]["head"]["id"], "minecraft:diamond_helmet");
    assert_eq!(d["inventory"][0]["enchanted"], true);

    let patch = serde_json::json!({ "xpLevel": 30, "gameMode": 1,
        "inventory": [{ "slot": 0, "id": "minecraft:diamond_sword", "count": 1 }, { "slot": 1, "id": "minecraft:stone", "count": 32, "dirty": true, "name": "Камень", "lore": ["строка"], "enchants": [{ "id": "sharpness", "lvl": 3 }], "unbreakable": true }, { "slot": 2, "id": "apple", "count": 5 }],
        "equipment": { "head": { "id": "minecraft:diamond_helmet", "count": 1 } }, "ender": [{ "slot": 0, "id": "minecraft:diamond", "count": 3 }] });
    let d = playerdata::save("inv1", "Steve", &patch).await.unwrap();
    assert_eq!(d["xpLevel"], 30.0);
    assert_eq!(d["gameMode"], 1.0);
    let inv = d["inventory"].as_array().unwrap();
    assert!(inv.iter().any(|i| i["id"] == "minecraft:stone" && i["count"] == 32));
    assert!(inv.iter().any(|i| i["id"] == "minecraft:apple" && i["count"] == 5));
    assert!(inv.iter().any(|i| i["id"] == "minecraft:diamond_sword" && i["enchanted"] == true), "зачарования сохраняются");
    let st = inv.iter().find(|i| i["id"] == "minecraft:stone").unwrap();
    assert_eq!(st["name"], "Камень");
    assert_eq!(st["lore"][0], "строка");
    assert_eq!(st["enchants"][0]["id"], "minecraft:sharpness");
    assert_eq!(st["enchants"][0]["lvl"], 3);
    assert_eq!(st["unbreakable"], true);
    let sw = inv.iter().find(|i| i["id"] == "minecraft:diamond_sword").unwrap();
    assert_eq!(sw["enchants"][0]["lvl"], 5, "у меча без правок уровень 5 остался");
    assert_eq!(d["equipment"]["head"]["id"], "minecraft:diamond_helmet");
    assert_eq!(d["ender"][0]["count"], 3);
    assert!(p.with_extension("dat.mcpanel-bak").exists());

    // перетаскивание: меч → эндер-сундук, шлем → инвентарь; свойства берутся с исходного места
    let d = playerdata::get("inv1", "Steve").await.unwrap();
    let mut inv: Vec<serde_json::Value> = d["inventory"].as_array().unwrap().clone();
    let mut sword = inv.iter().find(|i| i["id"] == "minecraft:diamond_sword").unwrap().clone();
    inv.retain(|i| i["id"] != "minecraft:diamond_sword");
    let mut helmet = d["equipment"]["head"].clone();
    helmet["slot"] = serde_json::json!(10); inv.push(helmet);
    sword["slot"] = serde_json::json!(5);
    let mut ender = d["ender"].as_array().unwrap().clone(); ender.push(sword);
    let d = playerdata::save("inv1", "Steve", &serde_json::json!({ "inventory": inv, "ender": ender, "equipment": {} })).await.unwrap();
    let sw = d["ender"].as_array().unwrap().iter().find(|i| i["id"] == "minecraft:diamond_sword").expect("меч в эндер-сундуке");
    assert_eq!(sw["slot"], 5);
    assert_eq!(sw["enchants"][0]["lvl"], 5, "зачарования переехали вместе с мечом");
    assert!(d["inventory"].as_array().unwrap().iter().any(|i| i["id"] == "minecraft:diamond_helmet" && i["slot"] == 10));
    assert!(d["equipment"]["head"].is_null());
    assert_eq!(d["inventory"].as_array().unwrap().iter().find(|i| i["id"] == "minecraft:stone").unwrap()["name"], "Камень", "имя камня без правок сохранилось");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Ручная проверка на настоящем файле (только чтение): MCPANEL_TEST_DAT=путь cargo test real_player_dat -- --nocapture
#[test]
fn real_player_dat() {
    let Ok(p) = std::env::var("MCPANEL_TEST_DAT") else { return };
    let v = playerdata::load(std::path::Path::new(&p)).unwrap();
    if let fastnbt::Value::Compound(m) = &v {
        let inv = match m.get("Inventory") { Some(fastnbt::Value::List(l)) => l.len(), _ => 0 };
        println!("DataVersion={:?} items={inv} equipment={}", m.get("DataVersion"), m.contains_key("equipment"));
        assert!(fastnbt::to_bytes(&v).is_ok(), "файл должен записываться обратно");
    } else { panic!("не compound") }
}

/// Ручная проверка на настоящем сервере (только чтение): MCPANEL_TEST_SERVER=папка cargo test real_player_stats -- --nocapture
#[test]
fn real_player_stats() {
    let Ok(dir) = std::env::var("MCPANEL_TEST_SERVER") else { return };
    let s: servers::Server = serde_json::from_value(serde_json::json!({ "id": "real1", "name": "R", "dir": dir })).unwrap();
    servers::add_mem(s);
    for p in servers::player_stats("real1").as_array().unwrap() {
        println!("{} · {} мин · заходов {} · first {} · last {}", p["name"], p["total"].as_u64().unwrap() / 60000, p["sessions"], p["first"], p["last"]);
    }
}

/// Ручная проверка имён и зачарований на настоящем сервере (только чтение):
/// MCPANEL_TEST_SERVER=папка MCPANEL_TEST_PLAYER=ник cargo test real_item_names -- --nocapture
#[tokio::test]
async fn real_item_names() {
    let (Ok(dir), Ok(name)) = (std::env::var("MCPANEL_TEST_SERVER"), std::env::var("MCPANEL_TEST_PLAYER")) else { return };
    let s: servers::Server = serde_json::from_value(serde_json::json!({ "id": "realn", "name": "R", "dir": dir })).unwrap();
    servers::add_mem(s);
    let d = playerdata::get("realn", &name).await.unwrap();
    let all: Vec<serde_json::Value> = ["inventory", "ender"].iter().flat_map(|k| d[*k].as_array().cloned().unwrap_or_default())
        .chain(d["equipment"].as_object().map(|o| o.values().cloned().collect::<Vec<_>>()).unwrap_or_default()).collect();
    for it in all.iter().filter(|i| !i["name"].is_null() || i["enchanted"] == true || !i["lore"].as_array().unwrap().is_empty()) {
        println!("{} · имя={} · зачар={} · lore={}", it["id"], it["name"], it["enchants"], it["lore"]);
    }
    println!("всего предметов: {}", all.len());
    if let Ok(out) = std::env::var("MCPANEL_TEST_DUMP") { std::fs::write(out, serde_json::to_string(&d).unwrap()).unwrap(); }
}
