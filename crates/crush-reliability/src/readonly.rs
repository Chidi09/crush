use std::path::Path;
use crush_types::{Result, CrushError};

pub struct ReadOnlyRootfs;

impl ReadOnlyRootfs {
    pub fn setup_overlays(rootfs: &Path) -> Result<Vec<String>> {
        let overlay_dirs = ["/tmp", "/run", "/var/run", "/tmp/.X11-unix"];

        for dir in &overlay_dirs {
            let target = if dir.starts_with('/') {
                rootfs.join(dir.strip_prefix("/").unwrap_or(dir))
            } else {
                rootfs.join(dir)
            };
            std::fs::create_dir_all(&target)
                .map_err(|e| CrushError::StorageError(format!("Failed to create {}: {}", target.display(), e)))?;
        }

        Ok(overlay_dirs.iter().map(|s| s.to_string()).collect())
    }

    pub fn detect_writes(rootfs: &Path) -> Result<Vec<String>> {
        let upper = rootfs.join(".crush_upper");
        if !upper.exists() {
            return Ok(Vec::new());
        }

        let mut written = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&upper) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') {
                    written.push(name);
                }
            }
        }
        Ok(written)
    }
}
