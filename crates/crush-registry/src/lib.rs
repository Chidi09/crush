pub mod auth;

use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError};
use serde::{Serialize, Deserialize};
use auth::AuthHandler;

#[derive(Clone)]
pub struct RegistryClientHandle {
    inner: Arc<Mutex<RegistryClient>>,
}

impl Default for RegistryClientHandle {
    fn default() -> Self {
        Self::new(None)
    }
}

impl RegistryClientHandle {
    pub fn new(config_path: Option<PathBuf>) -> Self {
        let client = RegistryClient::new(config_path);
        Self { inner: Arc::new(Mutex::new(client)) }
    }

    pub async fn fetch_manifest(&self, registry: &str, image: &str, reference: &str) -> Result<serde_json::Value> {
        let mut client = self.inner.lock().await;
        client.fetch_manifest_inner(registry, image, reference).await
    }

    pub async fn fetch_blob(&self, registry: &str, image: &str, digest: &str) -> Result<Vec<u8>> {
        let client = self.inner.lock().await;
        client.fetch_blob_inner(registry, image, digest).await
    }

    pub async fn upload_blob(&self, registry: &str, image: &str, file_path: &PathBuf) -> Result<String> {
        let client = self.inner.lock().await;
        client.upload_blob_inner(registry, image, file_path).await
    }

    pub async fn put_manifest(&self, registry: &str, image: &str, reference: &str, manifest: &str) -> Result<()> {
        let client = self.inner.lock().await;
        client.put_manifest_inner(registry, image, reference, manifest).await
    }
}

pub struct RegistryClient {
    http: reqwest::Client,
    auth: AuthHandler,
}

impl RegistryClient {
    pub fn new(config_path: Option<PathBuf>) -> Self {
        let mut auth = AuthHandler::new();
        if let Some(path) = config_path {
            let docker_config = path.join("config.json");
            if docker_config.exists() {
                auth.load_docker_config(&docker_config).ok();
            }
            let legacy_config = path.join("config.json");
            if legacy_config.exists() {
                auth.load_docker_config(&legacy_config).ok();
            }
        }

        Self {
            http: reqwest::Client::builder()
                .user_agent("crush/0.1.0")
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to build HTTP client"),
            auth,
        }
    }

    fn base_url(registry: &str) -> String {
        let r = registry.trim_end_matches('/');
        if r.starts_with("http") {
            format!("{}/v2", r.trim_end_matches("/v2"))
        } else if r.contains("localhost") || r.contains(':') {
            format!("http://{}/v2", r)
        } else {
            format!("https://{}/v2", r)
        }
    }

    async fn ensure_auth(&mut self, registry: &str, image: &str) -> Result<()> {
        if self.auth.get_auth_header(registry).is_some() {
            return Ok(());
        }
        // Pre-auth for Docker Hub using known token endpoint
        if AuthHandler::detect_registry_type(registry) == "dockerhub" {
            let http = self.http.clone();
            let token = self.auth.authenticate_dockerhub(&http, image).await?;
            self.auth.store_token(registry, token);
        }
        Ok(())
    }

    async fn fetch_bearer_token(&self, www_auth: &str) -> Result<String> {
        // Parse: Bearer realm="https://...",service="...",scope="..."
        let params = www_auth.trim_start_matches("Bearer ");
        let mut realm = String::new();
        let mut service = String::new();
        let mut scope = String::new();

        for part in params.split(',') {
            let part = part.trim();
            if let Some(v) = part.strip_prefix("realm=\"") {
                realm = v.trim_end_matches('"').to_string();
            } else if let Some(v) = part.strip_prefix("service=\"") {
                service = v.trim_end_matches('"').to_string();
            } else if let Some(v) = part.strip_prefix("scope=\"") {
                scope = v.trim_end_matches('"').to_string();
            }
        }

        if realm.is_empty() {
            return Err(CrushError::ImageError("No realm in WWW-Authenticate".to_string()));
        }

        let url = format!("{}?service={}&scope={}",
            realm,
            url::form_urlencoded::byte_serialize(service.as_bytes()).collect::<String>(),
            url::form_urlencoded::byte_serialize(scope.as_bytes()).collect::<String>(),
        );
        let resp = self.http.get(&url).send().await
            .map_err(|e| CrushError::ImageError(format!("Token endpoint failed: {}", e)))?;
        let json: serde_json::Value = resp.json().await
            .map_err(|e| CrushError::ImageError(format!("Token response parse failed: {}", e)))?;

        let token = json["token"].as_str()
            .or_else(|| json["access_token"].as_str())
            .unwrap_or("")
            .to_string();

        if token.is_empty() {
            Err(CrushError::ImageError("Empty token in auth response".to_string()))
        } else {
            Ok(token)
        }
    }

