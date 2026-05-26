use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError, Container, ContainerStatus, RuntimeBackend, StorageBackend};
use crush_image::db::ImageDatabase;

#[cfg(unix)]
use libc;
pub struct ApiServer {
    socket_path: PathBuf,
    data_dir: PathBuf,
    backend: Arc<dyn RuntimeBackend>,
    running: Arc<Mutex<bool>>,
}

impl ApiServer {
    pub fn new(socket_path: PathBuf, data_dir: PathBuf, backend: Arc<dyn RuntimeBackend>) -> Self {
        Self {
            socket_path,
            data_dir,
            backend,
            running: Arc::new(Mutex::new(false)),
        }
    }

    #[cfg(unix)]
    pub async fn serve_unix_socket(&self) -> Result<()> {
        let _ = tokio::fs::remove_file(&self.socket_path).await;

        let listener = tokio::net::UnixListener::bind(&self.socket_path)
            .map_err(|e| CrushError::ApiError(format!("Failed to bind Unix socket: {}", e)))?;

        {
            let mut running = self.running.lock().await;
            *running = true;
        }

        let running = self.running.clone();
        let data_dir = self.data_dir.clone();
        let backend = self.backend.clone();

        tokio::spawn(async move {
            loop {
                let is_running = { *running.lock().await };
                if !is_running {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, _)) => {
                        let dir = data_dir.clone();
                        let b = backend.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, dir, b).await.ok();
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

    #[cfg(windows)]
    pub async fn serve_named_pipe(&self) -> Result<()> {
        use tokio::net::windows::named_pipe::ServerOptions;

        // Convert any path to a named pipe name: \\.\pipe\crush-api
        let pipe_name = format!("\\\\.\\pipe\\crush-api");

        {
            let mut running = self.running.lock().await;
            *running = true;
        }

        let running = self.running.clone();
        let data_dir = self.data_dir.clone();
        let backend = self.backend.clone();

        tokio::spawn(async move {
            loop {
                let is_running = { *running.lock().await };
                if !is_running { break; }

                // Each accept cycle creates a new server instance
                let server = match ServerOptions::new()
                    .first_pipe_instance(false)
                    .create(&pipe_name)
                {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("API named pipe create error: {}", e);
                        break;
                    }
                };

                if server.connect().await.is_err() {
                    continue;
                }

                let dir = data_dir.clone();
                let b = backend.clone();
                tokio::spawn(async move {
                    handle_connection(server, dir, b).await.ok();
                });
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        {
            let mut running = self.running.lock().await;
            *running = false;
        }
        let _ = tokio::fs::remove_file(&self.socket_path).await;
        Ok(())
    }
}

async fn handle_connection<S>(
    mut stream: S,
    data_dir: PathBuf,
    backend: Arc<dyn RuntimeBackend>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};

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

    let (status, response_body) = route_request(&data_dir, &backend, method, path, &body).await;

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        status, response_body.len(), response_body
    );

    stream.write_all(response.as_bytes()).await
        .map_err(|e| CrushError::ApiError(format!("Write error: {}", e)))?;

    Ok(())
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
    data_dir: &PathBuf,
    backend: &Arc<dyn RuntimeBackend>,
    method: &str,
    path: &str,
    body: &[u8],
) -> (&'static str, String) {
    let path = path.trim_start_matches("/v1.41");

    match (method, path) {
        ("GET", "/containers/json") => {
            let mut summaries: Vec<serde_json::Value> = Vec::new();
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

                                    let status_str = match c.status {
                                        ContainerStatus::Running => "running",
                                        ContainerStatus::Paused => "paused",
                                        ContainerStatus::Stopped => "exited",
                                        _ => "created",
                                    };
                                    summaries.push(serde_json::json!({
                                        "Id": c.id,
                                        "Names": vec![format!("/{}", c.name)],
                                        "Image": c.image,
                                        "State": status_str,
                                        "Status": format!("Up 10 seconds"),
                                        "Created": c.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64,
                                        "Ports": c.ports.iter().map(|p| {
                                            serde_json::json!({
                                                "PrivatePort": p.container_port,
                                                "PublicPort": p.host_port,
                                                "Type": "tcp"
                                            })
                                        }).collect::<Vec<_>>(),
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            ("200 OK", serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string()))
        }
        ("GET", path) if path.starts_with("/containers/") && path.ends_with("/json") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", r#"{"message":"Invalid path"}"#.to_string());
            }
            let id = parts[2];
            let containers_dir = data_dir.join("containers");
            let mut found = None;
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    if c.id == id || c.name == id {
                                        found = Some(c);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(c) = found {
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
            
            let img_tag = req_body.image.clone().unwrap_or_else(|| "unknown".to_string());
            
            let mut effective_cmd = req_body.cmd.unwrap_or_default();
            let mut env_vars = req_body.env.unwrap_or_default();

            if let Ok(db) = ImageDatabase::new(&data_dir.join("images")) {
                if let Ok(Some(img)) = db.get_image_by_tag(&img_tag).await {
                    if effective_cmd.is_empty() {
                        if !img.entrypoint.is_empty() {
                            effective_cmd = img.entrypoint.clone();
                            effective_cmd.extend(img.cmd.iter().cloned());
                        } else if !img.cmd.is_empty() {
                            effective_cmd = img.cmd.clone();
                        }
                    }
                    if env_vars.is_empty() {
                        env_vars = img.env.clone();
                    }
                    
                    let rootfs = data_dir.join("containers").join(&id).join("rootfs");
                    let _ = std::fs::create_dir_all(&rootfs);
                    if let Ok(store) = crush_image::ImageStore::new(data_dir.clone()).await {
                        let _ = store.extract_layers(&img.id, &rootfs).await;
                    }
                }
            }

            if effective_cmd.is_empty() {
                effective_cmd = vec!["/bin/sh".to_string()];
            }

            let container = Container {
                id: id.clone(),
                name: format!("crush_{}", &id[..8]),
                image: img_tag,
                status: ContainerStatus::Creating,
                pid: None,
                created_at: std::time::SystemTime::now(),
                started_at: None,
                ports: Vec::new(),
                mounts: Vec::new(),
                memory_limit_bytes: None,
                cpu_shares: None,
                health: None,
                restart_count: None,
                restart_policy: None,
                health_cmd: None,
                health_interval: None,
                health_timeout: None,
                health_retries: None,
                pids_limit: None,
                read_only: None,
                security_opt: None,
            };

            let container_dir = data_dir.join("containers").join(&id);
            if let Err(e) = backend.create(&container, &container_dir).await {
                return ("500 Internal Server Error", format!(r#"{{"message":"{}"}}"#, e));
            }

            // Save config.json for internal-run to pick up
            let config_json = serde_json::json!({
                "cmd": effective_cmd,
                "env": env_vars,
            });
            if let Ok(serialized) = serde_json::to_string_pretty(&config_json) {
                let _ = std::fs::write(container_dir.join("config.json"), serialized);
            }

            ("201 Created", format!(r#"{{"Id":"{}","Warnings":[]}}"#, id))
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/start") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 3 {
                return ("404 Not Found", String::new());
            }
            let id = parts[2];
            let containers_dir = data_dir.join("containers");
            let mut found = None;
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    if c.id == id || c.name == id {
                                        found = Some(c.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(actual_id) = found {
                match backend.start(&actual_id).await {
                    Ok(_) => ("204 No Content", String::new()),
                    Err(_) => ("500 Internal Server Error", String::new()),
                }
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
            let containers_dir = data_dir.join("containers");
            let mut found = None;
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    if c.id == id || c.name == id {
                                        found = Some(c.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(actual_id) = found {
                match backend.stop(&actual_id, 10).await {
                    Ok(_) => ("204 No Content", String::new()),
                    Err(_) => ("500 Internal Server Error", String::new()),
                }
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
            let containers_dir = data_dir.join("containers");
            let mut found = None;
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    if c.id == id || c.name == id {
                                        found = Some(c.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(actual_id) = found {
                match backend.pause(&actual_id).await {
                    Ok(_) => ("204 No Content", String::new()),
                    Err(_) => ("500 Internal Server Error", String::new()),
                }
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
            let containers_dir = data_dir.join("containers");
            let mut found = None;
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    if c.id == id || c.name == id {
                                        found = Some(c.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(actual_id) = found {
                match backend.resume(&actual_id).await {
                    Ok(_) => ("204 No Content", String::new()),
                    Err(_) => ("500 Internal Server Error", String::new()),
                }
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
            let containers_dir = data_dir.join("containers");
            let mut found = None;
            if containers_dir.exists() {
                if let Ok(mut entries) = std::fs::read_dir(&containers_dir) {
                    while let Some(Ok(entry)) = entries.next() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    if c.id == id || c.name == id {
                                        found = Some(c.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(actual_id) = found {
                match backend.delete(&actual_id).await {
                    Ok(_) => ("204 No Content", String::new()),
                    Err(_) => ("500 Internal Server Error", String::new()),
                }
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
