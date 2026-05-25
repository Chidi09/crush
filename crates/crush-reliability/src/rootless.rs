use std::path::Path;
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct RootlessManager;

impl RootlessManager {
    pub fn check_support() -> bool {
        Path::new("/etc/subuid").exists()
            && Path::new("/etc/subgid").exists()
    }

    pub fn has_newuidmap() -> bool {
        let out = Command::new("which").arg("newuidmap").output();
        matches!(out, Ok(o) if o.status.success())
    }

    pub fn has_newgidmap() -> bool {
        let out = Command::new("which").arg("newgidmap").output();
        matches!(out, Ok(o) if o.status.success())
    }

    pub fn read_subuid(username: &str) -> Result<(u32, u32)> {
        let content = std::fs::read_to_string("/etc/subuid")
            .map_err(|e| CrushError::NamespaceError(format!("Cannot read /etc/subuid: {}", e)))?;
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 3 && parts[0] == username {
                let start: u32 = parts[1].parse().map_err(|_| CrushError::CgroupError("Invalid subuid".into()))?;
                let count: u32 = parts[2].parse().map_err(|_| CrushError::CgroupError("Invalid subuid count".into()))?;
                return Ok((start, count));
            }
        }
        Err(CrushError::NamespaceError(format!("No subuid range found for user '{}'", username)))
    }

    pub fn read_subgid(username: &str) -> Result<(u32, u32)> {
        let content = std::fs::read_to_string("/etc/subgid")
            .map_err(|e| CrushError::NamespaceError(format!("Cannot read /etc/subgid: {}", e)))?;
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 3 && parts[0] == username {
                let start: u32 = parts[1].parse().map_err(|_| CrushError::CgroupError("Invalid subgid".into()))?;
                let count: u32 = parts[2].parse().map_err(|_| CrushError::CgroupError("Invalid subgid count".into()))?;
                return Ok((start, count));
            }
        }
        Err(CrushError::NamespaceError(format!("No subgid range found for user '{}'", username)))
    }

    pub fn map_ids(child_pid: u32, username: &str) -> Result<()> {
        let (uid_start, uid_count) = Self::read_subuid(username)?;
        let (gid_start, gid_count) = Self::read_subgid(username)?;

        let setgroups_path = format!("/proc/{}/setgroups", child_pid);
        std::fs::write(&setgroups_path, "deny")
            .map_err(|e| CrushError::NamespaceError(format!(
                "Failed to write setgroups deny for PID {}: {}", child_pid, e
            )))?;

        let uid_map = format!("0 {} 1\n1 {} {}\n", Self::current_uid(), uid_start, uid_count);
        let uid_path = format!("/proc/{}/uid_map", child_pid);
        std::fs::write(&uid_path, &uid_map)
            .map_err(|e| CrushError::NamespaceError(format!("uid_map write failed: {}", e)))?;

        let gid_map = format!("0 {} 1\n1 {} {}\n", Self::current_gid(), gid_start, gid_count);
        let gid_path = format!("/proc/{}/gid_map", child_pid);
        std::fs::write(&gid_path, &gid_map)
            .map_err(|e| CrushError::NamespaceError(format!("gid_map write failed: {}", e)))?;

        Ok(())
    }

    pub fn current_uid() -> u32 {
        #[cfg(unix)] { unsafe { libc::getuid() } }
        #[cfg(not(unix))] { 1000 }
    }

    pub fn current_gid() -> u32 {
        #[cfg(unix)] { unsafe { libc::getgid() } }
        #[cfg(not(unix))] { 1000 }
    }
}
