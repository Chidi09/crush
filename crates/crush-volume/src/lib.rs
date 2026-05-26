pub mod named;
pub mod bind;
pub mod tmpfs;
pub mod driver;
pub mod quota;
pub mod backup;
pub mod windows;
pub mod lifecycle;

use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError, MountConfig};
use named::NamedVolumeManager;
use driver::{VolumeDriver, VolumeDriverRegistry, LocalDriver, NfsDriver, CifsDriver};
use lifecycle::VolumeLifecycleManager;

pub struct VolumeManager {
    pub named: NamedVolumeManager,
    pub drivers: VolumeDriverRegistry,
    pub lifecycle: VolumeLifecycleManager,
    base_dir: PathBuf,
}

impl VolumeManager {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let named = NamedVolumeManager::new(base_dir.clone())?;
        let lifecycle = VolumeLifecycleManager::new(
            NamedVolumeManager::new(base_dir.clone())?
        );

        Ok(Self {
            named,
            drivers: VolumeDriverRegistry::new(),
            lifecycle,
            base_dir,
        })
    }

    pub fn register_nfs_driver(&mut self, server: &str, export: &str, options: Vec<String>) {
        self.drivers.register(Box::new(NfsDriver::new(server, export, options)));
    }

    pub fn register_cifs_driver(&mut self, server: &str, share: &str, username: Option<String>, password: Option<String>) {
        self.drivers.register(Box::new(CifsDriver::new(server, share, username, password)));
    }

    pub async fn create_volume(&self, name: &str, driver: &str, labels: Vec<(String, String)>) -> Result<()> {
        if let Some(drv) = self.drivers.get(driver) {
            let vol_path = self.named.volume_path(name);
            drv.create_volume(name, &vol_path).await?;
        }
        self.named.create(name, driver, labels).await?;
        Ok(())
    }

    pub async fn remove_volume(&self, name: &str) -> Result<()> {
        if let Ok(meta) = self.named.get(name).await {
            if let Some(drv) = self.drivers.get(&meta.driver) {
                drv.remove_volume(name, &meta.mountpoint).await?;
            }
        }
        self.named.remove(name).await
    }

    pub async fn mount_volume(&self, name: &str) -> Result<()> {
        let meta = self.named.get(name).await?;
        if let Some(drv) = self.drivers.get(&meta.driver) {
            drv.mount_volume(name, &meta.mountpoint).await?;
        }
        self.named.increment_ref(name).await
    }

    pub async fn unmount_volume(&self, name: &str) -> Result<()> {
        let meta = self.named.get(name).await?;
        if let Some(drv) = self.drivers.get(&meta.driver) {
            drv.unmount_volume(name, &meta.mountpoint).await?;
        }
        self.named.decrement_ref(name).await
    }

    pub async fn backup_volume(&self, name: &str, dest: &Path) -> Result<()> {
        let meta = self.named.get(name).await?;
        backup::VolumeBackup::snapshot_consistent_backup(&meta.mountpoint, dest)
    }

    pub async fn restore_volume(&self, name: &str, src: &Path) -> Result<()> {
        let meta = self.named.get(name).await?;
        backup::VolumeBackup::restore_from_file(&meta.mountpoint, src)
    }

    pub async fn check_volume_quota(&self, name: &str, limit_bytes: u64) -> Result<()> {
        let meta = self.named.get(name).await?;
        quota::QuotaManager::check_and_warn(&meta.mountpoint, limit_bytes, name)
    }

    pub async fn setup_container_mounts(
        &self,
        mounts: &[MountConfig],
        container_root: &PathBuf,
        remove_on_exit: bool,
    ) -> Result<()> {
        for cfg in mounts {
            self.lifecycle.mount_volume(cfg, container_root, remove_on_exit).await?;
        }
        Ok(())
    }

    pub async fn teardown_container_mounts(
        &mut self,
        mounts: &[MountConfig],
        container_root: &PathBuf,
        remove_on_exit: bool,
    ) -> Result<()> {
        for cfg in mounts {
            self.lifecycle.unmount_volume(cfg, container_root, remove_on_exit).await?;
        }
        if remove_on_exit {
            self.lifecycle.cleanup_anonymous().await?;
        }
        Ok(())
    }

    pub fn named_manager(&self) -> &NamedVolumeManager {
        &self.named
    }
}
