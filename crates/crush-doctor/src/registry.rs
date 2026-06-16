use crate::check::{CheckAction, CheckStatus, DoctorCheck};
use anyhow::Result;

pub struct CheckResult {
    pub name: String,
    pub description: String,
    pub status: CheckStatus,
    pub fix: Option<CheckAction>,
}

pub struct CheckRegistry {
    checks: Vec<Box<dyn DoctorCheck>>,
}

impl Default for CheckRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckRegistry {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn register(&mut self, check: Box<dyn DoctorCheck>) {
        self.checks.push(check);
    }

    pub async fn run_all(&self) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        for check in &self.checks {
            let status = check.check().await?;
            let fix = if matches!(status, CheckStatus::Fail(_) | CheckStatus::Warning(_)) {
                check.fix()
            } else {
                None
            };

            results.push(CheckResult {
                name: check.name().to_string(),
                description: check.description().to_string(),
                status,
                fix,
            });
        }
        Ok(results)
    }
}
