use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use anyhow::Result;

pub mod local;
pub mod mount;

pub use local::LocalDriver;
pub use mount::VolumeMounter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub labels: HashMap<String, String>,
    pub ref_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MountType {
    #[serde(rename = "named")]
    Named,
    #[serde(rename = "bind")]
    Bind,
    #[serde(rename = "tmpfs")]
    Tmpfs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountRecord {
    pub source: String,
    pub destination: PathBuf,
    pub mount_type: MountType,
    pub readonly: bool,
}

#[async_trait]
pub trait VolumeDriver: Send + Sync {
    async fn create(&self, name: &str, labels: HashMap<String, String>) -> Result<VolumeInfo>;
    async fn remove(&self, name: &str) -> Result<()>;
    async fn list(&self) -> Result<Vec<VolumeInfo>>;
    async fn inspect(&self, name: &str) -> Result<VolumeInfo>;
    async fn path(&self, name: &str) -> Result<PathBuf>;
}
