use tauri::{State, Window};
use crate::state::AppState;
use crate::events;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

fn log_paths(data_dir: &PathBuf, container_id: &str) -> Option<(PathBuf, PathBuf)> {
    let base = data_dir.join("containers").join(container_id);
    let stdout = base.join("stdout.log");
    let stderr = base.join("stderr.log");
    if stdout.exists() || stderr.exists() {
        Some((stdout, stderr))
    } else {
        None
    }
}

#[tauri::command]
pub async fn subscribe_logs(
    container_id: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let paths = log_paths(&state.data_dir, &container_id)
        .ok_or_else(|| format!("No logs for container {}", container_id))?;

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut tailers = state.log_tailers.write().await;
        tailers.insert(container_id.clone(), crate::state::LogTailer { shutdown: shutdown_tx });
    }

    tokio::spawn(async move {
        let mut offsets: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for (path, stream_name) in [(&paths.0, "stdout"), (&paths.1, "stderr")] {
            if path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    offsets.insert(stream_name.to_string(), content.len() as u64);
                    // emit existing lines
                    for line in content.lines() {
                        events::emit_log_line(&window, &container_id, "", stream_name, line);
                    }
                }
            }
        }

        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {}
            }
            for (path, stream_name) in [(paths.0.clone(), "stdout"), (paths.1.clone(), "stderr")] {
                if !path.exists() { continue; }
                let offset = *offsets.get(stream_name).unwrap_or(&0);
                if let Ok(mut f) = tokio::fs::File::open(&path).await {
                    use tokio::io::AsyncSeekExt;
                    let len = f.seek(std::io::SeekFrom::End(0)).await.unwrap_or(0);
                    if len > offset {
                        f.seek(std::io::SeekFrom::Start(offset)).await.unwrap_or_default();
                        let mut buf = Vec::new();
                        f.read_to_end(&mut buf).await.unwrap_or_default();
                        offsets.insert(stream_name.to_string(), len);
                        let text = String::from_utf8_lossy(&buf);
                        for line in text.lines() {
                            events::emit_log_line(&window, &container_id, "", stream_name, line);
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn unsubscribe_logs(container_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut tailers = state.log_tailers.write().await;
    if let Some(tailer) = tailers.remove(&container_id) {
        let _ = tailer.shutdown.send(());
    }
    Ok(())
}
