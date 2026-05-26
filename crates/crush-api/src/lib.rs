use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncReadExt;
use crush_types::{Result, CrushError, Container, ContainerStatus, PortMapping, Protocol, MountConfig};
use crush_image::db::ImageDatabase;

struct ApiState {
    running: bool,
    containers: Vec<Container>,
}

pub struct ApiServer {
    socket_path: PathBuf,
    state: Arc<Mutex<ApiState>>,
}

impl ApiServer {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            state: Arc::new(Mutex::new(ApiState {
                running: false,
                containers: Vec::new(),
            })),
        }
    }

    pub async fn serve_unix_socket(&self) -> Result<()> {
        let _ = tokio::fs::remove_file(&self.socket_path).await;

        let listener = tokio::net::UnixListener::bind(&self.socket_path)
            .map_err(|e| CrushError::ApiError(format!("Failed to bind Unix socket: {}", e)))?;

        {
            let mut state = self.state.lock().await;
            state.running = true;
        }

        let state = self.state.clone();

        tokio::spawn(async move {
            loop {
                let is_running = { state.lock().await.running };
                if !is_running {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, _)) => {
                        let st = state.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, st).await.ok();
                        });
                    }
                    Err(e) => {
                        eprintln!("API accept error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.running = false;
        }
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        Ok(())
    }
}

async fn handle_connection(mut stream: tokio::net::UnixStream, state: Arc<Mutex<ApiState>>) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await
        .map_err(|e| CrushError::ApiError(format!("Read error: {}", e)))?;

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];

    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).await
            .map_err(|e| CrushError::ApiError(format!("Read header error: {}", e)))?;
        let trimmed = header.trim();
        if trimmed.is_empty() {
            break;
        }
        if trimmed.to_lowercase().starts_with("content-length:") {
            content_length = trimmed[15..].trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await
            .map_err(|e| CrushError::ApiError(format!("Read body error: {}", e)))?;
    }

    let (status, response_body) = route_request(&state, method, path, &body).await;

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        status, response_body.len(), response_body
    );

    stream.write_all(response.as_bytes()).await
        .map_err(|e| CrushError::ApiError(format!("Write error: {}", e)))?;

    Ok(())
}

fn dirs_or_default() -> PathBuf {
    let base = if cfg!(target_os = "linux") {
        PathBuf::from("/var/lib/crush")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData\\Crush".to_string()))
    } else {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("crush")
    };
    std::fs::create_dir_all(&base).ok();
    base
}

#[derive(serde::Deserialize)]
struct CreateBody {
    #[serde(rename = "Image")]
    image: Option<String>,
    #[serde(rename = "Cmd")]
    cmd: Option<Vec<String>>,
    #[serde(rename = "Env")]
    env: Option<Vec<String>>,
}

