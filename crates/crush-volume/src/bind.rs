use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError, MountConfig};

pub struct BindMountManager;

impl BindMountManager {
    pub fn validate_mount(cfg: &MountConfig) -> Result<()> {
        if !cfg.host_path.exists() {
            eprintln!("Warning: host path does not exist: {:?}", cfg.host_path);
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn perform_bind_mount(cfg: &MountConfig, container_root: &Path) -> Result<()> {
        use nix::mount::{mount, MsFlags};

        let target = if cfg.container_path.is_absolute() {
            container_root.join(
                cfg.container_path.strip_prefix("/")
                    .unwrap_or(&cfg.container_path)
            )
        } else {
            container_root.join(&cfg.container_path)
        };

        std::fs::create_dir_all(&target)
            .map_err(|e| CrushError::StorageError(format!("Failed to create mount target: {}", e)))?;

        let mut flags = MsFlags::MS_BIND | MsFlags::MS_REC;
        if cfg.read_only {
            flags |= MsFlags::MS_RDONLY;
        }

        mount(
            Some(&cfg.host_path),
            &target,
            None::<&str>,
            flags,
            None::<&str>,
        ).map_err(|e| CrushError::StorageError(format!("Bind mount failed: {}", e)))?;

        if cfg.read_only {
            mount(
                Some(&target),
                &target,
                None::<&str>,
                MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
                None::<&str>,
            ).map_err(|e| CrushError::StorageError(format!("Remount ro failed: {}", e)))?;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn perform_bind_mount(cfg: &MountConfig, container_root: &Path) -> Result<()> {
        let _ = (cfg, container_root);
        Ok(())
    }

    pub fn setup_rootless_remapping(host_uid: u32, host_gid: u32, mount_path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if let Ok(meta) = std::fs::metadata(mount_path) {
                if meta.uid() != host_uid || meta.gid() != host_gid {
                    unsafe {
                        libc::chown(
                            mount_path.to_string_lossy().as_ptr() as *const i8,
                            host_uid,
                            host_gid,
                        );
                    }
                }
            }
        }
        let _ = (host_uid, host_gid, mount_path);
        Ok(())
    }
}
