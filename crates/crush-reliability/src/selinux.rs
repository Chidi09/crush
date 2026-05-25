use std::process::Command;
use rand::Rng;
use crush_types::{Result, CrushError};

pub struct SelinuxManager;

impl SelinuxManager {
    pub fn is_enabled() -> bool {
        Command::new("selinuxenabled")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn get_enforce_mode() -> Result<SelinuxMode> {
        if !Self::is_enabled() { return Ok(SelinuxMode::Disabled); }
        let out = Command::new("getenforce")
            .output()
            .map_err(|e| CrushError::NamespaceError(format!("getenforce: {}", e)))?;
        let mode = String::from_utf8_lossy(&out.stdout).trim().to_string();
        match mode.as_str() {
            "Enforcing" => Ok(SelinuxMode::Enforcing),
            "Permissive" => Ok(SelinuxMode::Permissive),
            _ => Ok(SelinuxMode::Disabled),
        }
    }

    pub fn generate_mcs_label() -> String {
        let cat1 = rand::thread_rng().gen_range(0u32..1024);
        let cat2 = rand::thread_rng().gen_range(0u32..1024);
        format!("s0:c{},c{}", cat1, cat2)
    }

    pub fn label_process(pid: u32, label: &str) -> Result<()> {
        Command::new("chcon")
            .args(["-l", label, &format!("/proc/{}", pid)])
            .output()
            .map_err(|e| CrushError::NamespaceError(format!("chcon: {}", e)))?;
        Ok(())
    }

    pub fn label_path(path: &str, label: &str, recursive: bool) -> Result<()> {
        let mut cmd = Command::new("chcon");
        if recursive { cmd.arg("-R"); }
        cmd.arg("-l").arg(label).arg(path)
            .output()
            .map_err(|e| CrushError::NamespaceError(format!("chcon path: {}", e)))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelinuxMode {
    Disabled,
    Permissive,
    Enforcing,
}
