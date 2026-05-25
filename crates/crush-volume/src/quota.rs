use std::path::{Path, PathBuf};
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct QuotaManager;

impl QuotaManager {
    pub fn supports_project_quota(path: &Path) -> bool {
        let out = Command::new("stat")
            .args(["-f", "--format=%T", &path.to_string_lossy()])
            .output();
        match out {
            Ok(o) => {
                let fstype = String::from_utf8_lossy(&o.stdout);
                fstype.trim() == "xfs" || fstype.trim() == "ext4"
            }
            Err(_) => false,
        }
    }

    pub fn set_project_quota(volume_path: &Path, project_id: u32, limit_bytes: u64) -> Result<()> {
        if !Self::supports_project_quota(volume_path) {
            return Err(CrushError::StorageError(
                "Project quotas require XFS or ext4 filesystem".to_string()
            ));
        }

        let set_prjid = Command::new("chattr")
            .args(["-p", &project_id.to_string(), "-R", &volume_path.to_string_lossy()])
            .output();

        if let Ok(ref out) = set_prjid {
            if !out.status.success() {
                eprintln!("Warning: could not set project ID: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let limit = Command::new("setquota")
            .args([
                "-P", &project_id.to_string(),
                "0", &(limit_bytes / 1024).to_string(),
                "0", "0",
                &volume_path.to_string_lossy(),
            ])
            .output();

        if let Ok(ref out) = limit {
            if !out.status.success() {
                eprintln!("Warning: project quota set failed: {}", String::from_utf8_lossy(&out.stderr));
                // Fall back to du-based soft limit check
            }
        }

        Ok(())
    }

    pub fn get_disk_usage(path: &Path) -> Result<u64> {
        let out = Command::new("du")
            .args(["-sb", &path.to_string_lossy()])
            .output()
            .map_err(|e| CrushError::StorageError(format!("du failed: {}", e)))?;

        let output = String::from_utf8_lossy(&out.stdout);
        let size_str = output.split_whitespace().next().unwrap_or("0");
        size_str.parse::<u64>()
            .map_err(|e| CrushError::StorageError(format!("du parse failed: {}", e)))
    }

    pub fn check_soft_limit(path: &Path, limit_bytes: u64) -> Result<bool> {
        let usage = Self::get_disk_usage(path)?;
        Ok(usage <= limit_bytes)
    }

    pub fn check_and_warn(path: &Path, limit_bytes: u64, volume_name: &str) -> Result<()> {
        let usage = Self::get_disk_usage(path)?;
        if usage > limit_bytes {
            return Err(CrushError::StorageError(format!(
                "Volume '{}' exceeds size limit: {} > {} bytes",
                volume_name, usage, limit_bytes
            )));
        }
        if usage > limit_bytes.saturating_mul(9) / 10 {
            eprintln!("Warning: volume '{}' is at {}% capacity", volume_name, usage * 100 / limit_bytes);
        }
        Ok(())
    }
}
