use serde::Serialize;
use tauri::State;
use crate::state::AppState;

/// A native service's state file persists its PID; verify the process is still
/// alive so we don't report stale/dead services (e.g. a redis whose process was
/// killed but whose state JSON was never cleaned up).
#[cfg(target_os = "windows")]
fn pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::System::Threading::{OpenProcess, GetExitCodeProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::Foundation::CloseHandle;
    const STILL_ACTIVE: u32 = 259;
    if pid == 0 { return false; }
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h == 0 { return false; }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(h, &mut code);
        CloseHandle(h);
        ok != 0 && code == STILL_ACTIVE
    }
}

#[cfg(not(target_os = "windows"))]
fn pid_alive(pid: u32) -> bool {
    pid != 0 && std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Some services (notably Postgres via `pg_ctl`) daemonize, so Crush records a
/// pid of 0. In that case — or if the process check fails — fall back to a quick
/// TCP probe of the service port, which is the real "is it up?" signal.
fn port_listening(port: u16) -> bool {
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    if port == 0 { return false; }
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(250)).is_ok()
}

fn is_service_alive(pid: u32, port: u16) -> bool {
    if pid != 0 && pid_alive(pid) {
        return true;
    }
    port_listening(port)
}

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
    pub console_url: Option<String>,
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
                        if !is_service_alive(svc.pid, svc.port) { continue; }
                        let kind = match svc.kind {
                            crush_services::ServiceKind::Postgres => "postgres",
                            crush_services::ServiceKind::RedisCompat => "redis-compat",
                            crush_services::ServiceKind::MySQL => "mysql",
                            crush_services::ServiceKind::MongoDB => "mongodb",
                            crush_services::ServiceKind::ObjectStore => "minio",
                        }.to_string();
                        let conn_str = connection_string_for(&kind, svc.port, project, &svc.name);
                        let console_url = svc.console_port.map(|cp| format!("http://localhost:{}", cp));
                        result.push(NativeServiceSummary {
                            project: state_data.project.clone(),
                            name: svc.name.clone(),
                            kind,
                            port: svc.port,
                            pid: svc.pid,
                            connection_string: conn_str,
                            data_dir: svc.data_dir.to_string_lossy().to_string(),
                            started_at: state_data.started_at,
                            console_url,
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
        "mysql" => Some(format!("mysql://root:crush@localhost:{}/{}", port, project.replace('-', "_"))),
        "redis-compat" => Some(format!("redis://localhost:{}", port)),
        "mongodb" => Some(format!("mongodb://localhost:{}", port)),
        "minio" => Some(format!("http://localhost:{} (credentials: minioadmin/minioadmin)", port)),
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

/// Spin up a native service on demand, independent of any project — so the
/// Services screen isn't a dead end when nothing is running. Started services
/// are grouped under the synthetic "scratch" project so they persist and list.
#[tauri::command]
pub async fn start_native_service(
    kind: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<NativeServiceSummary, String> {
    use crush_build::{DepService, start_dep_service_smart, StartedService};
    use crush_services::{load_native_state, save_native_state, NativeServiceState, ServiceKind};

    const PROJECT: &str = "scratch";

    // Map the chosen kind to an image + default port + minimal env.
    let (image, port, env): (&str, u16, Vec<(String, String)>) = match kind.as_str() {
        "postgres" => ("postgres:16", 5432, vec![
            ("POSTGRES_USER".into(), "crush".into()),
            ("POSTGRES_PASSWORD".into(), "crush".into()),
            ("POSTGRES_DB".into(), "crush".into()),
        ]),
        "redis" | "redis-compat" => ("redis:7", 6379, vec![]),
        "mongodb" | "mongo" => ("mongo:7", 27017, vec![]),
        "mysql" | "mariadb" => ("mysql:8", 3306, vec![
            ("MYSQL_ROOT_PASSWORD".into(), "crush".into()),
            ("MYSQL_DATABASE".into(), "crush".into()),
        ]),
        "minio" => ("minio/minio", 9000, vec![
            ("MINIO_ROOT_USER".into(), "minioadmin".into()),
            ("MINIO_ROOT_PASSWORD".into(), "minioadmin".into()),
        ]),
        other => return Err(format!("unknown service kind '{other}' (use postgres, mysql, redis, mongodb, minio)")),
    };

    let dep = DepService {
        name: kind.clone(),
        image: image.to_string(),
        ports: vec![(port, port)],
        env,
        volumes: vec![],
    };

    let running = match start_dep_service_smart(&dep, PROJECT, &state.data_dir).await {
        Ok(StartedService::Native(r)) => r,
        Ok(StartedService::Container(_)) => return Err("service started as a container, not natively".into()),
        Err(e) => return Err(format!("failed to start {kind}: {e}")),
    };

    // Persist into the "scratch" project's native state (upsert by name).
    let state_dir = state.data_dir.join("services");
    let mut data = load_native_state(&state_dir, PROJECT).unwrap_or(NativeServiceState {
        project: PROJECT.to_string(),
        services: vec![],
        started_at: now_secs(),
    });
    data.services.retain(|s| s.name != running.name);
    data.services.push(running.clone());
    save_native_state(&state_dir, &data).map_err(|e| e.to_string())?;
    crate::events::emit_service_state_changed(&app);

    let kind_str = match running.kind {
        ServiceKind::Postgres => "postgres",
        ServiceKind::RedisCompat => "redis-compat",
        ServiceKind::MySQL => "mysql",
        ServiceKind::MongoDB => "mongodb",
        ServiceKind::ObjectStore => "minio",
    };
    Ok(NativeServiceSummary {
        project: PROJECT.to_string(),
        name: running.name.clone(),
        kind: kind_str.to_string(),
        port: running.port,
        pid: running.pid,
        connection_string: connection_string_for(kind_str, running.port, PROJECT, &running.name),
        data_dir: running.data_dir.to_string_lossy().to_string(),
        started_at: data.started_at,
        console_url: running.console_port.map(|p| format!("http://localhost:{p}")),
    })
}

fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
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
                    crush_services::ServiceKind::MongoDB => "mongodb",
                    crush_services::ServiceKind::ObjectStore => "minio",
                };
                return Ok(connection_string_for(kind, svc.port, &project, &name));
            }
        }
    }
    Ok(None)
}
