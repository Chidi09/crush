use std::path::Path;
use crush_types::{Result, CrushError};

pub struct TmpfsConfig {
    pub mountpoint: String,
    pub size_bytes: Option<u64>,
    pub size_percent: Option<u8>,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub noexec: bool,
    pub nosuid: bool,
}

impl TmpfsConfig {
    pub fn new(mountpoint: &str) -> Self {
        Self {
            mountpoint: mountpoint.to_string(),
            size_bytes: None,
            size_percent: None,
            mode: 0o755,
            uid: 0,
            gid: 0,
            noexec: false,
            nosuid: false,
        }
    }

    pub fn with_size(mut self, bytes: u64) -> Self {
        self.size_bytes = Some(bytes);
        self
    }

    pub fn with_percent(mut self, percent: u8) -> Self {
        self.size_percent = Some(percent.min(100));
        self
    }

    pub fn with_mode(mut self, mode: u32) -> Self {
        self.mode = mode;
        self
    }

    pub fn noexec(mut self) -> Self {
        self.noexec = true;
        self
    }

    pub fn nosuid(mut self) -> Self {
        self.nosuid = true;
        self
    }

    #[cfg(target_os = "linux")]
    pub fn mount(&self, container_root: &Path) -> Result<()> {
        use nix::mount::{mount, MsFlags};

        let target = if self.mountpoint.starts_with('/') {
            container_root.join(
                self.mountpoint.strip_prefix("/").unwrap_or(&self.mountpoint)
            )
        } else {
            container_root.join(&self.mountpoint)
        };

        std::fs::create_dir_all(&target)
            .map_err(|e| CrushError::StorageError(format!("Failed to create tmpfs target: {}", e)))?;

        let mut options = vec![
            format!("mode={:o}", self.mode),
            format!("uid={}", self.uid),
            format!("gid={}", self.gid),
        ];

        if let Some(bytes) = self.size_bytes {
            options.push(format!("size={}", bytes));
        } else if let Some(pct) = self.size_percent {
            options.push(format!("size={}%", pct));
        }

        if self.noexec {
            options.push("noexec".to_string());
        }
        if self.nosuid {
            options.push("nosuid".to_string());
        }

        let options_str = options.join(",");

        let mut flags = MsFlags::MS_NODEV;
        if self.noexec { flags |= MsFlags::MS_NOEXEC; }
        if self.nosuid { flags |= MsFlags::MS_NOSUID; }

        mount(
            Some("tmpfs"),
            &target,
            Some("tmpfs"),
            flags,
            Some(options_str.as_str()),
        ).map_err(|e| CrushError::StorageError(format!("tmpfs mount failed: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn mount(&self, _container_root: &Path) -> Result<()> {
        Ok(())
    }
}

pub fn setup_default_tmpfs_mounts(container_root: &Path) -> Result<()> {
    let mounts = vec![
        TmpfsConfig::new("/tmp").with_mode(0o1777).noexec().nosuid(),
        TmpfsConfig::new("/run").with_mode(0o755).nosuid(),
        TmpfsConfig::new("/var/run").with_mode(0o755),
    ];

    for m in mounts {
        m.mount(container_root)?;
    }

    Ok(())
}
