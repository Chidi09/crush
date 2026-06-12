pub mod commands;
pub mod events;
pub mod state;
pub mod platform;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;

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

            app.manage(AppState {
                data_dir,
                store: Arc::new(store),
                ai: Arc::new(ai),
                runs: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
                log_tailers: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::containers::list_containers,
            commands::containers::stop_container,
            commands::services::list_native_services,
            commands::services::stop_native_service,
            commands::services::get_connection_string,
            commands::images::list_images,
            commands::images::pull_image,
            commands::images::remove_image,
            commands::run_cmd::run_project,
            commands::run_cmd::abort_run,
            commands::logs::subscribe_logs,
            commands::logs::unsubscribe_logs,
            commands::build::list_build_history,
            commands::ai::diagnose_logs,
            commands::platform::pick_project_directory,
            commands::platform::open_url,
            commands::platform::reveal_in_explorer,
            commands::system::detect_project,
            commands::system::system_info,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Crush GUI");
}
