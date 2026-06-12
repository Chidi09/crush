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
                if let Ok(mut f) = tokio::fs::File::open(&path).await {
                    if let Ok(meta) = f.metadata().await {
                        let len = meta.len();
                        offsets.insert(stream_name.to_string(), len);
                        use tokio::io::AsyncSeekExt;
                        let max_tail = 256 * 1024;
                        let start = if len > max_tail { len - max_tail } else { 0 };
                        if start > 0 {
                            let _ = f.seek(std::io::SeekFrom::Start(start)).await;
                        }
                        let mut buf = Vec::new();
                        let _ = f.read_to_end(&mut buf).await;
                        let text = String::from_utf8_lossy(&buf);
                        // Split and handle partial lines correctly if we started mid-line
                        let mut lines: Vec<&str> = text.lines().collect();
                        if start > 0 && !lines.is_empty() {
                            lines.remove(0); // discard the first partial line
                        }
                        let tail = if lines.len() > 500 { &lines[lines.len() - 500..] } else { &lines[..] };
                        let mut replay = Vec::new();
                        for line in tail {
                            replay.push(crate::events::LogLine {
                                ts: "".to_string(),
                                stream: stream_name.to_string(),
                                text: line.to_string(),
                            });
                        }
                        if !replay.is_empty() {
                            crate::events::emit_log_replay(&window, &container_id, replay);
                        }
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

/// Read the tail of a native service's log file. Native services (Postgres,
/// Redis, MongoDB, MinIO…) redirect stdout+stderr to
/// `{data_dir}/services/logs/{project}/{name}.log` at spawn time, so we just
/// read that file. Returns the last `max_lines` lines (default 800).
#[tauri::command]
pub async fn read_service_log(
    project: String,
    name: String,
    max_lines: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = state.data_dir
        .join("services").join("logs").join(&project).join(format!("{}.log", name));
    if !path.exists() {
        return Ok(String::new());
    }
    let content = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    let cap = max_lines.unwrap_or(800);
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(cap);
    Ok(lines[start..].join("\n"))
}

#[tauri::command]
pub async fn unsubscribe_logs(container_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut tailers = state.log_tailers.write().await;
    if let Some(tailer) = tailers.remove(&container_id) {
        let _ = tailer.shutdown.send(());
    }
    Ok(())
}
