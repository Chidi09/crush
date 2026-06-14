use tauri::{State, Window, Manager, Emitter};
use crate::state::{AppState, RunProcess, RunId};
use uuid::Uuid;

#[tauri::command]
pub async fn run_project(
    project_path: String,
    dev_mode: bool,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let project_root = std::path::PathBuf::from(&project_path);
    let options = crush_build::run::RunOptions {
        dev: dev_mode,
        rebuild: false,
        repack: false,
        no_proxy: false,
        watch: false,
        memory_bytes: None,
        cpu_fraction: None,
        priority: None,
        assume_yes: true,
    };

    let handle = crush_build::run::run_project(project_root.clone(), state.data_dir.clone(), options)
        .await
        .map_err(|e| e.to_string())?;

    // Best-effort: register a real, exportable OCI image for this project so the
    // crush it just did actually shows up in the Images list (and can be run /
    // `crush export`ed), mirroring `crush build`. Runs in the background so it
    // never blocks the dev run; the base image is pulled at most once.
    {
        let store = state.store.clone();
        let proj = project_root.clone();
        tokio::spawn(async move {
            let detector = crush_build::StackDetector::new();
            if let Ok(stack) = detector.detect(&proj).await {
                let name = proj.file_name().and_then(|n| n.to_str()).unwrap_or("app");
                let tag = format!("{}:latest", name);
                let entry = if !stack.entry_point.trim().is_empty() {
                    stack.entry_point.clone()
                } else {
                    stack.build_command.clone()
                };
                let cmd = if entry.trim().is_empty() {
                    Vec::new()
                } else {
                    vec!["sh".to_string(), "-c".to_string(), entry]
                };
                let env = vec![format!("PORT={}", stack.default_port)];
                if let Err(e) = store
                    .commit_app_image(&tag, &stack.base_image, &proj, "/app", cmd, env)
                    .await
                {
                    eprintln!("crush: image registration failed for {}: {}", tag, e);
                }
            }
        });
    }

    let run_id = handle.run_id;
    let mut events = handle.events;
    let app_handle = window.app_handle().clone();
    let emit_window = window.clone();

    // Save abort handle
    {
        let mut runs = state.runs.write().await;
        runs.insert(run_id, RunProcess { abort: handle.abort });
    }

    // Spawn event forwarder
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let _ = emit_window.emit(&format!("run-event::{}", run_id), &event);
            if matches!(event, crush_build::run::RunEvent::Exited { .. }) {
                break;
            }
        }
        // Cleanup
        let cleanup_state = app_handle.state::<AppState>();
        let mut runs = cleanup_state.runs.write().await;
        runs.remove(&run_id);
    });

    Ok(run_id.to_string())
}


#[tauri::command]
pub async fn abort_run(run_id: String, window: Window, state: State<'_, AppState>) -> Result<(), String> {
    let uid = Uuid::parse_str(&run_id).map_err(|e| e.to_string())?;
    let mut runs = state.runs.write().await;
    if let Some(proc) = runs.remove(&uid) {
        let _ = proc.abort.send(());
        let _ = window.emit(&format!("run-event::{}", run_id), serde_json::json!({ "kind": "aborted" }));
    }
    Ok(())
}
