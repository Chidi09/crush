use std::path::PathBuf;
use std::fs;
use serde::Serialize;
use tauri::command;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize)]
pub struct BackupFile {
    pub name: String,
    pub size: u64,
    pub modified_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DbStatus {
    pub is_up: bool,
    pub port: u16,
}

#[command]
pub async fn db_status() -> Result<DbStatus, String> {
    // Just check if localhost:5432 is reachable
    let is_up = tokio::net::TcpStream::connect("127.0.0.1:5432").await.is_ok();
    Ok(DbStatus {
        is_up,
        port: 5432,
    })
}

#[command]
pub async fn db_backups() -> Result<Vec<BackupFile>, String> {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".crush/backups");
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let modified_ms = meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    files.push(BackupFile {
                        name: entry.file_name().to_string_lossy().to_string(),
                        size: meta.len(),
                        modified_ms,
                    });
                }
            }
        }
    }
    files.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    Ok(files)
}

#[command]
pub async fn db_backup_now() -> Result<(), String> {
    run_pg_dump().await
}

#[command]
pub async fn db_restore(filename: String) -> Result<(), String> {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".crush/backups");
    let file = backup_dir.join(&filename);
    if !file.exists() {
        return Err("Backup file not found".into());
    }

    let pg_restore = if cfg!(target_os = "windows") { "pg_restore.exe" } else { "pg_restore" };
    
    // We restore using pg_restore because pg_dump was with -F c
    let output = tokio::process::Command::new(pg_restore)
        .args([
            "-h", "localhost",
            "-p", "5432",
            "-U", "postgres",
            "-d", "postgres", // default db
            "--clean", // drop db objects before recreating
            "--if-exists",
            &file.to_string_lossy(),
        ])
        .env("PGPASSWORD", "postgres")
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pg_restore failed: {}", err));
    }
    Ok(())
}

#[command]
pub async fn db_delete_backup(filename: String) -> Result<(), String> {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".crush/backups");
    let file = backup_dir.join(&filename);
    if file.exists() {
        std::fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn run_pg_dump() -> Result<(), String> {
    let backup_dir = dirs::home_dir().unwrap_or_default().join(".crush/backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    
    let now = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let file = backup_dir.join(format!("crush_pg_{}.sql", now));
    
    // Attempt to use pg_dump
    let pg_dump = if cfg!(target_os = "windows") { "pg_dump.exe" } else { "pg_dump" };
    
    let output = tokio::process::Command::new(pg_dump)
        .args([
            "-h", "localhost",
            "-p", "5432",
            "-U", "postgres",
            "-F", "c", // Custom format
            "-f", &file.to_string_lossy(),
            "postgres", // Default database
        ])
        .env("PGPASSWORD", "postgres") // default
        .output()
        .await
        .map_err(|e| e.to_string())?;
        
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pg_dump failed: {}", err));
    }
    Ok(())
}

pub fn spawn_backup_task() {
    // NOTE: called from the Tauri `setup` closure, which does NOT run inside a
    // Tokio runtime context — raw `tokio::spawn` here calls `Handle::current()`
    // internally and panics ("must be called from the context of a Tokio
    // runtime"), crashing the GUI on launch. Use Tauri's managed runtime, same
    // as the mail-catcher spawn in lib.rs.
    tauri::async_runtime::spawn(async move {
        loop {
            // Run every 24h
            sleep(Duration::from_secs(24 * 3600)).await;
            let _ = run_pg_dump().await;
        }
    });
}

use serde::Deserialize;
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected: u64,
    pub error: Option<String>,
    pub duration_ms: u64,
}

struct MyConn {
    host: String,
    port: u16,
    user: String,
    password: Option<String>,
}

