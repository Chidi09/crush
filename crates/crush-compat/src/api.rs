use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError};

const DOCKER_API_VERSION: &str = "v1.41";

pub struct DockerApiServer {
    socket_path: PathBuf,
    state: Arc<Mutex<ApiState>>,
}

struct ApiState {
    containers: Vec<DockerContainerSummary>,
    images: Vec<DockerImageSummary>,
    running: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DockerContainerSummary {
    Id: Option<String>,
    Names: Option<Vec<String>>,
    Image: Option<String>,
    ImageID: Option<String>,
    State: Option<String>,
    Status: Option<String>,
    Created: Option<i64>,
    Ports: Option<Vec<DockerPort>>,
    Mounts: Option<Vec<DockerMount>>,
    NetworkSettings: Option<DockerNetworkSettings>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DockerPort { PrivatePort: u16, PublicPort: Option<u16>, Type: String }
#[derive(Debug, Clone, serde::Serialize)]
struct DockerMount { Type: String, Source: String, Destination: String, Mode: String, RW: bool }
#[derive(Debug, Clone, serde::Serialize)]
struct DockerNetworkSettings { Networks: std::collections::HashMap<String, DockerNetwork> }
#[derive(Debug, Clone, serde::Serialize)]
struct DockerNetwork { IPAddress: String, Gateway: String, NetworkID: String }

#[derive(Debug, Clone, serde::Serialize)]
struct DockerImageSummary {
    Id: Option<String>, RepoTags: Option<Vec<String>>,
    Created: Option<i64>, Size: Option<i64>,
    Labels: Option<std::collections::HashMap<String, String>>,
}

impl DockerApiServer {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            state: Arc::new(Mutex::new(ApiState {
                containers: Vec::new(), images: Vec::new(), running: false,
            })),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        let listener = tokio::net::UnixListener::bind(&self.socket_path)
            .map_err(|e| CrushError::ApiError(e.to_string()))?;
        {
            let mut s = self.state.lock().await;
            s.running = true;
        }
        let state = self.state.clone();
        tokio::spawn(async move {
            loop {
                if !state.lock().await.running { break; }
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let st = state.clone();
                        tokio::spawn(async move { handle_api_connection(stream, st).await.ok(); });
                    }
                    Err(e) => { eprintln!("API error: {}", e); break; }
                }
            }
        });
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.state.lock().await.running = false;
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        Ok(())
    }
}

async fn handle_api_connection(
    mut stream: tokio::net::UnixStream, state: Arc<Mutex<ApiState>>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await.map_err(|e| CrushError::ApiError(e.to_string()))?;
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 { return Ok(()); }
    let method = parts[0];
    let path = parts[1];

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).await.map_err(|e| CrushError::ApiError(e.to_string()))?;
        if header.trim().is_empty() { break; }
        if header.to_lowercase().starts_with("content-length:") {
            content_length = header.split(':').nth(1).unwrap_or("0").trim().parse().unwrap_or(0);
        }
    }
    // ⚠ Protect against OOM via Content-Length attacks. Cap at 64MB.
    const MAX_BODY: usize = 64 * 1024 * 1024;
    if content_length > MAX_BODY {
        let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n";
        use tokio::io::AsyncWriteExt;
        stream.write_all(resp.as_bytes()).await.ok();
        return Ok(());
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 { reader.read_exact(&mut body).await.map_err(|e| CrushError::ApiError(e.to_string()))?; }

    let (status, resp_body) = route(&state, method, path, &body).await;
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        status, resp_body.len(), resp_body
    );
    stream.write_all(response.as_bytes()).await.map_err(|e| CrushError::ApiError(e.to_string()))?;
    Ok(())
}

async fn route(state: &Arc<Mutex<ApiState>>, method: &str, path: &str, _body: &[u8]) -> (&'static str, String) {
    let p = path.trim_start_matches(&format!("/{}", DOCKER_API_VERSION));

    match (method, p) {
        ("GET", "/_ping") => ("200 OK", "\"OK\"".to_string()),
        ("GET", "/version") => ("200 OK", r#"{"Version":"0.1.0-crush","ApiVersion":"v1.41","Os":"linux","Arch":"amd64","KernelVersion":"crush-0.1"}"#.to_string()),
        ("GET", "/info") => ("200 OK", r#"{"ID":"crush","Containers":0,"ContainersRunning":0,"Images":0,"Driver":"crush","DriverStatus":[],"DockerRootDir":"/var/lib/crush","OperatingSystem":"Crush 0.1","OSType":"crush","Architecture":"amd64","NCPU":4,"MemTotal":8000000000,"Name":"crush","ServerVersion":"0.1.0","SecurityOptions":["name=seccomp"]}"#.to_string()),
        ("GET", _) if p == "/containers/json" || p.starts_with("/containers/json") => {
            let s = state.lock().await;
            ("200 OK", serde_json::to_string(&s.containers).unwrap_or_else(|_| "[]".to_string()))
        }
        ("GET", _) if p.starts_with("/containers/") && p.ends_with("/json") => ("200 OK", r#"{"Id":"test","State":{"Status":"running","Running":true,"Pid":7171}}"#.to_string()),
        ("GET", "/images/json") => ("200 OK", r#"[]"#.to_string()),
        ("POST", "/containers/create") => ("201 Created", r#"{"Id":"created","Warnings":[]}"#.to_string()),
        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/start") => ("204", String::new()),
        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/stop") => ("204", String::new()),
        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/restart") => ("204", String::new()),
        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/kill") => ("204", String::new()),
        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/pause") => ("204", String::new()),
        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/unpause") => ("204", String::new()),
        ("DELETE", _) if p.starts_with("/containers/") => ("204", String::new()),
        ("GET", _) if p.starts_with("/containers/") && p.ends_with("/logs") => ("200 OK", String::new()),
        ("GET", _) if p.starts_with("/containers/") && p.ends_with("/stats") => ("200 OK", r#"{}"#.to_string()),
        ("POST", "/networks/create") => ("201 Created", r#"{"Id":"net-created","Warnings":[]}"#.to_string()),
        ("GET", "/networks") => ("200 OK", "[]".to_string()),
        ("DELETE", _) if p.starts_with("/networks/") => ("204", String::new()),
        ("POST", "/volumes/create") => ("201 Created", r#"{"Name":"created","Mountpoint":"/var/lib/crush/volumes/created"}"#.to_string()),
        ("GET", "/volumes") => ("200 OK", r#"{"Volumes":[]}"#.to_string()),
        ("DELETE", _) if p.starts_with("/volumes/") => ("204", String::new()),
        ("POST", "/auth") => ("200 OK", r#"{"Status":"Login Succeeded"}"#.to_string()),
        ("GET", "/events") => ("200 OK", String::new()),
        ("POST", "/build") => ("200 OK", String::new()),
        _ => ("404", r#"{"message":"not found"}"#.to_string()),
    }
}
