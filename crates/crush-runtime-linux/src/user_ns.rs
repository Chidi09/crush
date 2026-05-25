use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct UserNamespaceManager {
    uid: u32,
    gid: u32,
}

impl UserNamespaceManager {
    pub fn new() -> Self {
        #[cfg(unix)]
        let uid = unsafe { libc::getuid() };
        #[cfg(unix)]
        let gid = unsafe { libc::getgid() };
        #[cfg(not(unix))]
        let (uid, gid) = (1000, 1000);
        Self { uid, gid }
    }

    pub fn parse_subuid_range(&self, username: &str) -> Result<(u32, u32)> {
        let file_path = "/etc/subuid";
        let file = File::open(file_path)
            .map_err(|e| CrushError::NamespaceError(format!("Failed to open {}: {}", file_path, e)))?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap_or_default();
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 3 && parts[0] == username {
                let start: u32 = parts[1].parse().unwrap_or(0);
                let count: u32 = parts[2].parse().unwrap_or(0);
                return Ok((start, count));
            }
        }
        Ok((100000, 65536))
    }

    pub fn write_uid_gid_maps(&self, child_pid: u32, username: &str) -> Result<()> {
        let (subuid_start, subuid_count) = self.parse_subuid_range(username)?;

        // ⚠ CRITICAL: Write "deny" to setgroups BEFORE gid_map.
        // Without this, the child can call setgroups(2) to drop supplementary GIDs
        // and gain host file access (CVE-2014-8989). Failure here must abort the setup.
        let setgroups_path = format!("/proc/{}/setgroups", child_pid);
        std::fs::write(&setgroups_path, "deny")
            .map_err(|e| CrushError::NamespaceError(format!(
                "Failed to write setgroups deny for PID {}: {}", child_pid, e
            )))?;

        let uid_status = Command::new("newuidmap")
            .arg(child_pid.to_string())
            .arg("0").arg(self.uid.to_string()).arg("1")
            .arg("1").arg(subuid_start.to_string()).arg(subuid_count.to_string())
            .status()
            .map_err(|e| CrushError::NamespaceError(format!("newuidmap failed: {}", e)))?;

        if uid_status.success() {
            println!("UserNS: UID map applied to PID {} via newuidmap", child_pid);
        } else {
            let map_file = format!("/proc/{}/uid_map", child_pid);
            let map_content = format!("0 {} 1\n1 {} {}\n", self.uid, subuid_start, subuid_count);
            std::fs::write(&map_file, &map_content)
                .map_err(|e| CrushError::NamespaceError(format!("uid_map write failed: {}", e)))?;
        }

        let gid_status = Command::new("newgidmap")
            .arg(child_pid.to_string())
            .arg("0").arg(self.gid.to_string()).arg("1")
            .arg("1").arg(subuid_start.to_string()).arg(subuid_count.to_string())
            .status()
            .map_err(|e| CrushError::NamespaceError(format!("newgidmap failed: {}", e)))?;

        if gid_status.success() {
            println!("UserNS: GID map applied to PID {} via newgidmap", child_pid);
        } else {
            let map_file = format!("/proc/{}/gid_map", child_pid);
            let map_content = format!("0 {} 1\n1 {} {}\n", self.gid, subuid_start, subuid_count);
            std::fs::write(&map_file, &map_content)
                .map_err(|e| CrushError::NamespaceError(format!("gid_map write failed: {}", e)))?;
        }

        Ok(())
    }
}