impl MyConn {
    fn parse(url: &str) -> Result<Self, String> {
        let rest = url.split("://").nth(1).ok_or_else(|| "bad mysql URL".to_string())?;
        let (auth_host, _db) = rest.split_once('/').unwrap_or((rest, ""));
        let (auth, hostport) = match auth_host.rsplit_once('@') {
            Some((a, h)) => (Some(a), h),
            None => (None, auth_host),
        };
        let (user, password) = match auth {
            Some(a) => match a.split_once(':') {
                Some((u, p)) => (u.to_string(), Some(p.to_string())),
                None => (a.to_string(), None),
            },
            None => ("root".to_string(), None),
        };
        let (host, port) = match hostport.split_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().unwrap_or(3306)),
            None => (hostport.to_string(), 3306u16),
        };
        Ok(MyConn { host: if host.is_empty() { "localhost".into() } else { host }, port, user, password })
    }
}

fn database_from_url(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let path = after_scheme.split('/').nth(1)?; // host[:port] / db
    let db = path.split(['?', ';']).next().unwrap_or(path);
    if db.is_empty() { None } else { Some(db.to_string()) }
}

async fn query_postgres(url: &str, sql: &str) -> Result<QueryResult, String> {
    let (client, connection) = tokio_postgres::connect(url, tokio_postgres::NoTls)
        .await
        .map_err(|e| format!("connection failed: {e}"))?;
    
    let handle = tokio::spawn(async move {
        let _ = connection.await;
    });

    let start = Instant::now();
    let res = client.simple_query(sql).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    handle.abort();

    match res {
        Ok(messages) => {
            let mut columns = Vec::new();
            let mut rows = Vec::new();
            let mut affected = 0;

            for msg in messages {
                match msg {
                    tokio_postgres::SimpleQueryMessage::Row(row) => {
                        if columns.is_empty() {
                            for col in row.columns() {
                                columns.push(col.name().to_string());
                            }
                        }
                        let mut row_vals = Vec::new();
                        for i in 0..row.len() {
                            let val = row.get(i).map(|s| serde_json::Value::String(s.to_string())).unwrap_or(serde_json::Value::Null);
                            row_vals.push(val);
                        }
                        rows.push(row_vals);
                    }
                    tokio_postgres::SimpleQueryMessage::CommandComplete(count) => {
                        affected = count;
                    }
                    _ => {}
                }
            }

            Ok(QueryResult {
                columns,
                rows,
                affected,
                error: None,
                duration_ms,
            })
        }
        Err(e) => {
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected: 0,
                error: Some(e.to_string()),
                duration_ms,
            })
        }
    }
}

async fn query_mysql(url: &str, sql: &str) -> Result<QueryResult, String> {
    let conn = MyConn::parse(url)?;
    let db = database_from_url(url).unwrap_or_default();

    let start = Instant::now();
    let mut cmd = tokio::process::Command::new("mysql");
    cmd.arg(format!("--host={}", conn.host))
       .arg(format!("--port={}", conn.port))
       .arg(format!("--user={}", conn.user))
       .arg("-B"); // batch mode
    
    if let Some(pw) = &conn.password {
        cmd.env("MYSQL_PWD", pw);
    }
    if !db.is_empty() {
        cmd.arg(format!("--database={}", db));
    }
    cmd.arg("-e").arg(sql);

    let output = cmd.output().await.map_err(|e| format!("failed to run mysql client: {e}"))?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected: 0,
            error: Some(err),
            duration_ms,
        });
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout_str.lines();
    
    let mut columns = Vec::new();
    let mut rows = Vec::new();

    if let Some(header_line) = lines.next() {
        columns = header_line.split('\t').map(|s| s.to_string()).collect();
        for line in lines {
            let row_vals = line.split('\t').map(|s| {
                if s == "NULL" {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(s.to_string())
                }
            }).collect();
            rows.push(row_vals);
        }
    }

    let affected = rows.len() as u64;
    Ok(QueryResult {
        columns,
        rows,
        affected,
        error: None,
        duration_ms,
    })
}

