use std::path::{Path, PathBuf};
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct WindowsVolumeMapper;

impl WindowsVolumeMapper {
    pub fn map_windows_path(host_path: &Path) -> PathBuf {
        let path_str = host_path.to_string_lossy();
        if cfg!(target_os = "windows") {
            if path_str.starts_with("C:\\") || path_str.starts_with("c:\\") {
                PathBuf::from(format!(
                    "/host/c/{}",
                    path_str[3..].replace('\\', "/")
                ))
            } else if path_str.starts_with("\\") {
                PathBuf::from(format!(
                    "/host/{}",
                    path_str[2..].replace('\\', "/")
                ))
            } else {
                host_path.to_path_buf()
            }
        } else {
            host_path.to_path_buf()
        }
    }

    pub fn mount_smb_share(share_path: &str, mountpoint: &Path, username: Option<&str>, password: Option<&str>) -> Result<()> {
        if !cfg!(target_os = "windows") {
            return Ok(());
        }

        std::fs::create_dir_all(mountpoint)
            .map_err(|e| CrushError::StorageError(format!("Failed to create mountpoint: {}", e)))?;

        let mut cmd = std::process::Command::new("net");
        cmd.args(["use", &mountpoint.to_string_lossy(), share_path]);

        if let Some(user) = username {
            cmd.arg(format!("/USER:{}", user));
        }
        if let Some(pass) = password {
            cmd.arg(pass);
        }

        let out = cmd.output()
            .map_err(|e| CrushError::StorageError(format!("SMB mount failed: {}", e)))?;
        if !out.status.success() {
            return Err(CrushError::StorageError(format!(
                "SMB mount failed: {}", String::from_utf8_lossy(&out.stderr)
            )));
        }

        Ok(())
    }

    pub fn translate_acls(unix_path: &Path, windows_acl: &str) -> Result<()> {
        if !cfg!(target_os = "windows") {
            return Ok(());
        }

        let out = Command::new("icacls")
            .args([&unix_path.to_string_lossy(), "/grant", windows_acl])
            .output();

        if let Ok(ref out) = out {
            if !out.status.success() {
                eprintln!("Warning: ACL translation failed on {:?}: {}", unix_path, String::from_utf8_lossy(&out.stderr));
            }
        }

        Ok(())
    }
}