    fn build_request(&self, url: &str, registry: &str) -> reqwest::RequestBuilder {
        let req = self.http.get(url);
        if let Some(auth_header) = self.auth.get_auth_header(registry) {
            req.header("Authorization", auth_header)
        } else {
            req
        }
    }

    fn accept_header() -> &'static str {
        "application/vnd.oci.image.manifest.v1+json, \
         application/vnd.docker.distribution.manifest.v2+json, \
         application/vnd.docker.distribution.manifest.list.v2+json, \
         application/vnd.oci.image.index.v1+json"
    }

    pub async fn fetch_manifest_inner(&mut self, registry: &str, image: &str, reference: &str) -> Result<serde_json::Value> {
        self.ensure_auth(registry, image).await?;

        let url = format!("{}/{}/manifests/{}", Self::base_url(registry), image, reference);
        let resp = self.build_request(&url, registry)
            .header("Accept", Self::accept_header())
            .send()
            .await
            .map_err(|e| CrushError::ImageError(format!("Manifest fetch failed: {}", e)))?;

        // Standard OCI auth: if 401, parse WWW-Authenticate, fetch bearer token, retry
        if resp.status().as_u16() == 401 {
            let www_auth = resp.headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            if let Some(www_auth) = www_auth {
                if let Ok(token) = self.fetch_bearer_token(&www_auth).await {
                    self.auth.store_token(registry, token);
                    let resp2 = self.build_request(&url, registry)
                        .header("Accept", Self::accept_header())
                        .send()
                        .await
                        .map_err(|e| CrushError::ImageError(format!("Manifest fetch failed: {}", e)))?;
                    if resp2.status().is_success() {
                        return resp2.json().await
                            .map_err(|e| CrushError::ImageError(format!("Manifest parse failed: {}", e)));
                    }
                    return Err(CrushError::ImageError(format!(
                        "Registry returned HTTP {} after auth retry", resp2.status()
                    )));
                }
            }
            return Err(CrushError::ImageError(format!(
                "Authentication required for {}/{}. Try `crush registry login` first.", registry, image
            )));
        }

        if !resp.status().is_success() {
            return Err(CrushError::ImageError(format!(
                "Registry returned HTTP {} for manifest", resp.status()
            )));
        }

        resp.json().await
            .map_err(|e| CrushError::ImageError(format!("Manifest parse failed: {}", e)))
    }

    pub async fn fetch_blob_inner(&self, registry: &str, image: &str, digest: &str) -> Result<Vec<u8>> {
        let url = format!("{}/{}/blobs/{}", Self::base_url(registry), image, digest);
        let resp = self.build_request(&url, registry)
            .send()
            .await
            .map_err(|e| CrushError::ImageError(format!("Blob fetch failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(CrushError::ImageError(format!(
                "Registry returned HTTP {} for blob", resp.status()
            )));
        }

        let bytes = resp.bytes().await
            .map_err(|e| CrushError::ImageError(format!("Blob read failed: {}", e)))?;
        Ok(bytes.to_vec())
    }

    pub async fn upload_blob_inner(&self, registry: &str, image: &str, file_path: &PathBuf) -> Result<String> {
        use sha2::{Sha256, Digest};
        let data = tokio::fs::read(file_path).await
            .map_err(|e| CrushError::StorageError(format!("Failed to read blob file: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let digest = format!("sha256:{}", hex::encode(hasher.finalize()));

        let base = Self::base_url(registry);
        let session_url = format!("{}/{}/blobs/uploads/", base, image);
        let session_resp = self.build_request(&session_url, registry)
            .send()
            .await
            .map_err(|e| CrushError::ImageError(format!("Upload session failed: {}", e)))?;

        let upload_url = session_resp.headers().get("Location")
            .and_then(|v| v.to_str().ok())
            .map(|s| {
                if s.starts_with("http") { s.to_string() }
                else { format!("{}{}", base.trim_end_matches("/v2"), s) }
            })
            .unwrap_or_else(|| format!("{}/{}/blobs/uploads/", base, image));

        let final_url = format!("{}?digest={}", upload_url.trim_end_matches('/'), digest);
        let put_resp = self.http.put(&final_url)
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await
            .map_err(|e| CrushError::ImageError(format!("Blob upload failed: {}", e)))?;

        if !put_resp.status().is_success() {
            return Err(CrushError::ImageError(format!("Blob upload returned HTTP {}", put_resp.status())));
        }

        Ok(digest)
    }

    pub async fn put_manifest_inner(&self, registry: &str, image: &str, reference: &str, manifest: &str) -> Result<()> {
        let url = format!("{}/{}/manifests/{}", Self::base_url(registry), image, reference);
        let resp = self.http.put(&url)
            .header("Content-Type", "application/vnd.docker.distribution.manifest.v2+json")
            .body(manifest.to_string())
            .send()
            .await
            .map_err(|e| CrushError::ImageError(format!("Manifest push failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(CrushError::ImageError(format!("Manifest push returned HTTP {}", resp.status())));
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredBlob {
    digest: String,
    data: Vec<u8>,
    media_type: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredManifest {
    reference: String,
    content: String,
}

pub struct LocalRegistryServer {
    port: u16,
    data_dir: PathBuf,
    blobs: Arc<Mutex<HashMap<String, StoredBlob>>>,
    manifests: Arc<Mutex<HashMap<String, StoredManifest>>>,
}

impl LocalRegistryServer {
    pub fn new(port: u16) -> Self {
        let data_dir = std::env::temp_dir().join("crush_registry").join(port.to_string());
        std::fs::create_dir_all(&data_dir).ok();
        Self {
            port,
            data_dir,
            blobs: Arc::new(Mutex::new(HashMap::new())),
            manifests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| CrushError::NetworkError(format!("Failed to bind registry: {}", e)))?;

        let blobs = self.blobs.clone();
        let manifests = self.manifests.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let b = blobs.clone();
                        let m = manifests.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, b, m).await.ok();
                        });
                    }
                    Err(e) => {
                        eprintln!("Registry accept error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    blobs: Arc<Mutex<HashMap<String, StoredBlob>>>,
    manifests: Arc<Mutex<HashMap<String, StoredManifest>>>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await
        .map_err(|e| CrushError::ApiError(format!("Read error: {}", e)))?;
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 { return Ok(()); }
    let method = parts[0];
    let path = parts[1];

    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).await
            .map_err(|e| CrushError::ApiError(format!("Header read error: {}", e)))?;
        let trimmed = header.trim();
        if trimmed.is_empty() { break; }
        if let Some(lower) = trimmed.to_lowercase().strip_prefix("content-length:") {
            content_length = lower.trim().parse().unwrap_or(0);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await
            .map_err(|e| CrushError::ApiError(format!("Body read error: {}", e)))?;
    }

    let (status, response_body) = handle_registry_request(method, path, &body, &blobs, &manifests).await;
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        status, response_body.len(), response_body
    );
    stream.write_all(response.as_bytes()).await
        .map_err(|e| CrushError::ApiError(format!("Write error: {}", e)))?;
    Ok(())
}

async fn handle_registry_request(
    method: &str, path: &str, body: &[u8],
    blobs: &Arc<Mutex<HashMap<String, StoredBlob>>>,
    manifests: &Arc<Mutex<HashMap<String, StoredManifest>>>,
) -> (&'static str, String) {
    match (method, path) {
        ("GET", p) if p.contains("/v2/") && p.contains("/blobs/") => {
            let guard = blobs.lock().await;
            let digest = p.split("/blobs/").nth(1).unwrap_or("");
            if let Some(blob) = guard.get(digest) {
                ("200 OK", String::from_utf8_lossy(&blob.data).to_string())
            } else {
                ("404 Not Found", r#"{"errors":[{"code":"BLOB_UNKNOWN"}]}"#.to_string())
            }
        }
        ("POST", p) if p.contains("/v2/") && p.contains("/blobs/uploads/") => {
            ("202 Accepted", r#"{}"#.to_string())
        }
        ("PUT", p) if p.contains("/v2/") && p.contains("/manifests/") => {
            let reference = p.split("/manifests/").nth(1).unwrap_or("latest");
            let mut m = manifests.lock().await;
            m.insert(reference.to_string(), StoredManifest {
                reference: reference.to_string(),
                content: String::from_utf8_lossy(body).to_string(),
            });
            ("201 Created", r#"{}"#.to_string())
        }
        ("GET", "/v2/_catalog") => {
            ("200 OK", r#"{"repositories":[]}"#.to_string())
        }
        _ => {
            ("404 Not Found", r#"{"errors":[{"code":"UNSUPPORTED"}]}"#.to_string())
        }
    }
}
