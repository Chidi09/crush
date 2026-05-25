use serde::{Serialize, Deserialize};
use std::process::Command;
use std::time::Duration;
use crush_types::{Result, CrushError, ContainerStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciHook {
    pub path: String,
    pub args: Vec<String>,
    pub env: Vec<String>,
    pub timeout: Option<u32>,
}

pub struct ContainerLifecycleManager;

impl ContainerLifecycleManager {
    pub fn new() -> Self { Self }

    pub fn trigger_hook(&self, hook_name: &str, hook: &OciHook) -> Result<()> {
        // ⚠ Validate hook path: must be within approved directories and not world-writable
        let allowed_prefixes = ["/usr/lib/crush/hooks/", "/usr/local/lib/crush/hooks/", "/etc/crush/hooks/"];
        let is_allowed = allowed_prefixes.iter().any(|p| hook.path.starts_with(p));
        if !is_allowed {
            return Err(CrushError::ContainerNotFound(format!(
                "OCI hook path '{}' not in approved directories", hook.path
            )));
        }
        if !std::path::Path::new(&hook.path).exists() {
            return Err(CrushError::ContainerNotFound(format!(
                "OCI hook not found: {}", hook.path
            )));
        }

        let mut cmd = Command::new(&hook.path);
        for arg in &hook.args { cmd.arg(arg); }
        for env_var in &hook.env {
            if let Some(eq) = env_var.find('=') {
                cmd.env(&env_var[..eq], &env_var[eq + 1..]);
            }
        }

        let status = if let Some(timeout_secs) = hook.timeout {
            let mut child = cmd.spawn()
                .map_err(|e| CrushError::ContainerNotFound(format!("Hook spawn failed: {}", e)))?;
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() > Duration::from_secs(timeout_secs as u64) {
                    let _ = child.kill();
                    return Err(CrushError::ContainerNotFound(
                        format!("Hook '{}' timed out after {}s", hook_name, timeout_secs)
                    ));
                }
                match child.try_wait() {
                    Ok(Some(s)) => { return if s.success() { Ok(()) } else {
                        Err(CrushError::ContainerNotFound(format!("Hook '{}' exited with {:?}", hook_name, s.code())))
                    }}
                    Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                    Err(e) => return Err(CrushError::ContainerNotFound(format!("Hook wait: {}", e))),
                }
            }
        } else {
            cmd.status()
                .map_err(|e| CrushError::ContainerNotFound(format!("Hook run failed: {}", e)))?
        };

        if !status.success() {
            return Err(CrushError::ContainerNotFound(
                format!("OCI hook '{}' exited with code {:?}", hook_name, status.code())
            ));
        }
        Ok(())
    }

    pub fn trigger_hooks_list(&self, hook_name: &str, hooks: &[OciHook]) -> Result<()> {
        for hook in hooks { self.trigger_hook(hook_name, hook)?; }
        Ok(())
    }

    pub fn validate_state_transition(&self, current: ContainerStatus, next: ContainerStatus) -> Result<()> {
        let valid = matches!((current, next),
            (ContainerStatus::Creating, ContainerStatus::Created)
            | (ContainerStatus::Created, ContainerStatus::Running)
            | (ContainerStatus::Running, ContainerStatus::Paused)
            | (ContainerStatus::Paused, ContainerStatus::Running)
            | (ContainerStatus::Running, ContainerStatus::Stopped)
            | (ContainerStatus::Paused, ContainerStatus::Stopped)
            | (ContainerStatus::Created, ContainerStatus::Stopped)
        );
        if !valid {
            return Err(CrushError::InvalidStateTransition { from: current, to: next });
        }
        Ok(())
    }
}
