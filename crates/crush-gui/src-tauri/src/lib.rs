pub mod commands;
pub mod events;
pub mod state;
pub mod platform;

use state::AppState;
use std::sync::Arc;
use tauri::{Manager, Emitter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = AppState::data_dir();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // The setup closure does not run inside a Tokio runtime context, so
            // `Handle::current()` would panic. Use Tauri's managed async runtime.
            let store = match tauri::async_runtime::block_on(async {
                crush_image::ImageStore::new(data_dir.join("images")).await
            }) {
                Ok(s) => s,
                Err(e) => {
                    rfd::MessageDialog::new()
                        .set_title("Crush Launch Error")
                        .set_description(&format!("Failed to initialize data directory at {}\n\nError: {}", data_dir.display(), e))
                        .set_level(rfd::MessageLevel::Error)
                        .show();
                    std::process::exit(1);
                }
            };

            let ai = crush_ai::AiEngine::new(None, data_dir.clone());

            let mailbox = crush_build::mailbox::MailStore::new();

            app.manage(AppState {
                data_dir,
                store: Arc::new(store),
                ai: Arc::new(ai),
                runs: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                log_tailers: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                tunnels: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                mailbox: mailbox.clone(),
            });

            // Start the local mail catcher (SMTP sink on :1025). It captures any
            // mail an app sends while developing and notifies the UI on arrival.
            // Bind failures (port taken) are non-fatal — the app still runs.
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let on_new = move |_m: &crush_build::mailbox::CapturedMail| {
                        let _ = handle.emit("mail-received", ());
                    };
                    if let Err(e) = crush_build::mailbox::serve(
                        crush_build::mailbox::DEFAULT_PORT, mailbox, on_new).await
                    {
                        eprintln!("mail catcher not started: {e}");
                    }
                });
            }

            commands::database::spawn_backup_task();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::containers::list_containers,
            commands::containers::stop_container,
            commands::services::list_native_services,
            commands::services::start_native_service,
            commands::services::stop_native_service,
            commands::services::get_connection_string,
            commands::images::list_images,
            commands::images::inspect_image,
            commands::images::pull_image,
            commands::images::remove_image,
            commands::images::list_catalog,
            commands::run_cmd::run_project,
            commands::run_cmd::abort_run,
            commands::logs::subscribe_logs,
            commands::logs::unsubscribe_logs,
            commands::logs::read_service_log,
            commands::build::list_build_history,
            commands::ai::diagnose_logs,
            commands::platform::pick_project_directory,
            commands::platform::open_url,
            commands::platform::reveal_in_explorer,
            commands::system::detect_project,
            commands::system::system_info,
            commands::system::system_resources,
            commands::deployments::save_deployment,
            commands::deployments::list_deployments,
            commands::deployments::get_deployment,
            commands::deployments::delete_deployment,
            commands::deployments::capture_preview,
            commands::git::git_info,
            commands::git::git_branches,
            commands::git::preview_branch,
            commands::git::remove_worktree,
            commands::git::switch_branch,
            commands::git::list_worktrees,
            commands::deployments::list_all_deployments,
            commands::deployments::list_cloud_deployments,
            commands::eject::eject_project,
            commands::deploy::write_project_file,
            commands::deploy::detect_deploy_targets,
            commands::deploy::cli_available,
            commands::deploy::run_deploy,
            commands::deploy::run_capture,
            commands::deploy::open_terminal,
            commands::config::get_config,
            commands::config::set_config,
            commands::env::read_env,
            commands::env::write_env,
            commands::inspect::inspect_postgres,
            commands::inspect::inspect_redis,
            commands::inspect::inspect_mongo,
            commands::inspect::inspect_minio,
            commands::device::adb_devices,
            commands::device::device_screencap,
            commands::device::device_tap,
            commands::device::device_swipe,
            commands::device::device_key,
            commands::tunnel::start_tunnel,
            commands::tunnel::stop_tunnel,
            commands::tunnel::list_tunnels,
            commands::mailbox::list_mail,
            commands::mailbox::clear_mail,
            commands::servers::ssh_hosts,
            commands::servers::ssh_connect,
            commands::servers::server_health,
            commands::servers::server_containers,
            commands::servers::server_container_stats,
            commands::servers::server_services,
            commands::servers::server_service_restart,
            commands::servers::server_container_logs,
            commands::servers::server_container_logs_follow,
            commands::servers::server_container_logs_unfollow,
            commands::servers::server_container_restart,
            commands::servers::server_container_stop,
            commands::servers::server_container_exec,
            commands::database::db_status,
            commands::database::db_backups,
            commands::database::db_backup_now,
            commands::database::db_restore,
            commands::database::db_delete_backup,
            commands::database::db_run_query,
            commands::database::redis_list_keys,
            commands::database::redis_get_val,
            commands::database::redis_set_val,
            commands::database::redis_del_key,
            commands::database::mongo_list_databases,
            commands::database::mongo_list_collections,
            commands::database::mongo_find_docs,
            commands::database::mongo_insert_doc,
            commands::database::mongo_update_doc,
            commands::database::mongo_delete_doc,
            commands::gateway::list_domains,
            commands::gateway::add_domain,
            commands::gateway::remove_domain,
            commands::storage::storage_get_connections,
            commands::storage::storage_save_connections,
            commands::storage::storage_list_buckets,
            commands::storage::storage_create_bucket,
            commands::storage::storage_delete_bucket,
            commands::storage::storage_list_objects,
            commands::storage::storage_upload_object,
            commands::storage::storage_upload_bytes,
            commands::storage::storage_download_object,
            commands::storage::storage_delete_objects,
            commands::storage::storage_get_presigned_url,
            commands::storage::storage_get_bucket_policy,
            commands::storage::storage_set_bucket_policy,
            commands::storage::storage_set_bucket_public,
            commands::storage::storage_get_object_metadata,
            commands::storage::storage_set_object_metadata,
            commands::storage::storage_read_object_preview,
            commands::storage::storage_pick_upload_file,
            commands::storage::storage_pick_download_destination,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Crush GUI");
}
