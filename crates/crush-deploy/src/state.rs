use std::path::PathBuf;
use super::provider::DeploymentInfo;

pub struct DeploymentState {
    dir: PathBuf,
}

impl DeploymentState {
    pub fn new() -> Self {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".crush")
            .join("deployments");
        std::fs::create_dir_all(&dir).ok();
        Self { dir }
    }

    fn path(&self, project: &str) -> PathBuf {
        self.dir.join(format!("{}.json", project))
    }

    pub fn load(&self, project: &str) -> Option<DeploymentInfo> {
        let content = std::fs::read_to_string(self.path(project)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(info)?;
        std::fs::write(self.path(&info.project), json)?;
        Ok(())
    }

    pub fn remove(&self, project: &str) {
        let _ = std::fs::remove_file(self.path(project));
    }

    pub fn list(&self) -> Vec<DeploymentInfo> {
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(info) = serde_json::from_str::<DeploymentInfo>(&content) {
                        result.push(info);
                    }
                }
            }
        }
        result
    }
}
