use serde::Serialize;
use tauri::State;
use std::collections::HashMap;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct NativeServiceSummary {
    pub project: String,
    pub name: String,
    pub kind: String,
    pub port: u16,
    pub pid: u32,
    pub connection_string: Option<String>,
    pub data_dir: String,
    pub started_at: u64,
}

#[tauri::command]
pub async fn list_native_services(state: State<'_, AppState>) -> Result<Vec<NativeServiceSummary>, String> {
    let services_dir = state.data_dir.join("services").join("native");
    let mut result = Vec::new();
    if !services_dir.exists() {
        return Ok(result);
    }
    if let Ok(entries) = std::fs::read_dir(&services_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e != "json").unwrap_or(true) { continue; }
            if let Some(project) = path.file_stem().and_then(|s| s.to_str()) {
                use crush_services::load_native_state;
                if let Some(state_data) = load_native_state(&services_dir.parent().unwrap_or(&services_dir), project) {
                    for svc in &state_data.services {
                        let kind = match svc.kind {
                            crush_services::ServiceKind::Postgres => "postgres",
                            crush_services::ServiceKind::RedisCompat => "redis-compat",
                            crush_services::ServiceKind::MySQL => "mysql",
                        }.to_string();
                        let conn_str = connection_string_for(&kind, svc.port, project, &svc.name);
                        result.push(NativeServiceSummary {
                            project: state_data.project.clone(),
                            name: svc.name.clone(),
                            kind,
                            port: svc.port,
                            pid: svc.pid,
                            connection_string: conn_str,
                            data_dir: svc.data_dir.to_string_lossy().to_string(),
                            started_at: state_data.started_at,
                        });
                    }
                }
            }
        }
    }
    Ok(result)
}

fn connection_string_for(kind: &str, port: u16, project: &str, _name: &str) -> Option<String> {
    match kind {
        "postgres" => Some(format!(
            "postgresql://{}:{}@localhost:{}/{}",
            project.replace('-', "_"),
            project.replace('-', "_"),
            port,
            project.replace('-', "_"),
        )),
        "redis-compat" => Some(format!("redis://localhost:{}", port)),
        _ => None,
    }
}

#[tauri::command]
pub async fn stop_native_service(name: String, project: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use crush_services::load_native_state;
    if let Some(mut state_data) = load_native_state(&state.data_dir.join("services"), &project) {
        state_data.services.retain(|s| s.name != name);
        crush_services::save_native_state(&state.data_dir.join("services"), &state_data)
            .map_err(|e| e.to_string())?;
        crate::events::emit_service_state_changed(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_connection_string(name: String, project: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    use crush_services::load_native_state;
    if let Some(state_data) = load_native_state(&state.data_dir.join("services"), &project) {
        for svc in &state_data.services {
            if svc.name == name {
                let kind = match svc.kind {
                    crush_services::ServiceKind::Postgres => "postgres",
                    crush_services::ServiceKind::RedisCompat => "redis-compat",
                    crush_services::ServiceKind::MySQL => "mysql",
                };
                return Ok(connection_string_for(kind, svc.port, &project, &name));
            }
        }
    }
    Ok(None)
}
