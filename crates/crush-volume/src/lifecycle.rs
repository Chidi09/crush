use std::path::PathBuf;
use crush_types::{Result, CrushError, MountConfig};
use crate::named::NamedVolumeManager;

pub struct VolumeLifecycleManager {
    named: NamedVolumeManager,
    anonymous_volumes: Vec<String>,
}

impl VolumeLifecycleManager {
    pub fn new(named: NamedVolumeManager) -> Self {
        Self {
            named,
            anonymous_volumes: Vec::new(),
        }
    }

    pub async fn mount_volume(
        &self,
        cfg: &MountConfig,
        container_root: &PathBuf,
        remove_on_exit: bool,
    ) -> Result<()> {
        if cfg.is_tmpfs {
            let tmpfs = crate::tmpfs::TmpfsConfig::new(
                &cfg.container_path.to_string_lossy()
            );
            return tmpfs.mount(container_root);
        }

        if cfg.host_path.starts_with(&self.named.volume_path("")) {
            let volume_name = cfg.host_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            self.named.increment_ref(&volume_name).await?;
        }

        crate::bind::BindMountManager::validate_mount(cfg)?;
        crate::bind::BindMountManager::perform_bind_mount(cfg, container_root)
    }

    pub async fn unmount_volume(
        &self,
        cfg: &MountConfig,
        container_root: &PathBuf,
        remove_on_exit: bool,
    ) -> Result<()> {
        let container_path = if cfg.container_path.is_absolute() {
            container_root.join(
                cfg.container_path.strip_prefix("/")
                    .unwrap_or(&cfg.container_path)
            )
        } else {
            container_root.join(&cfg.container_path)
        };

        #[cfg(target_os = "linux")]
        {
            let _ = nix::mount::umount(&container_path);
        }

        if cfg.host_path.starts_with(&self.named.volume_path("")) {
            let volume_name = cfg.host_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            self.named.decrement_ref(&volume_name).await?;
        }

        Ok(())
    }

    pub fn register_anonymous(&mut self, volume_id: String) {
        self.anonymous_volumes.push(volume_id);
    }

    pub async fn cleanup_anonymous(&mut self) -> Result<()> {
        for vol_id in self.anonymous_volumes.drain(..) {
            let _ = self.named.remove(&vol_id).await;
        }
        Ok(())
    }

    pub fn named_manager(&self) -> &NamedVolumeManager {
        &self.named
    }
}
