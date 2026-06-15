use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};
use crush_services::ServiceDriver;

// ── 2a. Compose YAML deserialization types ──────────────────────────

#[derive(Debug, Deserialize)]
struct ComposeFile {
    #[serde(default)]
    services: HashMap<String, ComposeServiceDef>,
}

#[derive(Debug, Deserialize, Default)]
struct ComposeServiceDef {
    image: Option<String>,
    #[serde(default)]
    build: serde_yaml::Value,
    #[serde(default)]
    ports: Vec<serde_yaml::Value>,
    #[serde(default)]
    environment: serde_yaml::Value,
    #[serde(default)]
    volumes: Vec<String>,
    command: Option<serde_yaml::Value>,
}

// ── 2b. Public output types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepService {
    pub name: String,
    pub image: String,
    pub ports: Vec<(u16, u16)>,   // (host_port, container_port)
    pub env: Vec<(String, String)>,
    pub volumes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppServiceHints {
    pub name: String,
    pub command: Option<String>,  // compose `command:` for native launch override
    pub env: Vec<(String, String)>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCompose {
    pub dep_services: Vec<DepService>,
    pub app_hints: Option<AppServiceHints>,
}

// ── 2c. Service classification ──────────────────────────────────────

static DEP_IMAGE_PREFIXES: &[&str] = &[
    "postgres", "pgvector", "timescale", "mysql", "mariadb", "mongodb", "mongo",
    "redis", "keydb", "dragonflydb",
    "rabbitmq", "kafka", "zookeeper", "nats", "mosquitto", "emqx",
    "elasticsearch", "opensearch", "meilisearch", "typesense",
    "memcached", "minio", "localstack",
    "mailhog", "mailpit", "smtp4dev",
    "nginx", "traefik", "haproxy", "envoy",
    "grafana", "prometheus", "jaeger", "zipkin",
    "vault", "consul", "etcd",
    "clickhouse", "cassandra", "cockroachdb", "questdb", "influxdb",
];

fn is_dep_image(image: &str) -> bool {
    // Strip tag/digest first, e.g. postgres:alpine -> postgres
    let img_without_tag = image.split(':').next().unwrap_or(image).split('@').next().unwrap_or(image);
    // Strip registry and organization, e.g. ghcr.io/foo/postgres -> postgres
    let img_name = img_without_tag.split('/').last().unwrap_or(img_without_tag);
    
    DEP_IMAGE_PREFIXES.iter().any(|&prefix| {
        img_name.starts_with(prefix)
    })
}

fn has_local_build(val: &serde_yaml::Value) -> bool {
    if let Some(s) = val.as_str() {
        return s == "." || s == "./" || s.starts_with("./") || s.starts_with("../");
    }
    if let Some(map) = val.as_mapping() {
        if let Some(ctx) = map.get(&serde_yaml::Value::String("context".to_string())) {
            if let Some(s) = ctx.as_str() {
                return s == "." || s == "./" || s.starts_with("./") || s.starts_with("../");
            }
        }
    }
    false
}

// ── 2d. Helper parsers ──────────────────────────────────────────────

fn parse_env(val: &serde_yaml::Value) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if let Some(map) = val.as_mapping() {
        for (k, v) in map {
            let key = match k {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                _ => continue,
            };
            let val = match v {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::Bool(b) => b.to_string(),
                serde_yaml::Value::Null => String::new(),
                _ => continue,
            };
            env.push((key, val));
        }
    } else if let Some(seq) = val.as_sequence() {
        for item in seq {
            if let Some(s) = item.as_str() {
                if let Some(idx) = s.find('=') {
                    let key = s[..idx].to_string();
                    let val = s[idx+1..].to_string();
                    env.push((key, val));
                }
            }
        }
    }
    env
}

fn parse_command(val: &Option<serde_yaml::Value>) -> Option<String> {
    let val = val.as_ref()?;
    if let Some(s) = val.as_str() {
        return Some(s.to_string());
    }
    if let Some(seq) = val.as_sequence() {
        let parts: Vec<String> = seq.iter()
            .filter_map(|item| {
                if let Some(s) = item.as_str() {
                    Some(s.to_string())
                } else if let Some(n) = item.as_f64() {
                    Some(n.to_string())
                } else if let Some(i) = item.as_i64() {
                    Some(i.to_string())
                } else {
                    None
                }
            })
            .collect();
        if !parts.is_empty() {
            return Some(parts.join(" "));
        }
    }
    None
}

