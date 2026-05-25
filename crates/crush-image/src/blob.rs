use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, Write};
use sha2::{Sha256, Digest};
use crush_types::{Result, CrushError};

pub struct BlobStore {
    blobs_dir: PathBuf,
    locks_dir: PathBuf,
}

impl BlobStore {
    pub fn new(base_dir: &Path) -> Self {
        let blobs_dir = base_dir.join("blobs").join("sha256");
        let locks_dir = base_dir.join("locks");
        fs::create_dir_all(&blobs_dir).ok();
        fs::create_dir_all(&locks_dir).ok();
        Self { blobs_dir, locks_dir }
    }

    pub fn path_for_digest(&self, digest: &str) -> PathBuf {
        let stripped = digest.strip_prefix("sha256:").unwrap_or(digest);
        self.blobs_dir.join(stripped)
    }

    pub fn contains(&self, digest: &str) -> bool {
        self.path_for_digest(digest).exists()
    }

    pub fn atomic_write(&self, data: &[u8]) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hex_digest = hex::encode(hasher.finalize());
        let digest = format!("sha256:{}", hex_digest);

        let final_path = self.path_for_digest(&digest);
        if final_path.exists() {
            return Ok(digest);
        }

        let tmp_path = self.blobs_dir.join(format!(".tmp_{}", hex_digest));
        {
            let mut tmp = fs::File::create(&tmp_path)
                .map_err(|e| CrushError::StorageError(format!("Failed to create temp blob: {}", e)))?;
            tmp.write_all(data)
                .map_err(|e| CrushError::StorageError(format!("Failed to write blob: {}", e)))?;
            tmp.sync_all()
                .map_err(|e| CrushError::StorageError(format!("Failed to sync blob: {}", e)))?;
        }
        fs::rename(&tmp_path, &final_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to rename blob: {}", e)))?;

        Ok(digest)
    }

    pub fn read_blob(&self, digest: &str) -> Result<Vec<u8>> {
        let path = self.path_for_digest(digest);
        let _lock_guard = self.acquire_read_lock(digest)
            .ok_or_else(|| CrushError::StorageError("Failed to acquire blob lock".to_string()))?;
        fs::read(&path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read blob {}: {}", digest, e)))
    }

    pub fn read_blob_stream(&self, digest: &str) -> Result<fs::File> {
        let path = self.path_for_digest(digest);
        fs::File::open(&path)
            .map_err(|e| CrushError::StorageError(format!("Failed to open blob {}: {}", digest, e)))
    }

    pub fn iter_digests(&self) -> Result<Vec<String>> {
        let mut digests = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.blobs_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && name.len() == 64 {
                    digests.push(format!("sha256:{}", name));
                }
            }
        }
        Ok(digests)
    }

    pub fn delete_blob(&self, digest: &str) -> Result<()> {
        let path = self.path_for_digest(digest);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| CrushError::StorageError(format!("Failed to delete blob: {}", e)))?;
        }
        Ok(())
    }

    pub fn blob_size(&self, digest: &str) -> Result<u64> {
        let path = self.path_for_digest(digest);
        fs::metadata(&path)
            .map(|m| m.len())
            .map_err(|e| CrushError::StorageError(format!("Failed to stat blob: {}", e)))
    }

    fn acquire_read_lock(&self, digest: &str) -> Option<fs::File> {
        let lock_path = self.locks_dir.join(format!("{}.lock", digest));
        fs::OpenOptions::new()
            .create(true)
            .read(true)
            .open(&lock_path)
            .ok()
    }
}
