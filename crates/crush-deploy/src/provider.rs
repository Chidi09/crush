use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub provider: String,
    pub project: String,
    pub server_id: String,
    pub public_ip: String,
    pub region: String,
    pub deployed_at: String,
    pub image_digest: String,
    pub port: u16,
    pub domain: Option<String>,
    pub status: DeployStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeployStatus {
    Provisioning,
    Running,
    Stopped,
    Failed(String),
}

#[async_trait]
pub trait DeployProvider: Send + Sync {
    async fn provision(&self, project: &str, region: &str, size: &str) -> anyhow::Result<DeploymentInfo>;
    async fn deploy(&self, info: &DeploymentInfo, image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()>;
    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()>;
    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus>;
    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String>;
}
