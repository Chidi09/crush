use std::path::{Path, PathBuf};

pub struct SnapshotStore {
    store_dir: PathBuf,
}

impl SnapshotStore {
    pub fn new(data_dir: &Path) -> Self {
        let store_dir = data_dir.join("fc-snapshots");
        std::fs::create_dir_all(&store_dir).ok();
        Self { store_dir }
    }

    /// Path to the memory snapshot file for this image digest.
    pub fn mem_path(&self, image_digest: &str) -> PathBuf {
        let safe = image_digest.replace(':', "_");
        self.store_dir.join(format!("{}.mem", &safe[..std::cmp::min(safe.len(), 64)]))
    }

    /// Path to the VM state snapshot file for this image digest.
    pub fn state_path(&self, image_digest: &str) -> PathBuf {
        let safe = image_digest.replace(':', "_");
        self.store_dir.join(format!("{}.state", &safe[..std::cmp::min(safe.len(), 64)]))
    }

    /// Returns true if both snapshot files exist and are non-empty.
    pub fn exists(&self, image_digest: &str) -> bool {
        let mem = self.mem_path(image_digest);
        let state = self.state_path(image_digest);
        mem.exists() && mem.metadata().map(|m| m.len() > 0).unwrap_or(false)
            && state.exists() && state.metadata().map(|m| m.len() > 0).unwrap_or(false)
    }

    /// Invalidate (delete) the snapshot — called when the image is re-pulled or
    /// the drive image is rebuilt.
    pub fn invalidate(&self, image_digest: &str) {
        let _ = std::fs::remove_file(self.mem_path(image_digest));
        let _ = std::fs::remove_file(self.state_path(image_digest));
    }
}
