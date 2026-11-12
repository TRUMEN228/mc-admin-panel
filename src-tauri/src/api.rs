//! Единая точка входа: команды UI (через IPC Tauri или веб-режим).
use crate::core::{self, arg_str, Res};
use crate::{backups, diagnose, files, importer, installer, modpack, mods, network, providers, servers};
use serde_json::{json, Value};

fn strs(a: &Value, k: &str) -> Vec<String> {
    a.get(k).and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}
fn ok() -> Res<Value> { Ok(json!({ "ok": true })) }

pub async fn dispatch(cmd: &str, a: Value, web: bool) -> Res<Value> {
    let id = arg_str(&a, "id");
    let s = |k: &str| arg_str(&a, k);
    match cmd {
        // ---- общее ----
        "settings.get" => Ok(core::settings()),
        "settings.save" => core::save_settings(a),
        "system.info" => { let mut v = servers::system_info(); v["web"] = json!(web); Ok(v) }
        "jobs" => Ok(json!(core::jobs())),
        "mobile.pairStart" => { if web { crate::bail!("Привязка телефона — только в самой панели"); } crate::mobile::pair_start() }
        "mobile.pairCancel" => { crate::mobile::pair_cancel(); ok() }
        "mobile.devices" => Ok(crate::mobile::devices()),
        "mobile.revoke" => { if web { crate::bail!("Только в самой панели"); } crate::mobile::revoke(&id)?; ok() }
        "mobile.addresses" => Ok(json!(crate::mobile::addresses())),
        "open" => { if web { crate::bail!("Недоступно в веб-режиме"); } core::open_path(&s("path"))?; ok() }
        "versions" => installer::mc_versions(a["snapshots"].as_bool().unwrap_or(false)).await,
        "loaderVersions" => installer::loader_versions(&s("loader"), &s("mc")).await,
        "catalog.providers" => Ok(providers::list(&s("loader"))),
        "catalog.search" | "modrinth.search" => providers::search(&match s("provider") { p if p.is_empty() => "modrinth".into(), p => p }, &s("q"), &s("mc"), &s("loader"),
            &match s("type") { t if t.is_empty() => "mod".into(), t => t }, a["offset"].as_u64().unwrap_or(0)).await,
        "catalog.details" => providers::details(&match s("provider") { p if p.is_empty() => "modrinth".into(), p => p }, &s("projectId")).await,
        "mods.meta" => {
            let dir = if s("kind") == "mods" { mods::mods_dir(&servers::get(&id)?) } else { mods::set_dir(&id) };
            crate::modmeta::for_dir(&dir).await
        }
        "updates.check" => mods::check_updates(&s("kind"), &id).await,
        "updates.apply" => mods::apply_updates(&s("kind"), &id, &a["list"]).await,

        // ---- сборки .mrpack ----
        "pack.inspect" => {
            let path = if s("path").is_empty() { modpack::download(&s("projectId"), Some(&s("versionId"))).await? } else { std::path::PathBuf::from(s("path")) };
            let p = modpack::read(&path)?;
            let mut v = modpack::summary(&p);
            v["path"] = json!(path);
            Ok(v)
        }
        "servers.createFromPack" => {
            let p = modpack::read(std::path::Path::new(&s("packPath")))?;
            let mut o = a.clone();
            o["mcVersion"] = json!(p.mc); o["loader"] = json!(p.loader); o["loaderVersion"] = json!(p.loader_version);
            if s("name").is_empty() { o["name"] = json!(p.name); }
            let client = a["clientSet"].as_bool().unwrap_or(false);
            servers::create_with(&o, Some((p, client)))
        }

        // ---- серверы ----
        "servers.list" => Ok(servers::all()),
        "servers.create" => servers::create(&a),
        "servers.import" => importer::import_servers(&s("path")).await,
        "servers.update" => servers::update(&id, &a["patch"]),
        "servers.remove" => { servers::remove(&id, a["files"].as_bool().unwrap_or(false)).await?; ok() }
        "servers.start" => { servers::start(id).await?; ok() }
        "servers.stop" => { servers::stop(&id).await?; ok() }
        "servers.restart" => { servers::restart(&id).await?; ok() }
        "servers.kill" => { servers::kill(&id); ok() }
        "servers.stopAll" => { servers::stop_all().await; ok() }
        "servers.reinstall" => Ok(json!({ "job": servers::reinstall(&id)? })),
        "servers.changeVersion" => Ok(json!({ "job": servers::change_version(&id, &s("mcVersion"), &s("loaderVersion"))? })),
        "servers.diagnose" => diagnose::diagnose(&id, a["since"].as_u64().unwrap_or(0)),
        "net.info" => network::info(&id).await,
        "net.firewall" => { if web { crate::bail!("Недоступно в веб-режиме"); } network::firewall_add(&id).await }
        "net.check" => network::check(&id).await,
        "files.list" => files::list(&id, &s("path")),
        "files.read" => files::read(&id, &s("path")),
        "files.write" => { files::write(&id, &s("path"), &s("text"))?; ok() }
        "servers.command" => { servers::command(&id, &s("cmd")).await?; ok() }
        "servers.log" => Ok(servers::log(&id)),
        "servers.props" => servers::read_props(&id),
        "servers.saveProps" => { servers::write_props(&id, &a["props"])?; ok() }
        "servers.lists" => servers::lists(&id),

        // ---- бэкапы ----
        "backups.list" => backups::list(&id),
        "backups.create" => Ok(json!({ "job": backups::backup(&id, &s("note"))? })),
        "backups.restore" => Ok(json!({ "job": backups::restore(&id, &s("file"))? })),
        "backups.delete" => { backups::delete(&id, &s("file"))?; ok() }

        // ---- моды сервера ----
        "mods.list" => mods::server_mods(&id),
        "mods.install" => Ok(json!(mods::install_server_mod(&id, &s("provider"), &s("projectId")).await?)),
        "mods.toggle" => { mods::toggle_server_mod(&id, &s("file"))?; ok() }
        "mods.delete" => { mods::delete_server_mod(&id, &s("file"))?; ok() }
        "mods.addFiles" => { if web { crate::bail!("Недоступно в веб-режиме"); } Ok(json!(mods::add_server_files(&id, &strs(&a, "paths"))?)) }
        "mods.upload" => { mods::upload_server_file(&id, &s("name"), &s("data"))?; ok() }

        // ---- наборы клиента ----
        "sets.list" => Ok(mods::list_sets()),
        "sets.create" => Ok(json!(mods::create_set(&a)?)),
        "sets.fromServer" => mods::set_from_server(&id).await,
        "sets.update" => mods::update_set(&id, &a["patch"]),
        "sets.delete" => { mods::delete_set(&id)?; ok() }
        "sets.install" => Ok(json!(mods::install_set_mod(&id, &s("provider"), &s("projectId")).await?)),
        "sets.toggle" => { mods::toggle_set_mod(&id, &s("file"))?; ok() }
        "sets.deleteMod" => { mods::delete_set_mod(&id, &s("file"))?; ok() }
        "sets.addFiles" => { if web { crate::bail!("Недоступно в веб-режиме"); } Ok(json!(mods::add_set_files(&id, &strs(&a, "paths"))?)) }
        "sets.upload" => { mods::upload_set_file(&id, &s("name"), &s("data"))?; ok() }
        "sets.apply" => Ok(json!({ "message": mods::apply_set(&id).await? })),
        "sets.import" => importer::import_sets(&s("path")),
        "sets.importPack" => mods::import_pack_as_set(&s("path")),
        "servers.installPack" => mods::install_pack_to_server(&id, &s("path")),
        "sets.dir" => Ok(json!(mods::set_dir(&id))),

        _ => Err(format!("Неизвестная команда: {cmd}")),
    }
}

#[tauri::command]
pub async fn call(cmd: String, args: Option<Value>) -> Result<Value, String> {
    dispatch(&cmd, args.unwrap_or(json!({})), false).await
}
