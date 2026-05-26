pub mod health;
pub mod restart;
pub mod oom;
pub mod secrets;
pub mod rootless;
pub mod apparmor;
pub mod selinux;
pub mod audit;
pub mod readonly;

use std::path::Path;
use crush_types::{Result, CrushError};

pub use health::{HealthChecker, HealthCheckConfig, HealthCheckType, HealthState};
pub use crush_types::HealthStatus;
pub use restart::{RestartManager, RestartPolicy};
pub use oom::{OomMonitor, OomEvent, OomPolicy};
pub use secrets::{SecretManager, SecretSpec, SecretValue, SecretSource, SecretDestination, VaultEngine};
pub use rootless::RootlessManager;
pub use apparmor::AppArmorProfile;
pub use selinux::{SelinuxManager, SelinuxMode};
pub use audit::{AuditLogger, AuditEvent, AuditEventType};
pub use readonly::ReadOnlyRootfs;

pub struct ReliabilityEngine {
    pub health: Option<HealthChecker>,
    pub restart: RestartManager,
    pub oom: OomMonitor,
    pub audit: AuditLogger,
    pub secrets: SecretManager,
}

impl ReliabilityEngine {
    pub fn new(data_dir: &Path, container_id: &str, restart_policy: RestartPolicy) -> Self {
        let secrets_dir = data_dir.join("secrets").join(container_id);
        std::fs::create_dir_all(&secrets_dir).ok();

        Self {
            health: None,
            restart: RestartManager::new(restart_policy),
            oom: OomMonitor::new(container_id, OomPolicy::ReportOnly),
            audit: AuditLogger::new(data_dir),
            secrets: SecretManager::new(secrets_dir),
        }
    }

    pub fn with_health_check(mut self, config: health::HealthCheckConfig) -> Self {
        self.health = Some(HealthChecker::new(config));
        self
    }

    pub fn with_vault(mut self, addr: String, token: String) -> Self {
        self.secrets = self.secrets.with_vault(addr, token);
        self
    }

    pub fn record_restart(&mut self, exit_code: i32, explicitly_stopped: bool) -> bool {
        let should = self.restart.should_restart(exit_code, explicitly_stopped);
        if should {
            self.restart.record_attempt();
            self.audit.log(AuditLogger::event(
                audit::AuditEventType::RestartAttempt,
                None, None,
                format!("Restart attempt {} (exit code {})", self.restart.attempt(), exit_code),
            )).ok();
        }
        should
    }
}
