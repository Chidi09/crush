use std::path::{Path, PathBuf};
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct VolumeBackup;

impl VolumeBackup {
    pub fn backup_to_stream(volume_path: &Path, output: impl std::io::Write) -> Result<()> {
        let mut builder = tar::Builder::new(output);

        let entries = std::fs::read_dir(volume_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read volume: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.strip_prefix(volume_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            if path.is_dir() {
                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Directory);
                header.set_mode(0o755);
                builder.append_data(&mut header, &name, std::io::empty())
                    .map_err(|e| CrushError::StorageError(format!("Tar error: {}", e)))?;
            } else if path.is_file() {
                let data = std::fs::read(&path)
                    .map_err(|e| CrushError::StorageError(format!("Read error: {}", e)))?;
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                builder.append_data(&mut header, &name, &data[..])
                    .map_err(|e| CrushError::StorageError(format!("Tar error: {}", e)))?;
            }
        }

        builder.finish()
            .map_err(|e| CrushError::StorageError(format!("Tar finish error: {}", e)))
    }

    pub fn backup_to_file(volume_path: &Path, output_path: &Path) -> Result<()> {
        let file = std::fs::File::create(output_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to create backup: {}", e)))?;
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        Self::backup_to_stream(volume_path, &mut encoder)?;
        encoder.finish()
            .map_err(|e| CrushError::StorageError(format!("Gzip finish error: {}", e)))?;
        Ok(())
    }

    pub fn restore_from_stream(volume_path: &Path, input: impl std::io::Read) -> Result<()> {
        if volume_path.exists() {
            std::fs::remove_dir_all(volume_path)
                .map_err(|e| CrushError::StorageError(format!("Failed to clean volume: {}", e)))?;
        }
        std::fs::create_dir_all(volume_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to create volume: {}", e)))?;

        let mut archive = tar::Archive::new(input);
        archive.unpack(volume_path)
            .map_err(|e| CrushError::StorageError(format!("Tar unpack error: {}", e)))
    }

    pub fn restore_from_file(volume_path: &Path, input_path: &Path) -> Result<()> {
        let file = std::fs::File::open(input_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to open backup: {}", e)))?;
        let decoder = flate2::read::GzDecoder::new(file);
        Self::restore_from_stream(volume_path, decoder)
    }

    pub fn fsfreeze_sync(volume_path: &Path) -> Result<()> {
        let out = Command::new("fsfreeze")
            .args(["-f", &volume_path.to_string_lossy()])
            .output();
        if let Ok(ref out) = out {
            if out.status.success() {
                let _ = Command::new("fsfreeze")
                    .args(["-u", &volume_path.to_string_lossy()])
                    .output();
            }
        }
        Ok(())
    }

    pub fn snapshot_consistent_backup(volume_path: &Path, output_path: &Path) -> Result<()> {
        Self::fsfreeze_sync(volume_path)?;
        let result = Self::backup_to_file(volume_path, output_path);
        Self::fsfreeze_sync(volume_path)?;
        result
    }
}
