use crate::check::{CheckAction, CheckStatus, DoctorCheck};
use async_trait::async_trait;

pub struct ToolchainCheck {
    pub expected_lang: String,
    pub expected_version: String,
}

impl ToolchainCheck {
    pub fn new(lang: &str, version: &str) -> Self {
        Self {
            expected_lang: lang.to_string(),
            expected_version: version.to_string(),
        }
    }
}

#[async_trait]
impl DoctorCheck for ToolchainCheck {
    fn name(&self) -> &'static str {
        "Toolchain Present & Matches Pin"
    }

    fn description(&self) -> &'static str {
        "Ensures that the required runtime is installed and matches the exact pinned version."
    }

    async fn check(&self) -> anyhow::Result<CheckStatus> {
        // Mock `crush-toolchain` integration
        // In reality, this would call crush_toolchain::detect() and check against cache
        // For now, return a mock failure to test the fix flow, or just pass
        Ok(CheckStatus::Warning(format!(
            "Toolchain for {} {} not found locally",
            self.expected_lang, self.expected_version
        )))
    }

    fn fix(&self) -> Option<CheckAction> {
        Some(CheckAction {
            label: format!(
                "Install {} {}",
                self.expected_lang, self.expected_version
            ),
            command: "crush".to_string(),
            args: vec![
                "toolchain".to_string(),
                "install".to_string(),
                self.expected_lang.clone(),
                self.expected_version.clone(),
            ],
        })
    }
}
