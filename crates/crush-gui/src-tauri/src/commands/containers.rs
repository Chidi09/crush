use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
use crate::state::AppState;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub pid: Option<u32>,
    pub created_at: u64,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub uptime_secs: Option<u64>,
    pub ports: Vec<PortInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub host_port: u16,
    pub container_port: u16,
}

fn list_containers_from_disk(data_dir: &PathBuf) -> Vec<ContainerSummary> {
    let containers_dir = data_dir.join("containers");
    let mut result = Vec::new();
    if !containers_dir.exists() {
        return result;
    }
    if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
        while let Some(Ok(entry)) = entries.next() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let json_path = path.join("container.json");
            if !json_path.exists() { continue; }
            if let Ok(content) = std::fs::read_to_string(&json_path) {
                if let Ok(c) = serde_json::from_str::<crush_types::Container>(&content) {
                    let status_str = match c.status {
                        crush_types::ContainerStatus::Running => "running",
                        crush_types::ContainerStatus::Stopped => "exited",
                        crush_types::ContainerStatus::Creating => "creating",
                        crush_types::ContainerStatus::Created => "created",
                        crush_types::ContainerStatus::Paused => "paused",
                    }.to_string();
                    let uptime = c.started_at.map(|s| {
                        let now = SystemTime::now();
                        now.duration_since(s).unwrap_or_default().as_secs()
                    });
                    let ports: Vec<PortInfo> = c.ports.iter().map(|p| PortInfo {
                        host_port: p.host_port,
                        container_port: p.container_port,
                    }).collect();
                    result.push(ContainerSummary {
                        id: c.id,
                        name: c.name,
                        image: c.image,
                        status: status_str,
                        pid: c.pid,
                        created_at: c.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                        cpu_percent: None,
                        memory_bytes: c.memory_limit_bytes,
                        uptime_secs: uptime,
                        ports,
                    });
                }
            }
        }
    }
    result
}

#[tauri::command]
pub async fn list_containers(state: State<'_, AppState>) -> Result<Vec<ContainerSummary>, String> {
    Ok(list_containers_from_disk(&state.data_dir))
}

#[tauri::command]
pub async fn stop_container(id: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let containers_dir = state.data_dir.join("containers");
    let container_path = containers_dir.join(&id).join("container.json");
    if !container_path.exists() {
        return Err(format!("Container {} not found", id));
    }
    let content = std::fs::read_to_string(&container_path).map_err(|e| e.to_string())?;
    let c: crush_types::Container = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    if let Some(pid) = c.pid {
        use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }

    crate::events::emit_container_state_changed(&app);
    Ok(())
}
