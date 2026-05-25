use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use serde::Deserialize;
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeV2 {
    pub version: Option<String>,
    pub services: Option<HashMap<String, ComposeService>>,
    pub networks: Option<HashMap<String, ComposeNetwork>>,
    pub volumes: Option<HashMap<String, ComposeVolume>>,
    pub configs: Option<HashMap<String, ComposeConfig>>,
    pub secrets: Option<HashMap<String, ComposeSecret>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeService {
    pub image: Option<String>,
    pub build: Option<serde_json::Value>,
    pub ports: Option<Vec<String>>,
    pub environment: Option<serde_json::Value>,
    pub depends_on: Option<serde_json::Value>,
    pub volumes: Option<Vec<String>>,
    pub command: Option<serde_json::Value>,
    pub entrypoint: Option<serde_json::Value>,
    pub restart: Option<String>,
    pub container_name: Option<String>,
    pub networks: Option<Vec<String>>,
    pub env_file: Option<Vec<String>>,
    pub healthcheck: Option<ComposeHealthcheck>,
    pub deploy: Option<ComposeDeploy>,
    pub profiles: Option<Vec<String>>,
    pub user: Option<String>,
    pub working_dir: Option<String>,
    pub labels: Option<serde_json::Value>,
    pub logging: Option<serde_json::Value>,
    pub cap_add: Option<Vec<String>>,
    pub cap_drop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeHealthcheck {
    pub test: Option<serde_json::Value>,
    pub interval: Option<String>,
    pub timeout: Option<String>,
    pub retries: Option<u32>,
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeDeploy {
    pub resources: Option<ComposeResources>,
    pub replicas: Option<u32>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeResources {
    pub limits: Option<ComposeResourceLimits>,
    pub reservations: Option<ComposeResourceLimits>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeResourceLimits {
    pub cpus: Option<String>,
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeNetwork {
    pub driver: Option<String>,
    pub driver_opts: Option<HashMap<String, String>>,
    pub ipam: Option<serde_json::Value>,
    pub external: Option<bool>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeVolume {
    pub driver: Option<String>,
    pub driver_opts: Option<HashMap<String, String>>,
    pub external: Option<bool>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeConfig {
    pub file: Option<String>,
    pub external: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComposeSecret {
    pub file: Option<String>,
    pub external: Option<bool>,
}

pub struct ComposeParser;

impl ComposeParser {
    pub fn new() -> Self { Self }

    pub fn parse_path(&self, path: &Path) -> Result<ComposeV2> {
        let content = fs::read_to_string(path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read compose file: {}", e)))?;
        self.parse(&content, path)
    }

    pub fn parse(&self, content: &str, path: &Path) -> Result<ComposeV2> {
        let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        if ext != "yml" && ext != "yaml" {
            return Err(CrushError::ImageError("Compose file must have .yml or .yaml extension".to_string()));
        }

        let compose: ComposeV2 = serde_yaml::from_str(content)
            .map_err(|e| CrushError::ImageError(format!("Compose YAML parse error: {} (file: {:?})", e, path)))?;

        Ok(compose)
    }

    pub fn get_service_names(compose: &ComposeV2) -> Vec<String> {
        compose.services.as_ref()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_dependency_order(compose: &ComposeV2) -> Result<Vec<String>> {
        let services = match compose.services.as_ref() {
            Some(s) => s,
            None => return Ok(Vec::new()),
        };

        let mut visited = HashMap::new();
        let mut order = Vec::new();

        for name in services.keys() {
            if !visited.contains_key(name) {
                visit_deps(services, name, &mut visited, &mut order)?;
            }
        }

        Ok(order)
    }
}

fn visit_deps(
    services: &HashMap<String, ComposeService>,
    name: &str,
    visited: &mut HashMap<String, bool>,
    order: &mut Vec<String>,
) -> Result<()> {
    if let Some(&in_progress) = visited.get(name) {
        if in_progress {
            return Err(CrushError::ImageError(format!(
                "Circular dependency detected in docker-compose.yml involving service '{}'", name
            )));
        }
        return Ok(());
    }

    visited.insert(name.to_string(), true);

    if let Some(service) = services.get(name) {
        let deps = match &service.depends_on {
            Some(d) if d.is_string() => vec![d.as_str().unwrap().to_string()],
            Some(d) if d.is_array() => {
                d.as_array().unwrap().iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
            }
            Some(d) if d.is_object() => {
                d.as_object().unwrap().keys().cloned().collect()
            }
            _ => Vec::new(),
        };

        for dep in &deps {
            if !services.contains_key(dep) {
                continue;
            }
            visit_deps(services, dep, visited, order)?;
        }
    }

    visited.insert(name.to_string(), false);
    order.push(name.to_string());
    Ok(())
}