#[command]
pub async fn db_run_query(
    engine: String,
    url: String,
    sql: String,
) -> Result<QueryResult, String> {
    if engine == "postgres" {
        query_postgres(&url, &sql).await
    } else if engine == "mysql" {
        query_mysql(&url, &sql).await
    } else {
        Err(format!("unsupported SQL engine '{}'", engine))
    }
}

// Redis operations
#[derive(Debug, Serialize, Deserialize)]
pub struct RedisKeyInfo {
    pub key: String,
    pub kind: String,
    pub ttl: i64,
}

#[command]
pub async fn redis_list_keys(
    port: u16,
    password: Option<String>,
    pattern: Option<String>,
) -> Result<Vec<RedisKeyInfo>, String> {
    let url = match password.filter(|s| !s.is_empty()) {
        Some(p) => format!("redis://:{p}@127.0.0.1:{port}/"),
        None => format!("redis://127.0.0.1:{port}/"),
    };
    let client = redis::Client::open(url).map_err(|e| e.to_string())?;
    let mut con = client.get_multiplexed_async_connection().await.map_err(|e| format!("connect failed: {e}"))?;

    let pat = pattern.filter(|p| !p.is_empty()).unwrap_or_else(|| "*".to_string());
    
    let found: Vec<String> = redis::cmd("KEYS")
        .arg(&pat)
        .query_async(&mut con).await.map_err(|e| e.to_string())?;

    let mut keys = Vec::new();
    for k in found.into_iter().take(200) {
        let kind: String = redis::cmd("TYPE").arg(&k).query_async(&mut con).await.unwrap_or_else(|_| "?".into());
        let ttl: i64 = redis::cmd("TTL").arg(&k).query_async(&mut con).await.unwrap_or(-1);
        keys.push(RedisKeyInfo { key: k, kind, ttl });
    }
    Ok(keys)
}

#[command]
pub async fn redis_get_val(
    port: u16,
    password: Option<String>,
    key: String,
) -> Result<String, String> {
    let url = match password.filter(|s| !s.is_empty()) {
        Some(p) => format!("redis://:{p}@127.0.0.1:{port}/"),
        None => format!("redis://127.0.0.1:{port}/"),
    };
    let client = redis::Client::open(url).map_err(|e| e.to_string())?;
    let mut con = client.get_multiplexed_async_connection().await.map_err(|e| format!("connect failed: {e}"))?;

    let val: String = redis::cmd("GET").arg(&key).query_async(&mut con).await.map_err(|e| e.to_string())?;
    Ok(val)
}

