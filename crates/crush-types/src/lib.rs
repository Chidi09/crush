use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use async_trait::async_trait;

#[derive(Error, Debug)]
pub enum CrushError {
    #[error("Namespace error: {0}")]
    NamespaceError(String),

    #[error("Cgroup error: {0}")]
    CgroupError(String),

    #[error("Seccomp profile error: {0}")]
    SeccompError(String),

    #[error("Overlay filesystem error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Container already exists: {0}")]
    ContainerAlreadyExists(String),

    #[error("Invalid container state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ContainerStatus,
        to: ContainerStatus,
    },

    #[error("Image error: {0}")]
    ImageError(String),

    #[error("OCI specification error: {0}")]
    OciSpecError(String),

    #[error("WASM runtime error: {0}")]
    WasmError(String),

    #[error("AI intelligence diagnostic failure: {0}")]
    AiError(String),

    #[error("CLI argument error: {0}")]
    CliError(String),

    #[error("API server error: {0}")]
    ApiError(String),

    #[error("Internal dynamic error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CrushError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    Creating,
    Created,
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Starting,
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub pid: Option<u32>,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountConfig>,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_shares: Option<u64>,
    pub health: Option<HealthStatus>,
    pub restart_count: Option<u32>,
    pub restart_policy: Option<String>,
    pub health_cmd: Option<String>,
    pub health_interval: Option<u64>,
    pub health_timeout: Option<u64>,
    pub health_retries: Option<u32>,
    pub pids_limit: Option<u32>,
    pub read_only: Option<bool>,
    pub security_opt: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_ip: String,
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
    pub is_tmpfs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub tag: String,
    pub digest: String,
    pub size_bytes: u64,
    pub layers: Vec<String>,
    pub architecture: String,
    pub os: String,
    #[serde(default)]
    pub entrypoint: Vec<String>,
    #[serde(default)]
    pub cmd: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub config_digest: Option<String>,
}

#[async_trait]
pub trait RuntimeBackend: Send + Sync {
    async fn create(&self, container: &Container, spec_path: &PathBuf) -> Result<()>;
    async fn start(&self, container_id: &str) -> Result<()>;
    async fn stop(&self, container_id: &str, timeout_seconds: u32) -> Result<()>;
    async fn pause(&self, container_id: &str) -> Result<()>;
    async fn resume(&self, container_id: &str) -> Result<()>;
    async fn delete(&self, container_id: &str) -> Result<()>;
    async fn exec(&self, container_id: &str, command: &[String], tty: bool) -> Result<i32>;
    async fn get_pid(&self, container_id: &str) -> Result<Option<u32>>;
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn pull_image(&self, tag: &str) -> Result<Image>;
    async fn push_image(&self, image_id: &str, registry: &str) -> Result<()>;
    async fn list_images(&self) -> Result<Vec<Image>>;
    async fn delete_image(&self, image_id: &str) -> Result<()>;
    async fn extract_layers(&self, image_id: &str, destination: &PathBuf) -> Result<()>;
}
