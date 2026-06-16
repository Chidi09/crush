use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use tauri::{command, State};
use crate::state::AppState;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Connection {
    pub name: String,
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub path_style: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BucketInfo {
    pub name: String,
    pub created_at: Option<i64>, // epoch ms
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectInfo {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<i64>, // epoch ms
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<i64>,
    pub content_type: Option<String>,
    pub metadata: HashMap<String, String>,
}

fn guess_mime_type(path: &str) -> &'static str {
    let ext = path.split('.').last().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

// Client helper
pub async fn make_s3_client(conn: &S3Connection) -> aws_sdk_s3::Client {
    let creds = Credentials::new(
        conn.access_key.clone(),
        conn.secret_key.clone(),
        None,
        None,
        "crush-storage",
    );
    let mut builder = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(conn.region.clone()))
        .credentials_provider(creds)
        .force_path_style(conn.path_style);

    if !conn.endpoint.is_empty() {
        builder = builder.endpoint_url(conn.endpoint.clone());
    }

    aws_sdk_s3::Client::from_conf(builder.build())
}

use std::sync::OnceLock;

static SWEEPER_SPAWNED: OnceLock<()> = OnceLock::new();

fn ensure_sweeper(data_dir: std::path::PathBuf) {
    if SWEEPER_SPAWNED.get().is_none() {
        if SWEEPER_SPAWNED.set(()).is_ok() {
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;
                loop {
                    let mut conns = Vec::new();
                    let path = data_dir.join("storage_connections.json");
                    if path.exists() {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(connections) = serde_json::from_str::<Vec<S3Connection>>(&content) {
                                conns = connections;
                            }
                        }
                    } else {
                        let config_path = data_dir.join("config.json");
                        let minio_port = if config_path.exists() {
                            if let Ok(content) = fs::read_to_string(&config_path) {
                                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                                    config.get("minio_port")
                                        .and_then(|v| v.as_u64())
                                        .map(|v| v as u16)
                                        .unwrap_or(9000)
                                } else { 9000 }
                            } else { 9000 }
                        } else { 9000 };
                        conns.push(S3Connection {
                            name: "Local MinIO".to_string(),
                            endpoint: format!("http://127.0.0.1:{minio_port}"),
                            region: "us-east-1".to_string(),
                            access_key: "minioadmin".to_string(),
                            secret_key: "minioadmin".to_string(),
                            path_style: true,
                        });
                    }
                    
                    for conn in conns {
                        let client = make_s3_client(&conn).await;
                        if let Ok(resp) = client.list_buckets().send().await {
                            for b in resp.buckets() {
                                if let Some(bucket_name) = b.name() {
                                    // List in-progress multipart uploads (single page is
                                    // sufficient for periodic cleanup; AWS returns up to
                                    // 1000 and we re-run on the next sweep).
                                    if let Ok(page) = client.list_multipart_uploads().bucket(bucket_name).send().await {
                                        for upload in page.uploads() {
                                            if let (Some(uid), Some(key), Some(initiated)) = (upload.upload_id(), upload.key(), upload.initiated()) {
                                                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                                if now > (initiated.secs() as u64) && now - (initiated.secs() as u64) > 24 * 3600 {
                                                    let _ = client.abort_multipart_upload()
                                                        .bucket(bucket_name)
                                                        .key(key)
                                                        .upload_id(uid)
                                                        .send()
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    tokio::time::sleep(std::time::Duration::from_secs(24 * 3600)).await;
                }
            });
        }
    }
}

// Connection management
#[command]
pub async fn storage_get_connections(state: State<'_, AppState>) -> Result<Vec<S3Connection>, String> {
    ensure_sweeper(state.data_dir.clone());
    let path = state.data_dir.join("storage_connections.json");
    if !path.exists() {
        // Find local minio port
        let config_path = state.data_dir.join("config.json");
        let minio_port = if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    config.get("minio_port")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u16)
                        .unwrap_or(9000)
                } else {
                    9000
                }
            } else {
                9000
            }
        } else {
            9000
        };

        let default_minio = S3Connection {
            name: "Local MinIO".to_string(),
            endpoint: format!("http://127.0.0.1:{minio_port}"),
            region: "us-east-1".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            path_style: true,
        };
        return Ok(vec![default_minio]);
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let connections: Vec<S3Connection> = serde_json::from_str(&content).unwrap_or_default();
    Ok(connections)
}

#[command]
pub async fn storage_save_connections(
    connections: Vec<S3Connection>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = state.data_dir.join("storage_connections.json");
    let content = serde_json::to_string_pretty(&connections).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

// Buckets
#[command]
pub async fn storage_list_buckets(conn: S3Connection) -> Result<Vec<BucketInfo>, String> {
    let client = make_s3_client(&conn).await;
    let resp = client.list_buckets().send().await.map_err(|e| e.to_string())?;
    let mut list = Vec::new();
    for b in resp.buckets() {
        let name = b.name().unwrap_or_default().to_string();
        let created_at = b.creation_date().map(|d| d.secs() * 1000);
        list.push(BucketInfo { name, created_at });
    }
    Ok(list)
}

#[command]
pub async fn storage_create_bucket(conn: S3Connection, name: String) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    client.create_bucket()
        .bucket(&name)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn storage_delete_bucket(conn: S3Connection, name: String, force: bool) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    if force {
        // List and delete all objects first
        let mut list_stream = client.list_objects_v2().bucket(&name).into_paginator().send();
        while let Some(page) = list_stream.next().await {
            let page = page.map_err(|e| e.to_string())?;
            let contents = page.contents();
            if !contents.is_empty() {
                let mut delete_builder = client.delete_objects().bucket(&name);
                let mut delete_objects = aws_sdk_s3::types::Delete::builder();
                for obj in contents {
                    if let Some(key) = obj.key() {
                        delete_objects = delete_objects.objects(
                            aws_sdk_s3::types::ObjectIdentifier::builder().key(key).build().unwrap()
                        );
                    }
                }
                delete_builder.delete(delete_objects.build().unwrap())
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    client.delete_bucket()
        .bucket(&name)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Objects
#[command]
pub async fn storage_list_objects(
    conn: S3Connection,
    bucket: String,
    prefix: Option<String>,
) -> Result<Vec<ObjectInfo>, String> {
    let client = make_s3_client(&conn).await;
    let mut builder = client.list_objects_v2().bucket(&bucket);
    if let Some(ref p) = prefix {
        builder = builder.prefix(p);
    }
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    let mut list = Vec::new();
    let contents = resp.contents();
    for obj in contents {
        let key = obj.key().unwrap_or_default().to_string();
        let size = obj.size().unwrap_or(0);
        let last_modified = obj.last_modified().map(|d| d.secs() * 1000);
        list.push(ObjectInfo {
            key,
            size,
            last_modified,
            content_type: None,
        });
    }
    Ok(list)
}

#[command]
pub async fn storage_upload_object(
    conn: S3Connection,
    bucket: String,
    key: String,
    file_path: String,
) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    let meta = tokio::fs::metadata(&file_path).await.map_err(|e| format!("failed to read file metadata: {e}"))?;
    let size = meta.len();
    let mime = guess_mime_type(&file_path);

    if size > 64 * 1024 * 1024 {
        let chunk_size: u64 = 8 * 1024 * 1024; // 8MB
        let num_parts = ((size + chunk_size - 1) / chunk_size) as i32;
        
        let state_dir = dirs::home_dir().unwrap_or_default().join(".crush/s3-state");
        std::fs::create_dir_all(&state_dir).ok();
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        conn.endpoint.hash(&mut hasher);
        bucket.hash(&mut hasher);
        key.hash(&mut hasher);
        file_path.hash(&mut hasher);
        let hash_id = hasher.finish();
        
        let state_file = state_dir.join(format!("{:x}.json", hash_id));
        
        let mut upload_id = None;
        let mut completed_parts_map = std::collections::HashMap::new();
        
        if state_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&state_file) {
                if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(uid) = state.get("upload_id").and_then(|v| v.as_str()) {
                        upload_id = Some(uid.to_string());
                    }
                    if let Some(parts) = state.get("parts").and_then(|v| v.as_array()) {
                        for p in parts {
                            if let (Some(n), Some(etag)) = (p.get("n").and_then(|v| v.as_i64()), p.get("etag").and_then(|v| v.as_str())) {
                                completed_parts_map.insert(n as i32, etag.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        if upload_id.is_none() {
            let res = client.create_multipart_upload()
                .bucket(&bucket)
                .key(&key)
                .content_type(mime)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            upload_id = Some(res.upload_id().unwrap().to_string());
        }
        let uid = upload_id.unwrap();

        let uid_arc = std::sync::Arc::new(uid.clone());
        let bucket_arc = std::sync::Arc::new(bucket.clone());
        let key_arc = std::sync::Arc::new(key.clone());
        let file_path_arc = std::sync::Arc::new(file_path.clone());
        let client_arc = std::sync::Arc::new(client.clone());
        let state_file_arc = std::sync::Arc::new(state_file.clone());

        let mut pending_parts = Vec::new();
        for part_number in 1..=num_parts {
            if !completed_parts_map.contains_key(&part_number) {
                let offset = (part_number - 1) as u64 * chunk_size;
                let current_chunk_size = std::cmp::min(chunk_size, size - offset);
                pending_parts.push((part_number, offset, current_chunk_size));
            }
        }

        let completed_parts_map = std::sync::Arc::new(tokio::sync::Mutex::new(completed_parts_map));

        // Save initial state
        {
            let map = completed_parts_map.lock().await;
            let mut parts_vec = Vec::new();
            for (n, e) in map.iter() {
                parts_vec.push(serde_json::json!({ "n": n, "etag": e }));
            }
            let state = serde_json::json!({
                "upload_id": *uid_arc,
                "bucket": *bucket_arc,
                "key": *key_arc,
                "parts": parts_vec
            });
            let _ = std::fs::write(state_file_arc.as_ref(), serde_json::to_string(&state).unwrap());
        }

        let stream = futures::stream::iter(pending_parts).map(|(part_number, offset, current_chunk_size)| {
            let uid = uid_arc.clone();
            let bucket = bucket_arc.clone();
            let key = key_arc.clone();
            let file_path = file_path_arc.clone();
            let client = client_arc.clone();
            
            async move {
                use tokio::io::{AsyncReadExt, AsyncSeekExt};
                let mut file = tokio::fs::File::open(file_path.as_ref()).await.map_err(|e| format!("part {part_number} file open failed: {e}"))?;
                file.seek(std::io::SeekFrom::Start(offset)).await.map_err(|e| format!("part {part_number} seek failed: {e}"))?;
                let mut buf = vec![0u8; current_chunk_size as usize];
                file.read_exact(&mut buf).await.map_err(|e| format!("part {part_number} read failed: {e}"))?;
                
                let body = ByteStream::from(buf);
                let res = client.upload_part()
                    .bucket(bucket.as_ref())
                    .key(key.as_ref())
                    .upload_id(uid.as_ref())
                    .part_number(part_number)
                    .body(body)
                    .send()
                    .await.map_err(|e| format!("part {} failed: {}", part_number, e))?;
                    
                Ok::<_, String>((part_number, res.e_tag().unwrap_or("").to_string()))
            }
        }).buffer_unordered(4);

        let mut stream = stream;
        while let Some(res) = stream.next().await {
            match res {
                Ok((part_number, etag)) => {
                    let mut map = completed_parts_map.lock().await;
                    map.insert(part_number, etag);
                    
                    let mut parts_vec = Vec::new();
                    for (n, e) in map.iter() {
                        parts_vec.push(serde_json::json!({ "n": n, "etag": e }));
                    }
                    let state = serde_json::json!({
                        "upload_id": *uid_arc,
                        "bucket": *bucket_arc,
                        "key": *key_arc,
                        "parts": parts_vec
                    });
                    let _ = std::fs::write(state_file_arc.as_ref(), serde_json::to_string(&state).unwrap());
                }
                Err(e) => {
                    let _ = client_arc.abort_multipart_upload()
                        .bucket(bucket_arc.as_ref())
                        .key(key_arc.as_ref())
                        .upload_id(uid_arc.as_ref())
                        .send()
                        .await;
                    let _ = std::fs::remove_file(state_file_arc.as_ref());
                    return Err(e);
                }
            }
        }

        let map = completed_parts_map.lock().await;
        let mut completed_parts_vec = Vec::new();
        for i in 1..=num_parts {
            if let Some(etag) = map.get(&i) {
                completed_parts_vec.push(
                    aws_sdk_s3::types::CompletedPart::builder()
                        .e_tag(etag)
                        .part_number(i)
                        .build()
                );
            } else {
                return Err(format!("missing part {}", i));
            }
        }
        
        let multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts_vec))
            .build();
            
        client.complete_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(&uid)
            .multipart_upload(multipart_upload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
            
        let _ = std::fs::remove_file(&state_file);
        Ok(())
    } else {
        let body = ByteStream::from_path(&file_path)
            .await
            .map_err(|e| format!("failed to read file: {e}"))?;
            
        client.put_object()
            .bucket(&bucket)
            .key(&key)
            .content_type(mime)
            .body(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadDirResult {
    pub uploaded: u32,
    pub total_files: u32,
    pub total_bytes: u64,
    pub errors: Vec<String>,
}

/// Recursively mirror a local directory into a bucket under `prefix`.
/// Real implementation (no simulation): walks the tree, preserving relative
/// paths as `/`-delimited keys, and uploads every file.
#[command]
pub async fn storage_upload_directory(
    conn: S3Connection,
    bucket: String,
    prefix: String,
    local_dir: String,
) -> Result<UploadDirResult, String> {
    let root = std::path::PathBuf::from(&local_dir);
    if !root.is_dir() {
        return Err(format!("not a directory: {local_dir}"));
    }

    let mut files = Vec::new();
    collect_files(&root, &mut files).map_err(|e| format!("failed to scan directory: {e}"))?;

    let client = make_s3_client(&conn).await;
    let mut result = UploadDirResult {
        uploaded: 0,
        total_files: files.len() as u32,
        total_bytes: 0,
        errors: Vec::new(),
    };

    for path in &files {
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let rel_key = rel.to_string_lossy().replace('\\', "/");
        let key = format!("{prefix}{rel_key}");
        let mime = guess_mime_type(&path.to_string_lossy());

        match ByteStream::from_path(path).await {
            Ok(body) => {
                let len = tokio::fs::metadata(path).await.map(|m| m.len()).unwrap_or(0);
                match client
                    .put_object()
                    .bucket(&bucket)
                    .key(&key)
                    .content_type(mime)
                    .body(body)
                    .send()
                    .await
                {
                    Ok(_) => {
                        result.uploaded += 1;
                        result.total_bytes += len;
                    }
                    Err(e) => result.errors.push(format!("{key}: {e}")),
                }
            }
            Err(e) => result.errors.push(format!("{key}: {e}")),
        }
    }

    Ok(result)
}

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // Skip VCS/system noise that shouldn't be mirrored into a bucket.
        if name == ".git" || name == ".DS_Store" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

#[command]
pub async fn storage_upload_bytes(
    conn: S3Connection,
    bucket: String,
    key: String,
    data_base64: String,
    content_type: Option<String>,
) -> Result<(), String> {
    use base64::Engine;
    let client = make_s3_client(&conn).await;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64)
        .map_err(|e| format!("failed to decode base64: {e}"))?;
        
    let size = bytes.len() as u64;
    let mime = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    if size > 64 * 1024 * 1024 {
        let chunk_size: u64 = 8 * 1024 * 1024; // 8MB
        let num_parts = ((size + chunk_size - 1) / chunk_size) as i32;

        let res = client.create_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .content_type(&mime)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let uid = res.upload_id().unwrap().to_string();

        let bytes_arc = std::sync::Arc::new(bytes);
        let uid_arc = std::sync::Arc::new(uid.clone());
        let bucket_arc = std::sync::Arc::new(bucket.clone());
        let key_arc = std::sync::Arc::new(key.clone());
        let client_arc = std::sync::Arc::new(client.clone());

        let mut pending_parts = Vec::new();
        for part_number in 1..=num_parts {
            let offset = (part_number - 1) as usize * chunk_size as usize;
            let current_chunk_size = std::cmp::min(chunk_size as usize, size as usize - offset);
            pending_parts.push((part_number, offset, current_chunk_size));
        }

        let stream = futures::stream::iter(pending_parts).map(|(part_number, offset, current_chunk_size)| {
            let uid = uid_arc.clone();
            let bucket = bucket_arc.clone();
            let key = key_arc.clone();
            let client = client_arc.clone();
            let bytes_ref = bytes_arc.clone();
            
            async move {
                let chunk = bytes_ref[offset..offset+current_chunk_size].to_vec();
                let body = ByteStream::from(chunk);
                let res = client.upload_part()
                    .bucket(bucket.as_ref())
                    .key(key.as_ref())
                    .upload_id(uid.as_ref())
                    .part_number(part_number)
                    .body(body)
                    .send()
                    .await.map_err(|e| format!("part {} failed: {}", part_number, e))?;
                    
                Ok::<_, String>((part_number, res.e_tag().unwrap_or("").to_string()))
            }
        }).buffer_unordered(4);

        let mut completed_parts_map = std::collections::HashMap::new();
        let mut stream = stream;
        while let Some(res) = stream.next().await {
            match res {
                Ok((part_number, etag)) => {
                    completed_parts_map.insert(part_number, etag);
                }
                Err(e) => {
                    let _ = client_arc.abort_multipart_upload()
                        .bucket(bucket_arc.as_ref())
                        .key(key_arc.as_ref())
                        .upload_id(uid_arc.as_ref())
                        .send()
                        .await;
                    return Err(e);
                }
            }
        }

        let mut completed_parts_vec = Vec::new();
        for i in 1..=num_parts {
            if let Some(etag) = completed_parts_map.get(&i) {
                completed_parts_vec.push(
                    aws_sdk_s3::types::CompletedPart::builder()
                        .e_tag(etag)
                        .part_number(i)
                        .build()
                );
            } else {
                return Err(format!("missing part {}", i));
            }
        }
        
        let multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts_vec))
            .build();
            
        client.complete_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(uid_arc.as_ref())
            .multipart_upload(multipart_upload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    } else {
        let body = ByteStream::from(bytes);
        client.put_object()
            .bucket(&bucket)
            .key(&key)
            .content_type(mime)
            .body(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[command]
pub async fn storage_download_object(
    conn: S3Connection,
    bucket: String,
    key: String,
    save_path: String,
) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    let resp = client.get_object()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let mut body = resp.body;
    let mut file = tokio::fs::File::create(&save_path)
        .await
        .map_err(|e| format!("failed to create destination file: {e}"))?;

    while let Some(chunk) = body.next().await {
        let bytes = chunk.map_err(|e| e.to_string())?;
        file.write_all(&bytes).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn storage_delete_objects(
    conn: S3Connection,
    bucket: String,
    keys: Vec<String>,
) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    let mut delete_objects = aws_sdk_s3::types::Delete::builder();
    for key in keys {
        delete_objects = delete_objects.objects(
            aws_sdk_s3::types::ObjectIdentifier::builder().key(key).build().unwrap()
        );
    }
    client.delete_objects()
        .bucket(&bucket)
        .delete(delete_objects.build().unwrap())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn storage_get_presigned_url(
    conn: S3Connection,
    bucket: String,
    key: String,
    method: String,
    ttl_secs: u64,
) -> Result<String, String> {
    let client = make_s3_client(&conn).await;
    let expires_in = std::time::Duration::from_secs(ttl_secs);
    let presigned_config = aws_sdk_s3::presigning::PresigningConfig::builder()
        .expires_in(expires_in)
        .build()
        .map_err(|e| e.to_string())?;

    if method.to_uppercase() == "PUT" {
        let presigned = client.put_object()
            .bucket(&bucket)
            .key(&key)
            .presigned(presigned_config)
            .await
            .map_err(|e| e.to_string())?;
        Ok(presigned.uri().to_string())
    } else {
        let presigned = client.get_object()
            .bucket(&bucket)
            .key(&key)
            .presigned(presigned_config)
            .await
            .map_err(|e| e.to_string())?;
        Ok(presigned.uri().to_string())
    }
}

// Access Control
#[command]
pub async fn storage_get_bucket_policy(conn: S3Connection, bucket: String) -> Result<String, String> {
    let client = make_s3_client(&conn).await;
    let res = client.get_bucket_policy()
        .bucket(&bucket)
        .send()
        .await;
    match res {
        Ok(policy_output) => {
            Ok(policy_output.policy().unwrap_or("").to_string())
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("NoSuchBucketPolicy") {
                Ok("".to_string())
            } else {
                Err(err_str)
            }
        }
    }
}

#[command]
pub async fn storage_set_bucket_policy(
    conn: S3Connection,
    bucket: String,
    policy_json: String,
) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    if policy_json.trim().is_empty() {
        client.delete_bucket_policy()
            .bucket(&bucket)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    } else {
        client.put_bucket_policy()
            .bucket(&bucket)
            .policy(policy_json)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn storage_set_bucket_public(
    conn: S3Connection,
    bucket: String,
    public: bool,
) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    if public {
        let policy = format!(
            r#"{{"Version":"2012-10-17","Statement":[{{"Sid":"PublicReadGetObject","Effect":"Allow","Principal":"*","Action":"s3:GetObject","Resource":"arn:aws:s3:::{}/*"}}]}}"#,
            bucket
        );
        client.put_bucket_policy()
            .bucket(&bucket)
            .policy(policy)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    } else {
        client.delete_bucket_policy()
            .bucket(&bucket)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Metadata
#[command]
pub async fn storage_get_object_metadata(
    conn: S3Connection,
    bucket: String,
    key: String,
) -> Result<ObjectMetadata, String> {
    let client = make_s3_client(&conn).await;
    let resp = client.head_object()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let mut metadata = HashMap::new();
    if let Some(meta) = resp.metadata() {
        for (k, v) in meta {
            metadata.insert(k.clone(), v.clone());
        }
    }
    
    Ok(ObjectMetadata {
        key: key.clone(),
        size: resp.content_length().unwrap_or(0),
        last_modified: resp.last_modified().map(|d| d.secs() * 1000),
        content_type: resp.content_type().map(|s| s.to_string()),
        metadata,
    })
}

#[command]
pub async fn storage_set_object_metadata(
    conn: S3Connection,
    bucket: String,
    key: String,
    content_type: String,
    metadata: HashMap<String, String>,
) -> Result<(), String> {
    let client = make_s3_client(&conn).await;
    let copy_source = format!("{}/{}", bucket, key);
    let mut builder = client.copy_object()
        .bucket(&bucket)
        .key(&key)
        .copy_source(copy_source)
        .content_type(content_type)
        .metadata_directive(aws_sdk_s3::types::MetadataDirective::Replace);
        
    for (k, v) in metadata {
        builder = builder.metadata(k, v);
    }
    
    builder.send()
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(())
}

// Read object preview
#[command]
pub async fn storage_read_object_preview(
    conn: S3Connection,
    bucket: String,
    key: String,
) -> Result<String, String> {
    let client = make_s3_client(&conn).await;
    let resp = client.get_object()
        .bucket(&bucket)
        .key(&key)
        .range("bytes=0-51200")
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    let content_type = resp.content_type().unwrap_or("application/octet-stream").to_string();
    let mut body = resp.body;
    let mut data = Vec::new();
    while let Some(chunk) = body.next().await {
        let bytes = chunk.map_err(|e| e.to_string())?;
        data.extend_from_slice(&bytes);
    }
    
    let is_text = data.iter().take(1024).all(|&b| b == 9 || b == 10 || b == 13 || (b >= 32 && b <= 126) || b >= 128);
    if is_text {
        Ok(String::from_utf8_lossy(&data).to_string())
    } else {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        Ok(format!("data:{};base64,{}", content_type, b64))
    }
}

// Dialog-based upload/download helpers
#[command]
pub async fn storage_pick_upload_file() -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .pick_file()
        .await;
    Ok(file.map(|f| f.path().to_string_lossy().to_string()))
}

#[command]
pub async fn storage_pick_download_destination(filename: String) -> Result<Option<String>, String> {
    let file = rfd::AsyncFileDialog::new()
        .set_file_name(&filename)
        .save_file()
        .await;
    Ok(file.map(|f| f.path().to_string_lossy().to_string()))
}