fn parse_port_pair(val: &serde_yaml::Value) -> Option<(u16, u16)> {
    if let Some(n) = val.as_u64() {
        return Some((n as u16, n as u16));
    }
    if let Some(s) = val.as_str() {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 1 {
            let port = parts[0].parse::<u16>().ok()?;
            return Some((port, port));
        } else if parts.len() == 2 {
            let host_port = parts[0].parse::<u16>().ok()?;
            let container_port = parts[1].parse::<u16>().ok()?;
            return Some((host_port, container_port));
        } else if parts.len() == 3 {
            let host_port = parts[1].parse::<u16>().ok()?;
            let container_port = parts[2].parse::<u16>().ok()?;
            return Some((host_port, container_port));
        }
    }
    None
}

pub fn parse_compose(path: &Path) -> anyhow::Result<ParsedCompose> {
    let content = fs::read_to_string(path)?;
    let compose: ComposeFile = serde_yaml::from_str(&content)?;
    
    let mut dep_services = Vec::new();
    let mut app_hints = None;
    
    for (name, svc) in compose.services {
        let is_app = has_local_build(&svc.build);
        
        if is_app && app_hints.is_none() {
            let env = parse_env(&svc.environment);
            let command = parse_command(&svc.command);
            let port = svc.ports.iter()
                .filter_map(|p| parse_port_pair(p))
                .map(|(hp, _)| hp)
                .next();
                
            app_hints = Some(AppServiceHints {
                name: name.clone(),
                command,
                env,
                port,
            });
        } else {
            let image = if let Some(ref img) = svc.image {
                img.clone()
            } else {
                continue;
            };
            
            // Only recognized infrastructure images count as deps. Services
            // without a local build that aren't on the dep image list are
            // almost always the user's own app variants (blue/green, canary,
            // worker pools) — starting them via crush would be wrong.
            let is_dep = is_dep_image(&image);
            if is_dep {
                let ports = svc.ports.iter()
                    .filter_map(|p| parse_port_pair(p))
                    .collect();
                let env = parse_env(&svc.environment);
                dep_services.push(DepService {
                    name: name.clone(),
                    image,
                    ports,
                    env,
                    volumes: svc.volumes.clone(),
                });
            }
        }
    }
    
    Ok(ParsedCompose {
        dep_services,
        app_hints,
    })
}

/// Synthesizes connection env vars (DATABASE_URL, REDIS_URL, etc.) that the
/// app needs to reach a dep service crush started. Without this the app falls
/// back to whatever default the framework picks (often SQLite or 127.0.0.1).
pub fn synthesize_dep_env(dep: &DepService) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let img = dep.image.split(':').next().unwrap_or(&dep.image);
    let img_name = img.split('/').last().unwrap_or(img);
    let host_port = dep.ports.first().map(|(hp, _)| *hp);
    let env_get = |k: &str| dep.env.iter().find(|(ek, _)| ek == k).map(|(_, v)| v.clone());

    if img_name.starts_with("postgres") || img_name.starts_with("pgvector") || img_name.starts_with("timescale") {
        let user = env_get("POSTGRES_USER").unwrap_or_else(|| "postgres".into());
        let pass = env_get("POSTGRES_PASSWORD").unwrap_or_else(|| "postgres".into());
        let db = env_get("POSTGRES_DB").unwrap_or_else(|| user.clone());
        let port = host_port.unwrap_or(5432);
        out.push(("POSTGRES_HOST".into(), "localhost".into()));
        out.push(("POSTGRES_PORT".into(), port.to_string()));
        out.push(("POSTGRES_USER".into(), user.clone()));
        out.push(("POSTGRES_PASSWORD".into(), pass.clone()));
        out.push(("POSTGRES_DB".into(), db.clone()));
        out.push(("DATABASE_URL".into(),
            format!("postgresql://{}:{}@localhost:{}/{}", user, pass, port, db)));
        // Spring Boot relaxed binding: these env vars override application.yml
        // without the app having to declare ${...} placeholders.
        out.push(("SPRING_DATASOURCE_URL".into(),
            format!("jdbc:postgresql://localhost:{}/{}", port, db)));
        out.push(("SPRING_DATASOURCE_USERNAME".into(), user));
        out.push(("SPRING_DATASOURCE_PASSWORD".into(), pass));
    } else if img_name.starts_with("redis") || img_name.starts_with("keydb") || img_name.starts_with("dragonflydb") {
        let port = host_port.unwrap_or(6379);
        out.push(("REDIS_HOST".into(), "localhost".into()));
        out.push(("REDIS_PORT".into(), port.to_string()));
        out.push(("REDIS_URL".into(), format!("redis://localhost:{}", port)));
        out.push(("SPRING_DATA_REDIS_HOST".into(), "localhost".into()));
        out.push(("SPRING_DATA_REDIS_PORT".into(), port.to_string()));
    } else if img_name.starts_with("mysql") || img_name.starts_with("mariadb") {
        let user = env_get("MYSQL_USER").or_else(|| env_get("MARIADB_USER")).unwrap_or_else(|| "root".into());
        let pass = env_get("MYSQL_PASSWORD").or_else(|| env_get("MARIADB_PASSWORD"))
            .or_else(|| env_get("MYSQL_ROOT_PASSWORD")).unwrap_or_default();
        let db = env_get("MYSQL_DATABASE").or_else(|| env_get("MARIADB_DATABASE")).unwrap_or_default();
        let port = host_port.unwrap_or(3306);
        out.push(("DATABASE_URL".into(),
            format!("mysql://{}:{}@localhost:{}/{}", user, pass, port, db)));
    } else if img_name.starts_with("mongo") {
        let port = host_port.unwrap_or(27017);
        out.push(("MONGODB_URL".into(), format!("mongodb://localhost:{}", port)));
    }
    out
}

