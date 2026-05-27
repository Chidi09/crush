use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};

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
            
            let is_dep = is_dep_image(&image) || !has_local_build(&svc.build);
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
