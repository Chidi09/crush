use tauri::{State, Window, Manager, Emitter};
use crate::state::{AppState, RunProcess, RunId};
use uuid::Uuid;

#[tauri::command]
pub async fn run_project(
    project_path: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let project_root = std::path::PathBuf::from(&project_path);
    let options = crush_build::run::RunOptions {
        dev: false,
        rebuild: false,
        repack: false,
        no_proxy: false,
        watch: false,
        memory_bytes: None,
        cpu_fraction: None,
        priority: None,
        assume_yes: true,
    };

    let handle = crush_build::run::run_project(project_root, state.data_dir.clone(), options)
        .await
        .map_err(|e| e.to_string())?;

    let run_id = handle.run_id;
    let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel::<()>();
    let mut events = handle.events;
    let app_handle = window.app_handle().clone();
    let emit_window = window.clone();

    // Save abort handle
    {
        let mut runs = state.runs.write().await;
        runs.insert(run_id, RunProcess { abort: abort_tx });
    }

    // Spawn event forwarder
    tokio::spawn(async move {
        use tokio::select;
        loop {
            select! {
                Some(event) = events.recv() => {
                    let kind = event_name(&event);
                    let _ = emit_window.emit(&format!("run-event::{}::{}", run_id, kind), &event);
                    if matches!(event, crush_build::run::RunEvent::Exited { .. }) {
                        break;
                    }
                }
                _ = &mut abort_rx => {
                    let _ = emit_window.emit(&format!("run-event::{}::aborted", run_id), "");
                    break;
                }
                else => break,
            }
        }
        // Cleanup
        let mut runs = app_handle.state::<AppState>().runs.write().await;
        runs.remove(&run_id);
    });

    Ok(run_id.to_string())
}

fn event_name(event: &crush_build::run::RunEvent) -> &'static str {
    match event {
        crush_build::run::RunEvent::Detected { .. } => "detected",
        crush_build::run::RunEvent::DepStarted { .. } => "dep-started",
        crush_build::run::RunEvent::DepFailed { .. } => "dep-failed",
        crush_build::run::RunEvent::ImageFresh { .. } => "image-fresh",
        crush_build::run::RunEvent::ImagePacked { .. } => "image-packed",
        crush_build::run::RunEvent::BuildStarted { .. } => "build-started",
        crush_build::run::RunEvent::BuildOutput { .. } => "build-output",
        crush_build::run::RunEvent::BuildFinished { .. } => "build-finished",
        crush_build::run::RunEvent::Spawning { .. } => "spawning",
        crush_build::run::RunEvent::AppOutput { .. } => "app-output",
        crush_build::run::RunEvent::PortBound { .. } => "port-bound",
        crush_build::run::RunEvent::Exited { .. } => "exited",
        crush_build::run::RunEvent::Warning { .. } => "warning",
        crush_build::run::RunEvent::WarmRun => "warm-run",
        crush_build::run::RunEvent::DepsFresh => "deps-fresh",
    }
}

#[tauri::command]
pub async fn abort_run(run_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uid = Uuid::parse_str(&run_id).map_err(|e| e.to_string())?;
    let mut runs = state.runs.write().await;
    if let Some(proc) = runs.remove(&uid) {
        let _ = proc.abort.send(());
    }
    Ok(())
}
