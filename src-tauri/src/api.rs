//! Единая точка входа: команды UI (через IPC Tauri или веб-режим).
use crate::core::{self, arg_str, Res};
use crate::{backups, importer, installer, mods, servers};
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
        "open" => { if web { crate::bail!("Недоступно в веб-режиме"); } core::open_path(&s("path"))?; ok() }
        "versions" => installer::mc_versions(a["snapshots"].as_bool().unwrap_or(false)).await,
        "loaderVersions" => installer::loader_versions(&s("loader"), &s("mc")).await,
        "modrinth.search" => mods::search(&s("q"), &s("mc"), &s("loader"), &match s("type") { t if t.is_empty() => "mod".into(), t => t }, a["offset"].as_u64().unwrap_or(0)).await,

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
        "servers.reinstall" => Ok(json!({ "job": servers::reinstall(&id)? })),
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
        "mods.install" => Ok(json!(mods::install_server_mod(&id, &s("projectId")).await?)),
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
        "sets.install" => Ok(json!(mods::install_set_mod(&id, &s("projectId")).await?)),
        "sets.toggle" => { mods::toggle_set_mod(&id, &s("file"))?; ok() }
        "sets.deleteMod" => { mods::delete_set_mod(&id, &s("file"))?; ok() }
        "sets.addFiles" => { if web { crate::bail!("Недоступно в веб-режиме"); } Ok(json!(mods::add_set_files(&id, &strs(&a, "paths"))?)) }
        "sets.upload" => { mods::upload_set_file(&id, &s("name"), &s("data"))?; ok() }
        "sets.apply" => Ok(json!({ "message": mods::apply_set(&id).await? })),
        "sets.import" => importer::import_sets(&s("path")),
        "sets.dir" => Ok(json!(mods::set_dir(&id))),

        _ => Err(format!("Неизвестная команда: {cmd}")),
    }
}

#[tauri::command]
pub async fn call(cmd: String, args: Option<Value>) -> Result<Value, String> {
    dispatch(&cmd, args.unwrap_or(json!({})), false).await
}
