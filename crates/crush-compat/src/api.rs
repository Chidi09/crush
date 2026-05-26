use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncReadExt;
use crush_types::{Result, CrushError, Container, ContainerStatus, Protocol, PortMapping};

const DOCKER_API_VERSION: &str = "v1.41";

pub struct DockerApiServer {
    socket_path: PathBuf,
    data_dir: PathBuf,
    running: Arc<Mutex<bool>>,
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
    pub fn new(socket_path: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            socket_path,
            data_dir,
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        
        // Ensure socket directory exists
        if let Some(parent) = self.socket_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let listener = tokio::net::UnixListener::bind(&self.socket_path)
            .map_err(|e| CrushError::ApiError(format!("Failed to bind Unix socket: {}", e)))?;
        
        {
            let mut s = self.running.lock().await;
            *s = true;
        }
        
        let running = self.running.clone();
        let data_dir = self.data_dir.clone();
        
        tokio::spawn(async move {
            loop {
                {
                    if !*running.lock().await { break; }
                }
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let dir = data_dir.clone();
                        tokio::spawn(async move { handle_api_connection(stream, dir).await.ok(); });
                    }
                    Err(e) => {
                        eprintln!("API error: {}", e);
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        {
            let mut s = self.running.lock().await;
            *s = false;
        }
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        Ok(())
    }
}

async fn handle_api_connection(
    mut stream: tokio::net::UnixStream, data_dir: PathBuf,
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
    
    const MAX_BODY: usize = 64 * 1024 * 1024;
    if content_length > MAX_BODY {
        let resp = "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(resp.as_bytes()).await.ok();
        return Ok(());
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 { reader.read_exact(&mut body).await.map_err(|e| CrushError::ApiError(e.to_string()))?; }

    let (status, resp_body) = route(&data_dir, method, path, &body).await;
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        status, resp_body.len(), resp_body
    );
    stream.write_all(response.as_bytes()).await.map_err(|e| CrushError::ApiError(e.to_string()))?;
    Ok(())
}

async fn route(data_dir: &PathBuf, method: &str, path: &str, body: &[u8]) -> (&'static str, String) {
    let p = path.trim_start_matches(&format!("/{}", DOCKER_API_VERSION));

    match (method, p) {
        ("GET", "/_ping") => ("200 OK", "\"OK\"".to_string()),
        ("GET", "/version") => ("200 OK", r#"{"Version":"0.1.0-crush","ApiVersion":"v1.41","Os":"linux","Arch":"amd64","KernelVersion":"crush-0.1"}"#.to_string()),
        ("GET", "/info") => ("200 OK", r#"{"ID":"crush","Containers":0,"ContainersRunning":0,"Images":0,"Driver":"crush","DriverStatus":[],"DockerRootDir":"/var/lib/crush","OperatingSystem":"Crush 0.1","OSType":"crush","Architecture":"amd64","NCPU":4,"MemTotal":8000000000,"Name":"crush","ServerVersion":"0.1.0","SecurityOptions":["name=seccomp"]}"#.to_string()),
        
        ("GET", "/images/json") => {
            let mut list = Vec::new();
            if let Ok(db) = crush_image::db::ImageDatabase::new(&data_dir.join("images")) {
                if let Ok(images) = db.list_images().await {
                    for img in images {
                        list.push(DockerImageSummary {
                            Id: Some(format!("sha256:{}", img.id)),
                            RepoTags: Some(vec![img.tag.clone()]),
                            Created: Some(1620000000i64),
                            Size: Some(img.size_bytes as i64),
                            Labels: Some(std::collections::HashMap::new()),
                        });
                    }
                }
            }
            ("200 OK", serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string()))
        }
        
        ("GET", "/containers/json") | ("GET", "/containers/json?all=1") | ("GET", "/containers/json?all=true") => {
            let show_all = p.contains("all=1") || p.contains("all=true");
            let mut list = Vec::new();
            let containers_dir = data_dir.join("containers");
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                                    let mut is_alive = false;
                                    if let Some(pid) = c.pid {
                                        #[cfg(unix)]
                                        {
                                            is_alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
                                        }
                                        #[cfg(not(unix))]
                                        {
                                            is_alive = true;
                                        }
                                    }
                                    if !is_alive && c.status == ContainerStatus::Running {
                                        c.status = ContainerStatus::Stopped;
                                        c.pid = None;
                                        if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                                            let _ = std::fs::write(&json_path, serialized);
                                        }
                                    }

                                    if show_all || c.status == ContainerStatus::Running {
                                        let status_str = match c.status {
                                            ContainerStatus::Running => "running",
                                            ContainerStatus::Paused => "paused",
                                            ContainerStatus::Stopped => "exited",
                                            _ => "created",
                                        };
                                        list.push(DockerContainerSummary {
                                            Id: Some(c.id.clone()),
                                            Names: Some(vec![format!("/{}", c.name)]),
                                            Image: Some(c.image.clone()),
                                            ImageID: Some(c.image.clone()),
                                            State: Some(status_str.to_string()),
                                            Status: Some(format!("Up 10 seconds")),
                                            Created: Some(c.created_at.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs() as i64),
                                            Ports: Some(vec![]),
                                            Mounts: Some(vec![]),
                                            NetworkSettings: Some(DockerNetworkSettings {
                                                Networks: std::collections::HashMap::new(),
                                            }),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ("200 OK", serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string()))
        }

        ("GET", _) if p.starts_with("/containers/") && p.ends_with("/json") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() >= 3 {
                let id = parts[2];
                let containers_dir = data_dir.join("containers");
                if containers_dir.exists() {
                    if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                        while let Some(Ok(entry)) = entries.next() {
                            let json_path = entry.path().join("container.json");
                            if json_path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&json_path) {
                                    if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                        if c.id == id || c.name == id {
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
                                            return ("200 OK", resp.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ("404 Not Found", r#"{"message":"Container not found"}"#.to_string())
        }

        ("POST", "/containers/create") => {
            #[derive(serde::Deserialize)]
            struct CreateBody {
                #[serde(rename = "Image")]
                image: Option<String>,
            }
            let req_body: CreateBody = serde_json::from_slice(body)
                .unwrap_or(CreateBody { image: None });

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

            let container_dir = data_dir.join("containers").join(&id);
            let _ = std::fs::create_dir_all(&container_dir);
            let json_path = container_dir.join("container.json");
            if let Ok(serialized) = serde_json::to_string_pretty(&container) {
                let _ = std::fs::write(&json_path, serialized);
            }

            ("201 Created", format!(r#"{{"Id":"{}","Warnings":[]}}"#, id))
        }

        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/start") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() >= 3 {
                let id = parts[2];
                let containers_dir = data_dir.join("containers");
                if containers_dir.exists() {
                    if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                        while let Some(Ok(entry)) = entries.next() {
                            let json_path = entry.path().join("container.json");
                            if json_path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&json_path) {
                                    if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                                        if c.id == id || c.name == id {
                                            c.status = ContainerStatus::Running;
                                            c.pid = Some(rand_pid());
                                            c.started_at = Some(std::time::SystemTime::now());
                                            if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                                                let _ = std::fs::write(&json_path, serialized);
                                            }
                                            return ("204 No Content", String::new());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ("404 Not Found", String::new())
        }

        ("POST", _) if p.starts_with("/containers/") && p.ends_with("/stop") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() >= 3 {
                let id = parts[2];
                let containers_dir = data_dir.join("containers");
                if containers_dir.exists() {
                    if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                        while let Some(Ok(entry)) = entries.next() {
                            let json_path = entry.path().join("container.json");
                            if json_path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&json_path) {
                                    if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                                        if c.id == id || c.name == id {
                                            if let Some(pid) = c.pid {
                                                #[cfg(unix)]
                                                {
                                                    unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL); }
                                                }
                                            }
                                            c.status = ContainerStatus::Stopped;
                                            c.pid = None;
                                            if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                                                let _ = std::fs::write(&json_path, serialized);
                                            }
                                            return ("204 No Content", String::new());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ("404 Not Found", String::new())
        }

        ("DELETE", _) if p.starts_with("/containers/") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() >= 3 {
                let id = parts[2];
                let containers_dir = data_dir.join("containers");
                if containers_dir.exists() {
                    if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                        while let Some(Ok(entry)) = entries.next() {
                            let json_path = entry.path().join("container.json");
                            if json_path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&json_path) {
                                    if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                        if c.id == id || c.name == id {
                                            let _ = std::fs::remove_dir_all(entry.path());
                                            return ("204 No Content", String::new());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ("404 Not Found", String::new())
        }

        ("GET", _) if p.starts_with("/containers/") && p.ends_with("/logs") => {
            let parts: Vec<&str> = p.split('/').collect();
            if parts.len() >= 3 {
                let id = parts[2];
                let containers_dir = data_dir.join("containers");
                if containers_dir.exists() {
                    if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                        while let Some(Ok(entry)) = entries.next() {
                            let json_path = entry.path().join("container.json");
                            if json_path.exists() {
                                if let Ok(content) = std::fs::read_to_string(&json_path) {
                                    if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                        if c.id == id || c.name == id {
                                            let stdout_path = entry.path().join("stdout.log");
                                            let logs = std::fs::read_to_string(stdout_path).unwrap_or_default();
                                            return ("200 OK", logs);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ("200 OK", String::new())
        }

        _ => ("404 Not Found", r#"{"message":"not found"}"#.to_string()),
    }
}

fn rand_pid() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (nanos as u32 % 65535) + 1
}
