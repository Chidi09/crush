use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct VmImageManager {
    assets_dir: PathBuf,
}

impl VmImageManager {
    pub fn new(base_dir: &Path) -> Self {
        let assets_dir = base_dir.join("_assets");
        fs::create_dir_all(&assets_dir).ok();
        Self { assets_dir }
    }

    pub fn ensure_base_image(&self) -> Result<PathBuf> {
        let base_image_path = self.assets_dir.join("crush-base.img");

        if !base_image_path.exists() {
            self.create_base_image(&base_image_path)?;
        }

        Ok(base_image_path)
    }

    fn create_base_image(&self, dest: &Path) -> Result<()> {
        let size_mb = 256;

        let file = fs::File::create(dest)
            .map_err(|e| CrushError::StorageError(format!("Failed to create base image: {}", e)))?;
        file.set_len(size_mb * 1024 * 1024)
            .map_err(|e| CrushError::StorageError(format!("Failed to set base image size: {}", e)))?;

        let status = Command::new("mkfs.ext4")
            .arg(&dest.to_string_lossy())
            .status();

        match status {
            Ok(s) if s.success() => {}
            _ => {
                // Use dd to create a minimal MBR/partition table
                let dd_status = Command::new("dd")
                    .args([
                        "if=/dev/zero",
                        &format!("of={}", dest.to_string_lossy()),
                        "bs=1M",
                        &format!("count={}", size_mb),
                    ])
                    .status();

                if let Ok(s) = dd_status {
                    if !s.success() {
                        eprintln!("Warning: dd base image creation returned non-zero");
                    }
                }
            }
        }

        Ok(())
    }

    pub fn create_overlay(&self, vm_id: &str, overlay_dir: &Path) -> Result<PathBuf> {
        let overlay_path = overlay_dir.join("overlay.img");
        let overlay_size_mb = 2048;

        let file = fs::File::create(&overlay_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to create overlay: {}", e)))?;
        file.set_len(overlay_size_mb * 1024 * 1024)
            .map_err(|e| CrushError::StorageError(format!("Failed to set overlay size: {}", e)))?;

        Ok(overlay_path)
    }

    pub fn garbage_collect(&self, active_vm_ids: &[String]) -> Result<u64> {
        let mut freed_bytes = 0u64;

        if let Ok(entries) = fs::read_dir(&self.assets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        let name = dir_name.to_string_lossy();
                        if !active_vm_ids.contains(&name.to_string()) && name.starts_with("vm_") {
                            if let Ok(meta) = fs::metadata(&path) {
                                freed_bytes += meta.len();
                            }
                            let _ = fs::remove_dir_all(&path);
                        }
                    }
                }
            }
        }

        Ok(freed_bytes)
    }
}
