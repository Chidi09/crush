use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckStatus {
    Pass,
    Fail(String),
    Warning(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckAction {
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
}

#[async_trait]
pub trait DoctorCheck: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    async fn check(&self) -> anyhow::Result<CheckStatus>;
    fn fix(&self) -> Option<CheckAction>;
}
