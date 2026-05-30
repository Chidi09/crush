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
    
    // Load api_key from config.json if it exists
    let config_path = state.data_dir.join("config.json");
    let api_key = if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                json.get("ai_api_key").and_then(|v| v.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let mut ai = (*state.ai).clone();
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            ai.api_key = Some(key);
        }
    }

    let diagnosis = ai.diagnose_stderr(&stderr, None, None).await.map_err(|e| e.to_string())?;
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
