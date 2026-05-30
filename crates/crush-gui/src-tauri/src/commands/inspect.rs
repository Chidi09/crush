//! Live inspection of running native services — peek at a database's tables /
//! connections, a Redis keyspace, etc. Connects over localhost using the
//! service's port (and, for Postgres, the cluster superuser, which the driver
//! initialises as `postgres`/`postgres`).

use serde::Serialize;

// ── Postgres ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize)]
pub struct PgTable { pub schema: String, pub name: String, pub rows: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct PgConn { pub pid: i32, pub user: String, pub db: String, pub state: String, pub query: String }
#[derive(Debug, Clone, Serialize)]
pub struct PgInspect {
    pub version: String,
    pub current_db: String,
    pub databases: Vec<String>,
    pub tables: Vec<PgTable>,
    pub connections: Vec<PgConn>,
}

#[tauri::command]
pub async fn inspect_postgres(
    port: u16,
    user: Option<String>,
    password: Option<String>,
    database: Option<String>,
) -> Result<PgInspect, String> {
    let user = user.filter(|s| !s.is_empty()).unwrap_or_else(|| "postgres".into());
    let password = password.filter(|s| !s.is_empty()).unwrap_or_else(|| "postgres".into());
    let database = database.filter(|s| !s.is_empty()).unwrap_or_else(|| "postgres".into());

    let conn_str = format!(
        "host=127.0.0.1 port={port} user={user} password={password} dbname={database} connect_timeout=4"
    );
    let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls)
        .await
        .map_err(|e| format!("connect failed: {e}"))?;
    let handle = tokio::spawn(async move { let _ = connection.await; });

    let version: String = client
        .query_one("SELECT version()", &[]).await
        .map(|r| r.get(0)).unwrap_or_default();

    let databases = client
        .query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[])
        .await.map_err(|e| e.to_string())?
        .iter().map(|r| r.get::<_, String>(0)).collect();

    let tables = client
        .query("SELECT schemaname, relname, n_live_tup FROM pg_stat_user_tables ORDER BY n_live_tup DESC LIMIT 300", &[])
        .await.map_err(|e| e.to_string())?
        .iter().map(|r| PgTable { schema: r.get(0), name: r.get(1), rows: r.get::<_, i64>(2) }).collect();

    let connections = client
        .query("SELECT pid, usename, datname, state, query FROM pg_stat_activity WHERE datname IS NOT NULL ORDER BY pid", &[])
        .await.map_err(|e| e.to_string())?
        .iter().map(|r| PgConn {
            pid: r.get::<_, Option<i32>>(0).unwrap_or(0),
            user: r.get::<_, Option<String>>(1).unwrap_or_default(),
            db: r.get::<_, Option<String>>(2).unwrap_or_default(),
            state: r.get::<_, Option<String>>(3).unwrap_or_default(),
            query: { let q: Option<String> = r.get(4); q.unwrap_or_default().chars().take(120).collect() },
        }).collect();

    handle.abort();
    Ok(PgInspect { version, current_db: database, databases, tables, connections })
}

// ── Redis / Valkey / Garnet ────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize)]
pub struct RedisKey { pub key: String, pub kind: String, pub ttl: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct RedisInspect { pub total: i64, pub keys: Vec<RedisKey> }

#[tauri::command]
pub async fn inspect_redis(port: u16, password: Option<String>) -> Result<RedisInspect, String> {
    let url = match password.filter(|s| !s.is_empty()) {
        Some(p) => format!("redis://:{p}@127.0.0.1:{port}/"),
        None => format!("redis://127.0.0.1:{port}/"),
    };
    let client = redis::Client::open(url).map_err(|e| e.to_string())?;
    let mut con = client.get_multiplexed_async_connection().await.map_err(|e| format!("connect failed: {e}"))?;

    let total: i64 = redis::cmd("DBSIZE").query_async(&mut con).await.map_err(|e| e.to_string())?;

    // One SCAN pass (up to ~200 keys) — enough for a browse, cheap on big DBs.
    let (_cursor, found): (String, Vec<String>) = redis::cmd("SCAN")
        .arg(0).arg("COUNT").arg(200)
        .query_async(&mut con).await.map_err(|e| e.to_string())?;

    let mut keys = Vec::new();
    for k in found.into_iter().take(200) {
        let kind: String = redis::cmd("TYPE").arg(&k).query_async(&mut con).await.unwrap_or_else(|_| "?".into());
        let ttl: i64 = redis::cmd("TTL").arg(&k).query_async(&mut con).await.unwrap_or(-1);
        keys.push(RedisKey { key: k, kind, ttl });
    }
    Ok(RedisInspect { total, keys })
}

// ── MongoDB ─────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize)]
pub struct MongoColl { pub name: String, pub count: u64 }
#[derive(Debug, Clone, Serialize)]
pub struct MongoDb { pub name: String, pub collections: Vec<MongoColl> }
#[derive(Debug, Clone, Serialize)]
pub struct MongoInspect { pub databases: Vec<MongoDb> }

#[tauri::command]
pub async fn inspect_mongo(port: u16) -> Result<MongoInspect, String> {
    let uri = format!("mongodb://127.0.0.1:{port}/?serverSelectionTimeoutMS=4000&connectTimeoutMS=4000");
    let client = mongodb::Client::with_uri_str(&uri).await.map_err(|e| format!("connect failed: {e}"))?;
    let names = client.list_database_names().await.map_err(|e| e.to_string())?;
    let mut databases = Vec::new();
    for name in names {
        let db = client.database(&name);
        let colls = db.list_collection_names().await.unwrap_or_default();
        let mut collections = Vec::new();
        for c in colls {
            let count = db
                .collection::<mongodb::bson::Document>(&c)
                .estimated_document_count().await.unwrap_or(0);
            collections.push(MongoColl { name: c, count });
        }
        databases.push(MongoDb { name, collections });
    }
    Ok(MongoInspect { databases })
}

// ── MinIO / S3 buckets ───────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize)]
pub struct S3Bucket { pub name: String, pub objects: i64, pub size: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct MinioInspect { pub buckets: Vec<S3Bucket> }

#[tauri::command]
pub async fn inspect_minio(port: u16, user: Option<String>, password: Option<String>) -> Result<MinioInspect, String> {
    use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
    let user = user.filter(|s| !s.is_empty()).unwrap_or_else(|| "minioadmin".into());
    let password = password.filter(|s| !s.is_empty()).unwrap_or_else(|| "minioadmin".into());
    let creds = Credentials::new(user, password, None, None, "crush");
    let conf = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url(format!("http://127.0.0.1:{port}"))
        .credentials_provider(creds)
        .force_path_style(true)
        .build();
    let client = aws_sdk_s3::Client::from_conf(conf);
    let resp = client.list_buckets().send().await.map_err(|e| format!("connect failed: {e}"))?;
    let mut buckets = Vec::new();
    for b in resp.buckets() {
        let name = b.name().unwrap_or_default().to_string();
        let (objects, size) = match client.list_objects_v2().bucket(&name).send().await {
            Ok(o) => {
                let count = o.key_count().unwrap_or(0) as i64;
                let total: i64 = o.contents().iter().map(|x| x.size().unwrap_or(0)).sum();
                (count, total)
            }
            Err(_) => (0, 0),
        };
        buckets.push(S3Bucket { name, objects, size });
    }
    Ok(MinioInspect { buckets })
}
