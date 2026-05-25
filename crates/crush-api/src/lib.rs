use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError};

struct ApiState {
    running: bool,
}

pub struct ApiServer {
    socket_path: PathBuf,
    listener: Option<tokio::net::UnixListener>,
    state: Arc<Mutex<ApiState>>,
}

impl ApiServer {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            listener: None,
            state: Arc::new(Mutex::new(ApiState { running: false })),
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
                        tokio::spawn(async move {
                            handle_connection(stream).await.ok();
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

async fn handle_connection(mut stream: tokio::net::UnixStream) -> Result<()> {
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

    let (status, response_body) = route_request(method, path, &body);

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        status, response_body.len(), response_body
    );

    stream.write_all(response.as_bytes()).await
        .map_err(|e| CrushError::ApiError(format!("Write error: {}", e)))?;

    Ok(())
}

fn route_request(method: &str, path: &str, _body: &[u8]) -> (&'static str, String) {
    match (method, path) {
        ("GET", "/containers/json") => {
            ("200 OK", r#"[]"#.to_string())
        }
        ("GET", path) if path.starts_with("/containers/") && path.ends_with("/json") => {
            let id = path.split('/').nth(2).unwrap_or("unknown");
            ("200 OK", format!(r#"{{"Id":"{}","State":"Running","PID":7171}}"#, id))
        }
        ("GET", "/images/json") => {
            ("200 OK", r#"[]"#.to_string())
        }
        ("POST", "/containers/create") => {
            ("201 Created", r#"{"Id":"created-container-id","Warnings":[]}"#.to_string())
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/start") => {
            ("204 No Content", String::new())
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/stop") => {
            ("204 No Content", String::new())
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/pause") => {
            ("204 No Content", String::new())
        }
        ("POST", path) if path.starts_with("/containers/") && path.ends_with("/unpause") => {
            ("204 No Content", String::new())
        }
        ("DELETE", path) if path.starts_with("/containers/") => {
            ("204 No Content", String::new())
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
