//! Модульные тесты чистой логики (без сети и процессов).
use crate::{diagnose, files, installer, modpack, providers, servers};

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