async fn route_request(
    state: &Arc<Mutex<ApiState>>,
    method: &str,
    path: &str,
    body: &[u8],
) -> (&'static str, String) {
    let path = path.trim_start_matches("/v1.41");

    match (method, path) {
        ("GET", "/containers/json") => {
            let s = state.lock().await;
            let summaries: Vec<serde_json::Value> = s.containers.iter().map(|c| {
                let status_str = match c.status {
                    ContainerStatus::Running => "running",
                    ContainerStatus::Paused => "paused",
                    ContainerStatus::Stopped => "exited",
                    _ => "created",
                };
                serde_json::json!({
                    "Id": c.id,
                    "Names": vec![format!("/{}", c.name)],
                    "Image": c.image,
                    "State": status_str,
                    "Status": format!("Up 10 seconds"),
                    "Created": 1620000000i64,
                    "Ports": c.ports.iter().map(|p| {
                        serde_json::json!({
                            "PrivatePort": p.container_port,
                            "PublicPort": p.host_port,
                            "Type": "tcp"
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect();
            ("200 OK", serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string()))
        }
        ("GET", path) if path.starts_with("/containers/") && path.ends_with("/json") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", r#"{"message":"Invalid path"}"#.to_string());
            }
            let id = parts[2];
            let s = state.lock().await;
            if let Some(c) = s.containers.iter().find(|c| c.id == id || c.name == id) {
                let status_str = match c.status {
                    ContainerStatus::Running => "running",
                    ContainerStatus::Paused => "paused",
                    ContainerStatus::Stopped => "exited",
                    _ => "created",
                };
                let resp = serde_json::json!({
                    "Id": c.id,
                    "Name": format!("/{}", c.name),
                    "State": {
                        "Status": status_str,
                        "Running": c.status == ContainerStatus::Running,
                        "Paused": c.status == ContainerStatus::Paused,
                        "Restarting": false,
                        "Dead": false,
                        "ExitCode": 0,
                        "Pid": c.pid.unwrap_or(0),
                    },
                    "Config": {
                        "Image": c.image,
                    }
                });
                ("200 OK", resp.to_string())
            } else {
                ("404 Not Found", r#"{"message":"Container not found"}"#.to_string())
            }
        }
        ("GET", "/images/json") => {
            let data_dir = dirs_or_default();
            if let Ok(db) = ImageDatabase::new(&data_dir.join("images")) {
                if let Ok(images) = db.list_images().await {
                    let docker_images: Vec<serde_json::Value> = images.iter().map(|img| {
                        serde_json::json!({
                            "Id": format!("sha256:{}", img.id),
                            "RepoTags": vec![img.tag.clone()],
                            "Created": 1620000000i64,
                            "Size": img.size_bytes,
                            "Labels": std::collections::HashMap::<String, String>::new(),
                        })
                    }).collect();
                    if let Ok(serialized) = serde_json::to_string(&docker_images) {
                        return ("200 OK", serialized);
                    }
                }
            }
            ("200 OK", "[]".to_string())
        }
        ("POST", "/containers/create") => {
            let req_body: CreateBody = serde_json::from_slice(body)
                .unwrap_or(CreateBody { image: None, cmd: None, env: None });
            
            let id = format!("crush_{:016x}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos());
            
            let container = Container {
                id: id.clone(),
                name: format!("crush_{}", &id[..8]),
                image: req_body.image.unwrap_or_else(|| "unknown".to_string()),
                status: ContainerStatus::Creating,
                pid: None,
                created_at: std::time::SystemTime::now(),
                started_at: None,
                ports: Vec::new(),
                mounts: Vec::new(),
                memory_limit_bytes: None,
                cpu_shares: None,
            };

            let mut s = state.lock().await;
            s.containers.push(container);

            ("201 Created", format!(r#"{{"Id":"{}","Warnings":[]}}"#, id))
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/start") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", String::new());
            }
            let id = parts[2];
            let mut s = state.lock().await;
            if let Some(c) = s.containers.iter_mut().find(|c| c.id == id || c.name == id) {
                c.status = ContainerStatus::Running;
                c.pid = Some(rand_pid());
                c.started_at = Some(std::time::SystemTime::now());
                ("204 No Content", String::new())
            } else {
                ("404 Not Found", String::new())
            }
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/stop") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", String::new());
            }
            let id = parts[2];
            let mut s = state.lock().await;
            if let Some(c) = s.containers.iter_mut().find(|c| c.id == id || c.name == id) {
                c.status = ContainerStatus::Stopped;
                c.pid = None;
                ("204 No Content", String::new())
            } else {
                ("404 Not Found", String::new())
            }
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/pause") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", String::new());
            }
            let id = parts[2];
            let mut s = state.lock().await;
            if let Some(c) = s.containers.iter_mut().find(|c| c.id == id || c.name == id) {
                c.status = ContainerStatus::Paused;
                ("204 No Content", String::new())
            } else {
                ("404 Not Found", String::new())
            }
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/unpause") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", String::new());
            }
            let id = parts[2];
            let mut s = state.lock().await;
            if let Some(c) = s.containers.iter_mut().find(|c| c.id == id || c.name == id) {
                c.status = ContainerStatus::Running;
                ("204 No Content", String::new())
            } else {
                ("404 Not Found", String::new())
            }
        }
        ("DELETE", path) if path.starts_with("/containers/") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", String::new());
            }
            let id = parts[2];
            let mut s = state.lock().await;
            if let Some(pos) = s.containers.iter().position(|c| c.id == id || c.name == id) {
                s.containers.remove(pos);
                ("204 No Content", String::new())
            } else {
                ("404 Not Found", String::new())
            }
        }
        ("GET", "/info") => {
            ("200 OK", r#"{"Name":"crush","Version":"0.1.0"}"#.to_string())
        }
        ("GET", "/_ping") => {
            ("200 OK", "OK".to_string())
        }
        _ => {
            ("404 Not Found", r#"{"message":"Not found"}"#.to_string())
        }
    }
}

fn rand_pid() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (nanos as u32 % 65535) + 1
}