/// Parses Spring Boot's `application.yml` / `application.properties` for
/// `spring.datasource.url`, `spring.data.redis.host`, etc., and returns
/// `DepService`s that crush can spin up. Lets projects without a compose
/// file still get their deps auto-started.
pub fn parse_spring_config(root: &Path) -> Vec<DepService> {
    let candidates = [
        "src/main/resources/application.yml",
        "src/main/resources/application.yaml",
    ];
    for path in candidates {
        if let Ok(content) = fs::read_to_string(root.join(path)) {
            return parse_spring_yaml(&content);
        }
    }
    if let Ok(content) = fs::read_to_string(root.join("src/main/resources/application.properties")) {
        return parse_spring_properties(&content);
    }
    Vec::new()
}

/// Resolves Spring's `${VAR:default}` placeholder syntax to the default value.
/// If no default is provided, returns the literal placeholder unchanged.
fn resolve_spring_placeholder(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(inner) = trimmed.strip_prefix("${").and_then(|r| r.strip_suffix("}")) {
        if let Some(idx) = inner.find(':') {
            return inner[idx + 1..].to_string();
        }
    }
    trimmed.to_string()
}

fn parse_spring_yaml(content: &str) -> Vec<DepService> {
    let val: serde_yaml::Value = match serde_yaml::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    let spring = &val["spring"];

    // JDBC datasource → postgres / mysql / mariadb
    if let Some(url) = spring["datasource"]["url"].as_str() {
        let resolved = resolve_spring_placeholder(url);
        let user = spring["datasource"]["username"].as_str()
            .map(resolve_spring_placeholder);
        let pass = spring["datasource"]["password"].as_str()
            .map(resolve_spring_placeholder);
        if let Some(dep) = jdbc_url_to_dep(&resolved, user.as_deref(), pass.as_deref()) {
            deps.push(dep);
        }
    }

    // Redis: Spring Boot 3.x uses spring.data.redis; older uses spring.redis
    for prefix in [&spring["data"]["redis"], &spring["redis"]] {
        if let Some(host) = prefix["host"].as_str() {
            let h = resolve_spring_placeholder(host);
            if h == "localhost" || h == "127.0.0.1" || h.is_empty() {
                let port = prefix["port"].as_u64()
                    .or_else(|| prefix["port"].as_str()
                        .map(resolve_spring_placeholder)
                        .and_then(|s| s.parse().ok()))
                    .unwrap_or(6379) as u16;
                deps.push(DepService {
                    name: "redis".into(),
                    image: "redis:7-alpine".into(),
                    ports: vec![(port, 6379)],
                    env: Vec::new(),
                    volumes: Vec::new(),
                });
                break;
            }
        }
    }
    deps
}

