use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct BuildSummary {
    pub timestamp_ms: u64,
    pub project_name: String,
    pub language: String,
    pub framework: String,
    pub duration_ms: u64,
    pub was_cached: bool,
    pub size_bytes: u64,
    pub digest: String,
    pub success: bool,
}

#[tauri::command]
pub async fn list_build_history(limit: Option<usize>, state: State<'_, AppState>) -> Result<Vec<BuildSummary>, String> {
    let limit = limit.unwrap_or(50);
    let records = crush_build::run::read_build_history(&state.data_dir);
    let result: Vec<BuildSummary> = records.into_iter().take(limit).map(|r| BuildSummary {
        timestamp_ms: r.timestamp_ms,
        project_name: r.project_name,
        language: r.language,
        framework: r.framework,
        duration_ms: r.duration_ms,
        was_cached: r.was_cached,
        size_bytes: r.size_bytes,
        digest: r.digest,
        success: r.success,
    }).collect();
    Ok(result)
}
