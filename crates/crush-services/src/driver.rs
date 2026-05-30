use std::path::{Path, PathBuf};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub port: u16,
    pub user: Option<String>,       // for postgres: superuser name
    pub password: Option<String>,   // for postgres: superuser password
    pub database: Option<String>,   // for postgres: initial db name
    pub extra_env: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_file: Option<PathBuf>,  // stdout/stderr redirect target
    /// Original image hint (e.g. "pgvector/pgvector:pg17") so the driver
    /// can fork behaviour — install the `vector` extension into the host
    /// PG before returning, etc. Empty for vanilla postgres.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub image: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningService {
    pub name: String,
    pub pid: u32,
    pub port: u16,
    pub data_dir: PathBuf,
    pub kind: ServiceKind,
    #[serde(default)]
    pub console_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceKind { Postgres, RedisCompat, MySQL, MongoDB, ObjectStore }

#[async_trait]
pub trait ServiceDriver: Send + Sync {
    fn name(&self) -> &'static str;
    fn default_port(&self) -> u16;
    /// Download binary if not cached, initialise data dir if needed.
    async fn ensure_ready(&self, data_dir: &Path, cache_dir: &Path) -> Result<()>;
    /// Start the service process. Returns immediately after process is spawned.
    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService>;
    /// Send graceful stop signal and wait up to 5s.
    async fn stop(&self, service: &RunningService) -> Result<()>;
    /// Returns true if the process is still alive.
    async fn is_alive(&self, service: &RunningService) -> bool;
    /// Wait until the service accepts connections, up to timeout_ms.
    async fn wait_ready(&self, service: &RunningService, timeout_ms: u64) -> bool;
}