fn parse_spring_properties(content: &str) -> Vec<DepService> {
    let mut map: HashMap<String, String> = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        if let Some((k, v)) = trimmed.split_once('=') {
            map.insert(k.trim().to_string(), resolve_spring_placeholder(v.trim()));
        }
    }
    let mut deps = Vec::new();
    if let Some(url) = map.get("spring.datasource.url") {
        let user = map.get("spring.datasource.username").map(String::as_str);
        let pass = map.get("spring.datasource.password").map(String::as_str);
        if let Some(dep) = jdbc_url_to_dep(url, user, pass) {
            deps.push(dep);
        }
    }
    let redis_host = map.get("spring.data.redis.host").or_else(|| map.get("spring.redis.host"));
    if let Some(h) = redis_host {
        if h == "localhost" || h == "127.0.0.1" || h.is_empty() {
            let port = map.get("spring.data.redis.port")
                .or_else(|| map.get("spring.redis.port"))
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(6379);
            deps.push(DepService {
                name: "redis".into(),
                image: "redis:7-alpine".into(),
                ports: vec![(port, 6379)],
                env: Vec::new(),
                volumes: Vec::new(),
            });
        }
    }
    deps
}

fn jdbc_url_to_dep(url: &str, user: Option<&str>, pass: Option<&str>) -> Option<DepService> {
    let no_jdbc = url.strip_prefix("jdbc:")?;
    let scheme_idx = no_jdbc.find("://")?;
    let scheme = &no_jdbc[..scheme_idx];
    let rest = &no_jdbc[scheme_idx + 3..];

    let (image, default_port, default_user) = match scheme {
        "postgresql" => ("postgres:17-alpine", 5432u16, "postgres"),
        "mysql"      => ("mysql:8", 3306, "root"),
        "mariadb"    => ("mariadb:11", 3306, "root"),
        // In-process / file-backed → nothing to start
        "h2" | "sqlite" | "hsqldb" | "derby" => return None,
        _ => return None,
    };

    // host[:port]/db[?params]
    let (hostport_db, _) = rest.split_once('?').unwrap_or((rest, ""));
    let (hostport, dbname) = hostport_db.split_once('/').unwrap_or((hostport_db, ""));
    let port = hostport.rsplit(':').next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(default_port);

    let username = user.unwrap_or(default_user).to_string();
    let password = pass.unwrap_or("").to_string();

    let mut env = Vec::new();
    match scheme {
        "postgresql" => {
            env.push(("POSTGRES_USER".into(), username));
            env.push(("POSTGRES_PASSWORD".into(), password));
            if !dbname.is_empty() {
                env.push(("POSTGRES_DB".into(), dbname.to_string()));
            }
        }
        "mysql" | "mariadb" => {
            env.push(("MYSQL_USER".into(), username.clone()));
            env.push(("MYSQL_PASSWORD".into(), password.clone()));
            env.push(("MYSQL_ROOT_PASSWORD".into(), password));
            if !dbname.is_empty() {
                env.push(("MYSQL_DATABASE".into(), dbname.to_string()));
            }
        }
        _ => {}
    }

    Some(DepService {
        name: "db".into(),
        image: image.into(),
        ports: vec![(port, default_port)],
        env,
        volumes: Vec::new(),
    })
}

// ── 2e. Backend detection ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendKind {
    Docker,
    Wsl2Docker,
    Podman,
    None,
}

impl BackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackendKind::Docker => "docker",
            BackendKind::Wsl2Docker => "wsl2_docker",
            BackendKind::Podman => "podman",
            BackendKind::None => "none",
        }
    }
}

fn docker_available(cmd: &str) -> bool {
    let mut command = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", &format!("{} info", cmd)]);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", &format!("{} info", cmd)]);
        c
    };
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    if let Ok(status) = command.status() {
        status.success()
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
fn wsl2_docker_available() -> bool {
    let mut command = std::process::Command::new("wsl");
    command.args(["--", "docker", "info"]);
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::null());
    if let Ok(status) = command.status() {
        status.success()
    } else {
        false
    }
}

