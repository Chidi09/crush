use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    pub fn new(base_dir: PathBuf) -> Self {
        fs::create_dir_all(&base_dir).ok();
        Self { base_dir }
    }

    pub fn prepare_vm_directory(&self, vm_id: &str) -> Result<PathBuf> {
        let vm_dir = self.base_dir.join(vm_id);
        fs::create_dir_all(&vm_dir)
            .map_err(|e| CrushError::StorageError(format!("Failed to create VM dir: {}", e)))?;
        fs::create_dir_all(vm_dir.join("overlay"))
            .map_err(|e| CrushError::StorageError(format!("Failed to create overlay dir: {}", e)))?;
        Ok(vm_dir)
    }

    pub fn ensure_kernel(&self) -> Result<PathBuf> {
        let kernel_dir = self.base_dir.join("_assets").join("kernel");
        fs::create_dir_all(&kernel_dir).ok();

        let kernel_path = kernel_dir.join("vmlinuz");

        if !kernel_path.exists() {
            let assets_dir = self.base_dir.join("_assets");
            let is_cached = self.download_if_missing(
                "https://crush.run/assets/kernel/vmlinuz-6.1-crush",
                &kernel_path,
            )?;
            if !is_cached && kernel_path.exists() {
                // already exists from previous run
            }
        }

        if !kernel_path.exists() {
            self.build_minimal_kernel(&kernel_path)?;
        }

        Ok(kernel_path)
    }

    pub fn ensure_initrd(&self) -> Result<PathBuf> {
        let initrd_dir = self.base_dir.join("_assets").join("initrd");
        fs::create_dir_all(&initrd_dir).ok();

        let initrd_path = initrd_dir.join("initramfs.cpio.gz");

        if !initrd_path.exists() {
            self.build_minimal_initrd(&initrd_path)?;
        }

        Ok(initrd_path)
    }

    pub fn create_overlay_disk(&self, vm_id: &str) -> Result<PathBuf> {
        let overlay_path = self.base_dir.join(vm_id).join("overlay.qcow2");

        if !overlay_path.exists() {
            let status = Command::new("qemu-img")
                .args([
                    "create",
                    "-f", "qcow2",
                    &overlay_path.to_string_lossy(),
                    "10G",
                ])
                .status();

            match status {
                Ok(s) if s.success() => {}
                _ => {
                    self.create_sparse_file(&overlay_path, 10 * 1024 * 1024 * 1024)?;
                }
            }
        }

        Ok(overlay_path)
    }

    fn create_sparse_file(&self, path: &Path, size: u64) -> Result<()> {
        let file = fs::File::create(path)
            .map_err(|e| CrushError::StorageError(format!("Failed to create sparse file: {}", e)))?;
        file.set_len(size)
            .map_err(|e| CrushError::StorageError(format!("Failed to set sparse file len: {}", e)))?;
        Ok(())
    }

    fn download_if_missing(&self, url: &str, dest: &Path) -> Result<bool> {
        if dest.exists() {
            return Ok(true);
        }

        let status = Command::new("curl")
            .args(["-L", "-o", &dest.to_string_lossy(), url])
            .status();

        match status {
            Ok(s) if s.success() => Ok(true),
            _ => Ok(false),
        }
    }

    fn build_minimal_kernel(&self, dest: &Path) -> Result<()> {
        let kernel_src = PathBuf::from(if cfg!(target_os = "macos") {
            "/System/Library/Kernels/kernel"
        } else {
            "/boot/vmlinuz"
        });

        if kernel_src.exists() {
            fs::copy(&kernel_src, dest)
                .map_err(|e| CrushError::StorageError(format!("Failed to copy kernel: {}", e)))?;
            Ok(())
        } else {
            Err(CrushError::StorageError(
                "No kernel found. Install linux kernel or place vmlinuz in VM assets.".to_string(),
            ))
        }
    }

    fn build_minimal_initrd(&self, dest: &Path) -> Result<()> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"crush-initrd");
        let marker = hex::encode(hasher.finalize());

        let mut builder = tar::Builder::new(Vec::new());

        let init_content = format!(
            "#!/bin/sh\n\
             echo 'Crush VM initialized'\n\
             echo '{}' > /crush_vm_id\n\
             mount -t devtmpfs none /dev\n\
             mount -t proc none /proc\n\
             mount -t sysfs none /sys\n\
             mkdir -p /dev/pts\n\
             mount -t devpts none /dev/pts\n\
             exec /bin/sh\n",
            marker
        );

        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o755);
        header.set_size(init_content.len() as u64);
        builder.append_data(&mut header, "init", init_content.as_bytes())
            .map_err(|e| CrushError::StorageError(format!("Tar error: {}", e)))?;

        let tar_data = builder.into_inner()
            .map_err(|e| CrushError::StorageError(format!("Tar finalize error: {}", e)))?;

        let compressed = {
            let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            std::io::Write::write_all(&mut encoder, &tar_data)
                .map_err(|e| CrushError::StorageError(format!("Gzip error: {}", e)))?;
            encoder.finish()
                .map_err(|e| CrushError::StorageError(format!("Gzip finish: {}", e)))?
        };

        fs::write(dest, &compressed)
            .map_err(|e| CrushError::StorageError(format!("Failed to write initrd: {}", e)))?;

        Ok(())
    }

    pub fn cleanup_vm(&self, vm_id: &str) -> Result<()> {
        let vm_dir = self.base_dir.join(vm_id);
        if vm_dir.exists() {
            fs::remove_dir_all(&vm_dir)
                .map_err(|e| CrushError::StorageError(format!("Failed to remove VM dir: {}", e)))?;
        }
        Ok(())
    }
}
