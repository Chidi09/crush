use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};
use crate::{MountRecord, MountType, LocalDriver, VolumeDriver};

#[derive(Debug, Clone)]
pub struct VolumeMounter {
    pub data_dir: PathBuf,
}

impl VolumeMounter {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    async fn add_mount_record(&self, container_id: &str, record: MountRecord) -> Result<()> {
        let mounts_json_path = self.data_dir.join("containers").join(container_id).join("mounts.json");
        if let Some(parent) = mounts_json_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut records: Vec<MountRecord> = if mounts_json_path.exists() {
            let content = tokio::fs::read_to_string(&mounts_json_path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        records.push(record);
        let serialized = serde_json::to_string_pretty(&records)?;
        tokio::fs::write(&mounts_json_path, serialized).await?;
        Ok(())
    }

    async fn update_ref_count(&self, name: &str, delta: i32) -> Result<()> {
        let local_driver = LocalDriver::new(self.data_dir.clone());
        let mut info = local_driver.inspect(name).await?;
        if delta > 0 {
            info.ref_count += delta as u32;
        } else {
            info.ref_count = info.ref_count.saturating_sub((-delta) as u32);
        }
        let json_path = local_driver.volume_json_path(name);
        let serialized = serde_json::to_string_pretty(&info)?;
        tokio::fs::write(&json_path, serialized).await?;
        Ok(())
    }

    // --- named volume mounting ---

    pub async fn mount_named(&self, container_id: &str, name: &str, container_path: &str, rootfs: &Path, readonly: bool) -> Result<()> {
        let local_driver = LocalDriver::new(self.data_dir.clone());
        let source_path = local_driver.path(name).await?;
        if !source_path.exists() {
            return Err(anyhow!("named volume source path does not exist: {:?}", source_path));
        }

        let container_path_clean = container_path.trim_start_matches('/').trim_start_matches('\\');
        let dest_path = rootfs.join(container_path_clean);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        #[cfg(target_os = "linux")]
        {
            tokio::fs::create_dir_all(&dest_path).await?;
            use nix::mount::{mount, MsFlags};
            mount(
                Some(&source_path),
                &dest_path,
                None::<&str>,
                MsFlags::MS_BIND,
                None::<&str>
            )?;
            if readonly {
                mount(
                    Some(&source_path),
                    &dest_path,
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
                    None::<&str>
                )?;
            }
        }

        #[cfg(not(target_os = "linux"))]
        return Err(anyhow!("named volume mounting requires Linux"));

        #[cfg(target_os = "linux")]
        self.update_ref_count(name, 1).await?;

        let record = MountRecord {
            source: name.to_string(),
            destination: dest_path,
            mount_type: MountType::Named,
            readonly,
        };
        self.add_mount_record(container_id, record).await?;

        Ok(())
    }

    // --- bind mounting ---

    pub async fn mount_bind(&self, container_id: &str, host_path: &str, container_path: &str, rootfs: &Path, readonly: bool) -> Result<()> {
        let source_path = PathBuf::from(host_path);
        if !source_path.exists() {
            return Err(anyhow!("host path does not exist: {}", host_path));
        }

        let container_path_clean = container_path.trim_start_matches('/').trim_start_matches('\\');
        let dest_path = rootfs.join(container_path_clean);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        #[cfg(target_os = "linux")]
        {
            tokio::fs::create_dir_all(&dest_path).await?;
            use nix::mount::{mount, MsFlags};
            mount(
                Some(&source_path),
                &dest_path,
                None::<&str>,
                MsFlags::MS_BIND,
                None::<&str>
            )?;
            if readonly {
                mount(
                    Some(&source_path),
                    &dest_path,
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
                    None::<&str>
                )?;
            }
        }

        #[cfg(not(target_os = "linux"))]
        return Err(anyhow!("bind mounting requires Linux"));

        let record = MountRecord {
            source: host_path.to_string(),
            destination: dest_path,
            mount_type: MountType::Bind,
            readonly,
        };
        self.add_mount_record(container_id, record).await?;

        Ok(())
    }

    // --- tmpfs mounting ---

    pub async fn mount_tmpfs(&self, container_id: &str, container_path: &str, rootfs: &Path, size_bytes: u64, mode: &str) -> Result<()> {
        let container_path_clean = container_path.trim_start_matches('/').trim_start_matches('\\');
        let dest_path = rootfs.join(container_path_clean);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        #[cfg(target_os = "linux")]
        {
            tokio::fs::create_dir_all(&dest_path).await?;
            use nix::mount::{mount, MsFlags};
            let mut opts = format!("size={}", size_bytes);
            if !mode.is_empty() {
                opts.push_str(&format!(",mode={}", mode));
            }
            mount(
                Some("tmpfs"),
                &dest_path,
                Some("tmpfs"),
                MsFlags::empty(),
                Some(opts.as_str())
            )?;
        }

        #[cfg(not(target_os = "linux"))]
        return Err(anyhow!("tmpfs mounting requires Linux"));

        let record = MountRecord {
            source: "tmpfs".to_string(),
            destination: dest_path,
            mount_type: MountType::Tmpfs,
            readonly: false,
        };
        self.add_mount_record(container_id, record).await?;

        Ok(())
    }

    pub async fn unmount_all(&self, container_id: &str) -> Result<()> {
        let mounts_json_path = self.data_dir.join("containers").join(container_id).join("mounts.json");
        if !mounts_json_path.exists() {
            return Ok(());
        }
        let content = tokio::fs::read_to_string(&mounts_json_path).await?;
        let records: Vec<MountRecord> = serde_json::from_str(&content)?;

        for record in records.iter().rev() {
            #[cfg(target_os = "linux")]
            {
                use nix::mount::{umount2, MntFlags};
                let _ = umount2(&record.destination, MntFlags::MNT_FORCE);
            }


            if record.mount_type == MountType::Named {
                let _ = self.update_ref_count(&record.source, -1).await;
            }
        }

        let _ = tokio::fs::remove_file(&mounts_json_path).await;
        Ok(())
    }
}
