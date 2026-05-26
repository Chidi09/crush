use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use sha2::{Sha256, Digest};
use crush_types::{Result, CrushError};

pub struct BuildCache {
    cache_dir: PathBuf,
    registry_endpoint: Option<String>,
    local_manifest: Arc<Mutex<HashMap<String, CacheEntry>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub layer_digest: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub stage_name: String,
    pub compressed: bool,
}

impl BuildCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).ok();
        Self {
            cache_dir,
            registry_endpoint: None,
            local_manifest: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_remote_endpoint(&mut self, endpoint: String) {
        self.registry_endpoint = Some(endpoint);
    }

    pub fn cache_key(data: &[u8], label: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(label.as_bytes());
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }

    pub fn lockfile_key(root: &Path) -> Result<String> {
        let lockfiles = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml",
            "Cargo.lock", "go.sum", "Gemfile.lock", "poetry.lock",
            "composer.lock", "mix.lock"];

        let mut hasher = Sha256::new();
        let mut found = false;

        for lf in &lockfiles {
            let path = root.join(lf);
            if path.exists() {
                if let Ok(data) = std::fs::read(&path) {
                    hasher.update(data);
                    found = true;
                }
            }
        }

        if !found { return Ok(String::new()); }
        Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
    }

    pub fn source_tree_hash(root: &Path, ignore_file: &Path) -> Result<String> {
        let mut hasher = Sha256::new();
        let mut paths: Vec<PathBuf> = Vec::new();

        let walker = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target"
                    && name != "node_modules"
            });

        for entry in walker {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    paths.push(entry.path().to_path_buf());
                }
            }
        }

        paths.sort();
        for path in &paths {
            if let Ok(data) = std::fs::read(path) {
                hasher.update(data);
            }
        }

        Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
    }

    pub async fn get(&self, key: &str) -> Result<Option<CacheEntry>> {
        let manifest = self.local_manifest.lock().await;
        if let Some(entry) = manifest.get(key) {
            let layer_path = self.layer_path(key);
            if layer_path.exists() {
                return Ok(Some(entry.clone()));
            }
        }

        let entry_path = self.cache_dir.join(format!("{}.json", key.replace(':', "_")));
        if entry_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&entry_path) {
                if let Ok(entry) = serde_json::from_str::<CacheEntry>(&data) {
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }

    pub async fn put(&self, key: &str, data: &[u8], stage_name: &str, compressed: bool) -> Result<CacheEntry> {
        let layer_path = self.layer_path(key);
        std::fs::create_dir_all(layer_path.parent().unwrap())
            .map_err(|e| CrushError::StorageError(format!("Cache dir error: {}", e)))?;

        std::fs::write(&layer_path, data)
            .map_err(|e| CrushError::StorageError(format!("Cache write error: {}", e)))?;

        let entry = CacheEntry {
            key: key.to_string(),
            layer_digest: key.to_string(),
            size_bytes: data.len() as u64,
            created_at: chrono::Utc::now().to_rfc3339(),
            stage_name: stage_name.to_string(),
            compressed,
        };

        let mut manifest = self.local_manifest.lock().await;
        manifest.insert(key.to_string(), entry.clone());

        let entry_path = self.cache_dir.join(format!("{}.json", key.replace(':', "_")));
        let meta = serde_json::to_string(&entry)
            .map_err(|e| CrushError::ImageError(e.to_string()))?;
        std::fs::write(&entry_path, meta)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        Ok(entry)
    }

    pub fn export_tarball(&self, keys: &[String], output: &Path) -> Result<()> {
        let mut builder = tar::Builder::new(std::fs::File::create(output)
            .map_err(|e| CrushError::StorageError(e.to_string()))?);
        for key in keys {
            let path = self.layer_path(key);
            if path.exists() {
                let data = std::fs::read(&path)
                    .map_err(|e| CrushError::StorageError(e.to_string()))?;
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                builder.append_data(&mut header, &key.replace(':', "_"), &data[..])
                    .map_err(|e| CrushError::StorageError(e.to_string()))?;
            }
        }
        builder.finish().map_err(|e| CrushError::StorageError(e.to_string()))
    }

    pub fn import_tarball(&self, input: &Path) -> Result<Vec<String>> {
        let file = std::fs::File::open(input)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        let mut archive = tar::Archive::new(file);
        let mut imported = Vec::new();
        for entry in archive.entries()
            .map_err(|e| CrushError::StorageError(e.to_string()))? {
            if let Ok(mut entry) = entry {
                if let Ok(path) = entry.path() {
                    let key = format!("sha256:{}", path.to_string_lossy().replace('_', ":"));
                    let dest = self.layer_path(&key);
                    std::fs::create_dir_all(dest.parent().unwrap()).ok();
                    entry.unpack(&dest).ok();
                    imported.push(key);
                }
            }
        }
        Ok(imported)
    }

    fn layer_path(&self, key: &str) -> PathBuf {
        let filename = key.replace(':', "_");
        self.cache_dir.join("layers").join(filename)
    }
}