pub fn detect_backend() -> BackendKind {
    #[cfg(target_os = "windows")]
    {
        if wsl2_docker_available() {
            return BackendKind::Wsl2Docker;
        }
        if docker_available("docker") {
            return BackendKind::Docker;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if docker_available("docker") {
            return BackendKind::Docker;
        }
        if docker_available("podman") {
            return BackendKind::Podman;
        }
    }
    BackendKind::None
}

// ── 2f. Start dependency service ────────────────────────────────────

fn rewrite_volume(vol: &str, project_name: &str) -> String {
    if let Some(idx) = vol.find(':') {
        let host = &vol[..idx];
        let container = &vol[idx+1..];
        
        let is_path = host.starts_with('/') 
            || host.starts_with("./") 
            || host.starts_with("../") 
            || host.starts_with('~')
            || (host.len() >= 2 && host.as_bytes()[1] == b':');
            
        if !is_path {
            return format!("crush_{}_{}:{}", project_name, host, container);
        }
    }
    vol.to_string()
}

async fn run_docker_command(backend: &BackendKind, args: &[String]) -> anyhow::Result<String> {
    let mut cmd = match backend {
        BackendKind::Wsl2Docker => {
            let mut c = tokio::process::Command::new("wsl");
            c.arg("--").arg("docker");
            c
        }
        BackendKind::Docker => tokio::process::Command::new("docker"),
        BackendKind::Podman => tokio::process::Command::new("podman"),
        BackendKind::None => {
            anyhow::bail!("No container backend available");
        }
    };
    
    cmd.args(args);
    let output = cmd.output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    
    if !output.status.success() {
        let err_lower = stderr.to_lowercase();
        if err_lower.contains("already in use") || err_lower.contains("already exists") {
            return Ok(stdout);
        }
        anyhow::bail!("Docker command failed: {}\n{}", stdout, stderr);
    }
    
    Ok(stdout)
}

pub async fn start_dep_service(backend: &BackendKind, dep: &DepService, project_name: &str) -> anyhow::Result<String> {
    let container_name = format!("crush_{}_{}", project_name, dep.name);
    
    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.clone(),
    ];
    
    for &(host_port, container_port) in &dep.ports {
        args.push("-p".to_string());
        args.push(format!("{}:{}", host_port, container_port));
    }
    
    for (key, val) in &dep.env {
        args.push("-e".to_string());
        args.push(format!("{}={}", key, val));
    }
    
    for vol in &dep.volumes {
        let rewritten = rewrite_volume(vol, project_name);
        args.push("-v".to_string());
        args.push(rewritten);
    }
    
    args.push(dep.image.clone());
    
    match run_docker_command(backend, &args).await {
        Ok(_) => Ok(container_name),
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("already in use") || err_str.contains("already exists") {
                Ok(container_name)
            } else {
                Err(e)
            }
        }
    }
}

// ── 2g. Stop dependency service ─────────────────────────────────────

pub async fn stop_dep_service(backend: &BackendKind, container_name: &str) -> anyhow::Result<()> {
    let stop_args = vec!["stop".to_string(), container_name.to_string()];
    let rm_args = vec!["rm".to_string(), "-f".to_string(), container_name.to_string()];
    
    let _ = run_docker_command(backend, &stop_args).await;
    let _ = run_docker_command(backend, &rm_args).await;
    
    Ok(())
}

// ── 2h. Rewrite environment hostnames ────────────────────────────────

pub fn rewrite_env_hostnames(env: &[(String, String)], service_names: &[String]) -> Vec<(String, String)> {
    let mut rewritten = Vec::new();
    for (k, v) in env {
        let mut val = v.clone();
        for name in service_names {
            val = val.replace(&format!("://{}/", name), "://localhost/");
            val = val.replace(&format!("://{}:", name), "://localhost:");
            val = val.replace(&format!("@{}:", name), "@localhost:");
            val = val.replace(&format!("@{}/", name), "@localhost/");
            val = val.replace(&format!("host={}", name), "host=localhost");
            val = val.replace(&format!("hostname={}", name), "hostname=localhost");
            
            if val == *name {
                val = "localhost".to_string();
            }
        }
        rewritten.push((k.clone(), val));
    }
    rewritten
}

// ── 2i. State persistence ───────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceState {
    pub project: String,
    pub backend: String,
    pub containers: Vec<RunningContainer>,
    pub started_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunningContainer {
    pub service_name: String,
    pub container_name: String,
    pub ports: Vec<(u16, u16)>,
}

pub fn save_service_state(state_dir: &Path, state: &ServiceState) -> anyhow::Result<()> {
    fs::create_dir_all(state_dir)?;
    let file_path = state_dir.join(format!("{}.json", state.project));
    let serialized = serde_json::to_string_pretty(state)?;
    fs::write(file_path, serialized)?;
    Ok(())
}

pub fn load_service_state(state_dir: &Path, project: &str) -> Option<ServiceState> {
    let file_path = state_dir.join(format!("{}.json", project));
    if !file_path.exists() {
        return None;
    }
    let content = fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn clear_service_state(state_dir: &Path, project: &str) {
    let file_path = state_dir.join(format!("{}.json", project));
    let _ = fs::remove_file(file_path);
}

// ── v0.7.0 Native Integration ──────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StartedService {
    Native(crush_services::RunningService),
    Container(String),
}

