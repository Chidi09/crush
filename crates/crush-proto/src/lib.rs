use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ContainerStatusRequest {
    pub container_id: String,
    pub verbose: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ContainerStatusResponse {
    pub status: String,
    pub exit_code: i32,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CreateContainerRequest {
    pub image: String,
    pub command: Vec<String>,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_shares: Option<u64>,
    pub env: HashMap<String, String>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct CreateContainerResponse {
    pub container_id: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ListContainersRequest {
    pub all: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ContainerInfo {
    pub container_id: String,
    pub image: String,
    pub status: String,
    pub pid: Option<u32>,
    pub created_at: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ExecRequest {
    pub container_id: String,
    pub command: Vec<String>,
    pub tty: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

struct ContainerRecord {
    id: String,
    image: String,
    status: String,
    pid: Option<u32>,
    exit_code: i32,
    created_at: String,
    env: HashMap<String, String>,
    memory_limit_bytes: Option<u64>,
    cpu_shares: Option<u64>,
}

pub struct CriService {
    containers: Arc<Mutex<HashMap<String, ContainerRecord>>>,
}

impl CriService {
    pub fn new() -> Self {
        Self {
            containers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_container(&self, req: CreateContainerRequest) -> CreateContainerResponse {
        let id = format!("crush_{}", hex_encode_random());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        let record = ContainerRecord {
            id: id.clone(),
            image: req.image.clone(),
            status: "Created".to_string(),
            pid: None,
            exit_code: 0,
            created_at: now,
            env: req.env,
            memory_limit_bytes: req.memory_limit_bytes,
            cpu_shares: req.cpu_shares,
        };

        let mut containers = self.containers.lock().await;
        containers.insert(id.clone(), record);

        CreateContainerResponse { container_id: id }
    }

    pub async fn start_container(&self, container_id: &str) -> Result<(), String> {
        let mut containers = self.containers.lock().await;
        if let Some(container) = containers.get_mut(container_id) {
            container.status = "Running".to_string();
            container.pid = Some(rand_pid());
            Ok(())
        } else {
            Err(format!("Container {} not found", container_id))
        }
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<(), String> {
        let mut containers = self.containers.lock().await;
        if let Some(container) = containers.get_mut(container_id) {
            container.status = "Stopped".to_string();
            container.exit_code = 0;
            container.pid = None;
            Ok(())
        } else {
            Err(format!("Container {} not found", container_id))
        }
    }

    pub async fn get_container_status(&self, req: ContainerStatusRequest) -> ContainerStatusResponse {
        let containers = self.containers.lock().await;
        if let Some(container) = containers.get(&req.container_id) {
            let mut metadata = HashMap::new();
            if req.verbose {
                metadata.insert("image".to_string(), container.image.clone());
                metadata.insert("created_at".to_string(), container.created_at.clone());
                if let Some(mem) = container.memory_limit_bytes {
                    metadata.insert("memory_limit".to_string(), mem.to_string());
                }
                if let Some(cpu) = container.cpu_shares {
                    metadata.insert("cpu_shares".to_string(), cpu.to_string());
                }
                for (k, v) in &container.env {
                    metadata.insert(format!("env_{}", k), v.clone());
                }
            }
            ContainerStatusResponse {
                status: container.status.clone(),
                exit_code: container.exit_code,
                metadata,
            }
        } else {
            ContainerStatusResponse {
                status: "NotFound".to_string(),
                exit_code: -1,
                metadata: HashMap::new(),
            }
        }
    }

    pub async fn list_containers(&self, req: ListContainersRequest) -> Vec<ContainerInfo> {
        let containers = self.containers.lock().await;
        containers.values().filter(|c| {
            req.all || c.status == "Running"
        }).map(|c| ContainerInfo {
            container_id: c.id.clone(),
            image: c.image.clone(),
            status: c.status.clone(),
            pid: c.pid,
            created_at: c.created_at.clone(),
        }).collect()
    }

    pub async fn delete_container(&self, container_id: &str) -> Result<(), String> {
        let mut containers = self.containers.lock().await;
        if containers.remove(container_id).is_some() {
            Ok(())
        } else {
            Err(format!("Container {} not found", container_id))
        }
    }
}

fn hex_encode_random() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let combined = (pid as u128).wrapping_mul(3141592653589793238u128).wrapping_add(nanos);
    format!("{:032x}", combined)
}

fn rand_pid() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (nanos as u32 % 65535) + 1
}
