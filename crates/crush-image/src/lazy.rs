use std::path::PathBuf;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError};
use sha2::{Sha256, Digest};

const CHUNK_SIZE: usize = 8 * 1024 * 1024;
const PREFETCH_AHEAD: usize = 3;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkManifest {
    pub layer_digest: String,
    pub chunk_size: u64,
    pub chunks: Vec<ChunkEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkEntry {
    pub index: u32,
    pub offset: u64,
    pub size: u32,
    pub sha256: String,
}

pub struct LazyLoader {
    pub layer_dir: PathBuf,
    pub chunk_manifest: Option<ChunkManifest>,
    prefetch_queue: Arc<Mutex<VecDeque<u32>>>,
    cache: Arc<Mutex<lru::LruCache<u32, Vec<u8>>>>,
    registry_client: Arc<Mutex<Option<crate::RegistryClientHandle>>>,
    registry: String,
    image: String,
}

impl LazyLoader {
    pub fn new(layer_dir: PathBuf, registry: String, image: String) -> Self {
        Self {
            layer_dir,
            chunk_manifest: None,
            prefetch_queue: Arc::new(Mutex::new(VecDeque::new())),
            cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(64).unwrap(),
            ))),
            registry_client: Arc::new(Mutex::new(None)),
            registry,
            image,
        }
    }

    pub fn set_registry_client(&self, client: crate::RegistryClientHandle) {
        let mut c = self.registry_client.blocking_lock();
        *c = Some(client);
    }

    /// Split `data` into CHUNK_SIZE chunks, write each to disk, and return
    /// the manifest. `layer_digest` is stored in the manifest for range fetches.
    pub fn load_from_blob(&mut self, data: &[u8], layer_digest: &str) -> Result<ChunkManifest> {
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut index = 0u32;

        while (offset as usize) < data.len() {
            let end = ((offset as usize) + CHUNK_SIZE).min(data.len());
            let chunk_data = &data[offset as usize..end];

            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hex::encode(hasher.finalize());

            let chunk_path = self.layer_dir.join(format!("chunk_{:06}", index));
            std::fs::write(&chunk_path, chunk_data)
                .map_err(|e| CrushError::StorageError(format!("Failed to write chunk {}: {}", index, e)))?;

            chunks.push(ChunkEntry { index, offset, size: chunk_data.len() as u32, sha256: hash });
            offset = end as u64;
            index += 1;
        }

        let manifest = ChunkManifest {
            layer_digest: layer_digest.to_string(),
            chunk_size: CHUNK_SIZE as u64,
            chunks,
        };

        let manifest_path = self.layer_dir.join("chunk_manifest.json");
        let json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| CrushError::ImageError(format!("Manifest serialization: {}", e)))?;
        std::fs::write(&manifest_path, json)
            .map_err(|e| CrushError::StorageError(format!("Failed to write manifest: {}", e)))?;

        self.chunk_manifest = Some(manifest.clone());
        Ok(manifest)
    }

    pub fn generate_chunk_manifest(&self, data: &[u8]) -> ChunkManifest {
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut index = 0u32;

        while (offset as usize) < data.len() {
            let end = ((offset as usize) + CHUNK_SIZE).min(data.len());
            let chunk_data = &data[offset as usize..end];

            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let hash = hex::encode(hasher.finalize());

            chunks.push(ChunkEntry {
                index,
                offset,
                size: chunk_data.len() as u32,
                sha256: hash,
            });
            offset = end as u64;
            index += 1;
        }

        ChunkManifest {
            layer_digest: String::new(),
            chunk_size: CHUNK_SIZE as u64,
            chunks,
        }
    }

    pub async fn store_chunk_manifest(&self, manifest: &ChunkManifest) -> Result<()> {
        let path = self.layer_dir.join("chunk_manifest.json");
        let data = serde_json::to_string_pretty(manifest)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        tokio::fs::write(&path, data).await
            .map_err(|e| CrushError::StorageError(format!("Failed to write chunk manifest: {}", e)))?;
        Ok(())
    }

    pub async fn get_chunk(&self, index: u32) -> Result<Vec<u8>> {
        {
            let mut cache = self.cache.lock().await;
            if let Some(data) = cache.get(&index) {
                self.schedule_prefetch(index).await;
                return Ok(data.clone());
            }
        }

        let manifest = self.chunk_manifest.as_ref()
            .ok_or_else(|| CrushError::ImageError("No chunk manifest loaded".to_string()))?;

        let chunk_entry = manifest.chunks.iter()
            .find(|c| c.index == index)
            .ok_or_else(|| CrushError::ImageError(format!("Chunk index {} not found", index)))?
            .clone();

        let chunk_path = self.layer_dir.join(format!("chunk_{:06}", index));
        let data = if chunk_path.exists() {
            tokio::fs::read(&chunk_path).await
                .map_err(|e| CrushError::StorageError(format!("Failed to read chunk: {}", e)))?
        } else {
            let fetched = self.fetch_chunk_from_remote(index).await?;
            tokio::fs::write(&chunk_path, &fetched).await.ok();
            fetched
        };

        {
            let mut cache = self.cache.lock().await;
            cache.put(index, data.clone());
        }

        self.schedule_prefetch(index).await;
        Ok(data)
    }

    async fn schedule_prefetch(&self, current_index: u32) {
        let manifest = match &self.chunk_manifest {
            Some(m) => m,
            None => return,
        };

        let total = manifest.chunks.len() as u32;
        let mut queue = self.prefetch_queue.lock().await;
        for i in 1..=(PREFETCH_AHEAD as u32) {
            let next = current_index + i;
            if next < total && !queue.contains(&next) {
                queue.push_back(next);
            }
        }
        // Actual prefetch is handled lazily on the next get_chunk call that hits disk.
        // Spawning a background task here would require Arc<Self> — deferred for now.
    }

    async fn fetch_chunk_from_remote(&self, index: u32) -> Result<Vec<u8>> {
        let manifest = self.chunk_manifest.as_ref()
            .ok_or_else(|| CrushError::ImageError("No manifest".into()))?;
        let entry = manifest.chunks.iter().find(|c| c.index == index)
            .ok_or_else(|| CrushError::ImageError("Chunk not found".into()))?;

        let client_guard = self.registry_client.lock().await;
        if let Some(client) = client_guard.as_ref() {
            let start = entry.offset;
            let end = entry.offset + entry.size as u64 - 1;
            client.fetch_blob_range(
                &self.registry,
                &self.image,
                &manifest.layer_digest,
                start,
                end,
            ).await.map_err(|e| CrushError::NetworkError(format!("Lazy fetch failed: {}", e)))
        } else {
            Err(CrushError::ImageError("No registry client configured".into()))
        }
    }
}