pub fn native_driver_for(image: &str) -> Option<&'static str> {
    let name = image.split('/').last().unwrap_or(image).split(':').next().unwrap_or(image);
    match name {
        // pgvector and timescale go native too. The PostgresDriver detects
        // these via the image hint and builds/installs the extension into
        // the host PG before returning (see extensions::pgvector). User
        // can fall back to container backend with CRUSH_PGVECTOR_DOCKER=1.
        "postgres" | "pgvector" | "timescale" => Some("postgres"),
        n if n.starts_with("redis") || n.starts_with("valkey") || n.starts_with("keydb") || n.starts_with("garnet") => Some("redis"),
        "mongo" | "mongodb" => Some("mongodb"),
        "mysql" | "mariadb" => Some("mysql"),
        n if n.starts_with("minio") => Some("minio"),
        _ => None,
    }
}

pub async fn start_dep_service_smart(
    dep: &DepService,
    project_name: &str,
    data_dir: &Path,
) -> anyhow::Result<StartedService> {
    if let Some(driver_name) = native_driver_for(&dep.image) {
        let cache_dir = data_dir.join("cache");
        let svc_data_dir = data_dir.join("services").join("data").join(&dep.name);
        fs::create_dir_all(&svc_data_dir).ok();

        let host_port = dep.ports.iter().next().map(|(hp, _)| *hp).unwrap_or(0);
        let password = dep.env.iter().find(|(k, _)| k == "POSTGRES_PASSWORD" || k == "REDIS_PASSWORD" || k == "MONGO_INITDB_ROOT_PASSWORD" || k == "MINIO_ROOT_PASSWORD" || k == "MINIO_SECRET_KEY" || k == "MYSQL_ROOT_PASSWORD").map(|(_, v)| v.clone());
        let database = dep.env.iter().find(|(k, _)| k == "POSTGRES_DB" || k == "MONGO_INITDB_DATABASE" || k == "MYSQL_DATABASE").map(|(_, v)| v.clone());
        let user = dep.env.iter().find(|(k, _)| k == "POSTGRES_USER" || k == "MONGO_INITDB_ROOT_USERNAME" || k == "MINIO_ROOT_USER" || k == "MINIO_ACCESS_KEY" || k == "MYSQL_USER").map(|(_, v)| v.clone());

        let log_dir = data_dir.join("services").join("logs").join(project_name);
        let _ = fs::create_dir_all(&log_dir);
        let log_file = Some(log_dir.join(format!("{}.log", dep.name)));

        let config = crush_services::ServiceConfig {
            port: if host_port > 0 { host_port } else {
                match driver_name {
                    "postgres" => 5432,
                    "redis" => 6379,
                    "mongodb" => 27017,
                    "minio" => 9000,
                    "mysql" => 3306,
                    _ => 8080,
                }
            },
            user,
            password,
            database,
            extra_env: dep.env.clone(),
            log_file,
            image: dep.image.clone(),
        };

        if driver_name == "postgres" {
            let driver = crush_services::PostgresDriver::new(cache_dir.clone());
            driver.ensure_ready(&svc_data_dir, &cache_dir).await?;
            let running = driver.start(&config, &svc_data_dir).await?;
            return Ok(StartedService::Native(running));
        } else if driver_name == "redis" {
            let driver = crush_services::RedisCompatDriver::new(cache_dir.clone());
            driver.ensure_ready(&svc_data_dir, &cache_dir).await?;
            let running = driver.start(&config, &svc_data_dir).await?;
            return Ok(StartedService::Native(running));
        } else if driver_name == "mongodb" {
            let driver = crush_services::MongoDriver::new(cache_dir.clone());
            driver.ensure_ready(&svc_data_dir, &cache_dir).await?;
            let running = driver.start(&config, &svc_data_dir).await?;
            return Ok(StartedService::Native(running));
        } else if driver_name == "minio" {
            let driver = crush_services::MinioDriver::new(cache_dir.clone());
            driver.ensure_ready(&svc_data_dir, &cache_dir).await?;
            let running = driver.start(&config, &svc_data_dir).await?;
            return Ok(StartedService::Native(running));
        }
    }

    let backend = detect_backend();
    match backend {
        BackendKind::Docker | BackendKind::Wsl2Docker | BackendKind::Podman => {
            let cname = start_dep_service(&backend, dep, project_name).await?;
            Ok(StartedService::Container(cname))
        }
        BackendKind::None => {
            anyhow::bail!("No container or native backend available. Please install Docker, Podman, or ensure the service matches a supported native runtime.");
        }
    }
}
