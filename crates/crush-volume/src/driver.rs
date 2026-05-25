use std::path::{Path, PathBuf};
use std::process::Command;
use async_trait::async_trait;
use crush_types::{Result, CrushError};

#[async_trait]
pub trait VolumeDriver: Send + Sync {
    fn name(&self) -> &str;
    async fn create_volume(&self, name: &str, mountpoint: &Path) -> Result<()>;
    async fn remove_volume(&self, name: &str, mountpoint: &Path) -> Result<()>;
    async fn mount_volume(&self, name: &str, mountpoint: &Path) -> Result<()>;
    async fn unmount_volume(&self, name: &str, mountpoint: &Path) -> Result<()>;
    async fn exists(&self, name: &str) -> Result<bool>;
}

pub struct LocalDriver;

#[async_trait]
impl VolumeDriver for LocalDriver {
    fn name(&self) -> &str { "local" }

    async fn create_volume(&self, name: &str, mountpoint: &Path) -> Result<()> {
        std::fs::create_dir_all(mountpoint)
            .map_err(|e| CrushError::StorageError(format!("Local volume create failed: {}", e)))
    }

    async fn remove_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        if mountpoint.exists() {
            std::fs::remove_dir_all(mountpoint)
                .map_err(|e| CrushError::StorageError(format!("Failed to remove volume: {}", e)))
        } else {
            Ok(())
        }
    }

    async fn mount_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        if !mountpoint.exists() {
            std::fs::create_dir_all(mountpoint)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
        }
        Ok(())
    }

    async fn unmount_volume(&self, _name: &str, _mountpoint: &Path) -> Result<()> {
        Ok(())
    }

    async fn exists(&self, name: &str) -> Result<bool> {
        Ok(std::path::Path::new(
            &std::env::temp_dir().join("crush").join("volumes").join(name)
        ).exists())
    }
}

pub struct NfsDriver {
    server: String,
    export_path: String,
    options: Vec<String>,
}

impl NfsDriver {
    pub fn new(server: &str, export_path: &str, options: Vec<String>) -> Self {
        Self {
            server: server.to_string(),
            export_path: export_path.to_string(),
            options,
        }
    }
}

#[async_trait]
impl VolumeDriver for NfsDriver {
    fn name(&self) -> &str { "nfs" }

    async fn create_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        std::fs::create_dir_all(mountpoint)
            .map_err(|e| CrushError::StorageError(e.to_string()))
    }

    async fn remove_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        let _ = Command::new("umount").arg(&mountpoint).output();
        if mountpoint.exists() {
            std::fs::remove_dir_all(mountpoint).ok();
        }
        Ok(())
    }

    async fn mount_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        let nfs_source = format!("{}:{}", self.server, self.export_path);
        let mut cmd = Command::new("mount");
        cmd.arg("-t").arg("nfs");
        for opt in &self.options {
            cmd.arg("-o").arg(opt);
        }
        cmd.arg(&nfs_source).arg(mountpoint);

        let out = cmd.output()
            .map_err(|e| CrushError::StorageError(format!("NFS mount failed: {}", e)))?;
        if !out.status.success() {
            return Err(CrushError::StorageError(format!(
                "NFS mount failed: {}", String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    async fn unmount_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        let out = Command::new("umount").arg(mountpoint).output()
            .map_err(|e| CrushError::StorageError(format!("NFS unmount failed: {}", e)))?;
        if !out.status.success() {
            return Err(CrushError::StorageError(format!(
                "NFS unmount failed: {}", String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    async fn exists(&self, _name: &str) -> Result<bool> { Ok(true) }
}

pub struct CifsDriver {
    server: String,
    share: String,
    username: Option<String>,
    password: Option<String>,
}

impl CifsDriver {
    pub fn new(server: &str, share: &str, username: Option<String>, password: Option<String>) -> Self {
        Self { server: server.to_string(), share: share.to_string(), username, password }
    }
}

#[async_trait]
impl VolumeDriver for CifsDriver {
    fn name(&self) -> &str { "cifs" }

    async fn create_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        std::fs::create_dir_all(mountpoint)
            .map_err(|e| CrushError::StorageError(e.to_string()))
    }

    async fn remove_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        let _ = Command::new("umount").arg(mountpoint).output();
        if mountpoint.exists() { std::fs::remove_dir_all(mountpoint).ok(); }
        Ok(())
    }

    async fn mount_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        let cifs_source = format!("//{}/{}", self.server, self.share);
        let mut cmd = Command::new("mount");
        cmd.arg("-t").arg("cifs").arg(&cifs_source).arg(mountpoint);
        if let Some(ref user) = self.username {
            cmd.arg("-o").arg(format!("user={}", user));
            if let Some(ref pass) = self.password {
                cmd.arg(format!("password={}", pass));
            }
        }
        let out = cmd.output()
            .map_err(|e| CrushError::StorageError(format!("CIFS mount failed: {}", e)))?;
        if !out.status.success() {
            return Err(CrushError::StorageError(format!(
                "CIFS mount failed: {}", String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }

    async fn unmount_volume(&self, _name: &str, mountpoint: &Path) -> Result<()> {
        let _ = Command::new("umount").arg(mountpoint).output();
        Ok(())
    }

    async fn exists(&self, _name: &str) -> Result<bool> { Ok(true) }
}

pub struct VolumeDriverRegistry {
    drivers: Vec<Box<dyn VolumeDriver>>,
}

impl VolumeDriverRegistry {
    pub fn new() -> Self {
        Self {
            drivers: vec![Box::new(LocalDriver)],
        }
    }

    pub fn register(&mut self, driver: Box<dyn VolumeDriver>) {
        self.drivers.push(driver);
    }

    pub fn get(&self, name: &str) -> Option<&dyn VolumeDriver> {
        self.drivers.iter().find(|d| d.name() == name).map(|d| d.as_ref())
    }
}
