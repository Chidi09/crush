use std::path::Path;
use std::collections::HashSet;
use std::fs;
use crush_types::{Result, CrushError};

pub struct GarbageCollector {
    blobs_dir: std::path::PathBuf,
}

#[derive(Debug)]
pub struct GcReport {
    pub blobs_removed: u64,
    pub bytes_reclaimed: u64,
}

impl GarbageCollector {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            blobs_dir: base_dir.join("blobs").join("sha256"),
        }
    }

    pub fn run_mark_and_sweep(&self, protected_digests: &HashSet<String>) -> Result<GcReport> {
        let mut removed = 0u64;
        let mut reclaimed = 0u64;

        if !self.blobs_dir.exists() {
            return Ok(GcReport { blobs_removed: 0, bytes_reclaimed: 0 });
        }

        for entry in fs::read_dir(&self.blobs_dir)
            .map_err(|e| CrushError::StorageError(format!("Failed to read blobs dir: {}", e)))? {
            let entry = entry
                .map_err(|e| CrushError::StorageError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with('.') {
                continue;
            }

            let digest = format!("sha256:{}", filename);

            if protected_digests.contains(&digest) {
                continue;
            }

            if let Ok(meta) = fs::metadata(&path) {
                reclaimed += meta.len();
            }

            fs::remove_file(&path)
                .map_err(|e| CrushError::StorageError(format!("Failed to remove blob: {}", e)))?;
            removed += 1;
        }

        Ok(GcReport {
            blobs_removed: removed,
            bytes_reclaimed: reclaimed,
        })
    }
}