#[command]
pub async fn redis_set_val(
    port: u16,
    password: Option<String>,
    key: String,
    value: String,
    ttl_secs: Option<i64>,
) -> Result<(), String> {
    let url = match password.filter(|s| !s.is_empty()) {
        Some(p) => format!("redis://:{p}@127.0.0.1:{port}/"),
        None => format!("redis://127.0.0.1:{port}/"),
    };
    let client = redis::Client::open(url).map_err(|e| e.to_string())?;
    let mut con = client.get_multiplexed_async_connection().await.map_err(|e| format!("connect failed: {e}"))?;

    if let Some(t) = ttl_secs {
        if t > 0 {
            let _: () = redis::cmd("SETEX").arg(&key).arg(t).arg(&value).query_async(&mut con).await.map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    let _: () = redis::cmd("SET").arg(&key).arg(&value).query_async(&mut con).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn redis_del_key(
    port: u16,
    password: Option<String>,
    key: String,
) -> Result<(), String> {
    let url = match password.filter(|s| !s.is_empty()) {
        Some(p) => format!("redis://:{p}@127.0.0.1:{port}/"),
        None => format!("redis://127.0.0.1:{port}/"),
    };
    let client = redis::Client::open(url).map_err(|e| e.to_string())?;
    let mut con = client.get_multiplexed_async_connection().await.map_err(|e| format!("connect failed: {e}"))?;

    let _: i32 = redis::cmd("DEL").arg(&key).query_async(&mut con).await.map_err(|e| e.to_string())?;
    Ok(())
}

// MongoDB operations
#[command]
pub async fn mongo_list_databases(port: u16) -> Result<Vec<String>, String> {
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let names = client.list_database_names().await.map_err(|e| e.to_string())?;
    Ok(names)
}

#[command]
pub async fn mongo_list_collections(port: u16, database: String) -> Result<Vec<String>, String> {
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let db = client.database(&database);
    let names = db.list_collection_names().await.map_err(|e| e.to_string())?;
    Ok(names)
}

#[command]
pub async fn mongo_find_docs(
    port: u16,
    database: String,
    collection: String,
    filter_json: Option<String>,
    limit: i64,
    skip: i64,
) -> Result<Vec<String>, String> {
    use mongodb::bson::Document;
    use futures::stream::StreamExt;
    
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let db = client.database(&database);
    let coll = db.collection::<Document>(&collection);

    let filter = if let Some(f_str) = filter_json.filter(|s| !s.trim().is_empty()) {
        let parsed: serde_json::Value = serde_json::from_str(&f_str).map_err(|e| format!("invalid filter JSON: {e}"))?;
        let doc: Document = mongodb::bson::to_document(&parsed).map_err(|e| format!("failed to convert filter to BSON: {e}"))?;
        doc
    } else {
        Document::new()
    };

    let find_options = mongodb::options::FindOptions::builder()
        .limit(limit)
        .skip(skip as u64)
        .build();

    let mut cursor = coll.find(filter).with_options(find_options).await.map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    while let Some(doc) = cursor.next().await {
        if let Ok(d) = doc {
            let json_val: serde_json::Value = mongodb::bson::from_document(d).map_err(|e| e.to_string())?;
            results.push(json_val.to_string());
        }
    }
    Ok(results)
}

#[command]
pub async fn mongo_insert_doc(
    port: u16,
    database: String,
    collection: String,
    doc_json: String,
) -> Result<(), String> {
    use mongodb::bson::Document;
    
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let db = client.database(&database);
    let coll = db.collection::<Document>(&collection);

    let parsed: serde_json::Value = serde_json::from_str(&doc_json).map_err(|e| format!("invalid document JSON: {e}"))?;
    let doc: Document = mongodb::bson::to_document(&parsed).map_err(|e| format!("failed to convert document to BSON: {e}"))?;

    coll.insert_one(doc).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn mongo_update_doc(
    port: u16,
    database: String,
    collection: String,
    filter_json: String,
    update_json: String,
) -> Result<u64, String> {
    use mongodb::bson::Document;
    
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let db = client.database(&database);
    let coll = db.collection::<Document>(&collection);

    let parsed_filter: serde_json::Value = serde_json::from_str(&filter_json).map_err(|e| format!("invalid filter JSON: {e}"))?;
    let filter: Document = mongodb::bson::to_document(&parsed_filter).map_err(|e| format!("failed to convert filter to BSON: {e}"))?;

    let parsed_update: serde_json::Value = serde_json::from_str(&update_json).map_err(|e| format!("invalid update JSON: {e}"))?;
    let update: Document = mongodb::bson::to_document(&parsed_update).map_err(|e| format!("failed to convert update to BSON: {e}"))?;

    let res = coll.update_many(filter, update).await.map_err(|e| e.to_string())?;
    Ok(res.modified_count)
}

#[command]
pub async fn mongo_delete_doc(
    port: u16,
    database: String,
    collection: String,
    filter_json: String,
) -> Result<u64, String> {
    use mongodb::bson::Document;
    
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let db = client.database(&database);
    let coll = db.collection::<Document>(&collection);

    let parsed_filter: serde_json::Value = serde_json::from_str(&filter_json).map_err(|e| format!("invalid filter JSON: {e}"))?;
    let filter: Document = mongodb::bson::to_document(&parsed_filter).map_err(|e| format!("failed to convert filter to BSON: {e}"))?;

    let res = coll.delete_many(filter).await.map_err(|e| e.to_string())?;
    Ok(res.deleted_count)
}

