use crate::check::{CheckAction, CheckStatus, DoctorCheck};
use async_trait::async_trait;
use std::path::PathBuf;

pub struct ManifestDriftCheck {
    pub manifest_path: PathBuf,
}

impl ManifestDriftCheck {
    pub fn new(manifest_path: impl Into<PathBuf>) -> Self {
        Self {
            manifest_path: manifest_path.into(),
        }
    }
}

#[async_trait]
impl DoctorCheck for ManifestDriftCheck {
    fn name(&self) -> &'static str {
        "Lockfile-Manifest Drift"
    }

    fn description(&self) -> &'static str {
        "Checks if the package manifest (e.g. package.json) is out of sync with the lockfile."
    }

    async fn check(&self) -> anyhow::Result<CheckStatus> {
        // R2.3: Lockfile <-> manifest drift
        // Parse manifest deps, confirm each resolves in the lockfile and versions satisfy ranges.
        // Mock drift detected
        Ok(CheckStatus::Warning(format!(
            "Manifest at {} is out of sync with lockfile",
            self.manifest_path.display()
        )))
    }

    fn fix(&self) -> Option<CheckAction> {
        Some(CheckAction {
            label: "Auto-sync lockfile".to_string(),
            command: "npm".to_string(), // In reality, detect pm
            args: vec!["install".to_string(), "--package-lock-only".to_string()],
        })
    }
}
