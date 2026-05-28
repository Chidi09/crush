use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosisResult {
    pub summary: String,
    pub details: Option<String>,
    pub fix: Option<String>,
}

#[tauri::command]
pub async fn diagnose_logs(lines: Vec<String>, state: State<'_, AppState>) -> Result<DiagnosisResult, String> {
    let stderr = lines.join("\n");
    let diagnosis = state.ai.diagnose_stderr(&stderr, None, None).await.map_err(|e| e.to_string())?;
    let detail = diagnosis.diagnosis.as_ref();
    let summary = detail
        .map(|d| d.root_cause.trim().to_string())
        .unwrap_or_else(|| "No diagnosis returned".to_string());
    let fix = detail.map(|d| d.fix_description.trim().to_string());
    Ok(DiagnosisResult {
        summary,
        details: None,
        fix,
    })
}
