use serde::Serialize;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crush_build::detect::CrushSpecDetector;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub runtime: String,
    pub version: String,
    pub framework: Option<String>,
    pub port: u16,
    pub confidence: f32,
    pub is_monorepo: bool,
    pub env_required: Vec<String>,
    pub service_count: usize,
}

/// Heuristically detect the stack of a project directory — same detector the
/// `crush` run flow uses, so the dashboard shows what would actually run.
#[tauri::command]
pub async fn detect_project(path: String) -> Result<ProjectInfo, String> {
    let root = std::path::PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    let det = CrushSpecDetector::new().detect(&root);
    Ok(ProjectInfo {
        name: det.project_name,
        runtime: format!("{:?}", det.runtime_type),
        version: det.runtime_version,
        framework: if det.framework_detected && !det.framework_name.is_empty() {
            Some(det.framework_name)
        } else {
            None
        },
        port: det.port,
        confidence: det.confidence,
        is_monorepo: det.is_monorepo,
        env_required: det.env_required,
        service_count: det.services.len(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub disk_used_bytes: u64,
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            match e.metadata() {
                Ok(m) if m.is_dir() => total += dir_size(&e.path()),
                Ok(m) => total += m.len(),
                Err(_) => {}
            }
        }
    }
    total
}

#[tauri::command]
pub async fn system_info(state: State<'_, AppState>) -> Result<SystemInfo, String> {
    let data_dir = state.data_dir.clone();
    let disk = tokio::task::spawn_blocking({
        let d = data_dir.clone();
        move || dir_size(&d)
    })
    .await
    .unwrap_or(0);
    Ok(SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: data_dir.to_string_lossy().to_string(),
        disk_used_bytes: disk,
    })
}
