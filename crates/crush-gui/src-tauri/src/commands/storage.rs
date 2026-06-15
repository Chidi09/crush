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

// Connection management
#[command]
pub async fn storage_get_connections(state: State<'_, AppState>) -> Result<Vec<S3Connection>, String> {
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
    let body = ByteStream::from_path(&file_path)
        .await
        .map_err(|e| format!("failed to read file: {e}"))?;
    
    let mime = guess_mime_type(&file_path);

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
    let body = ByteStream::from(bytes);
    
    let mime = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

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
