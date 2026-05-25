use std::path::{Path, PathBuf};
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
    layer_dir: PathBuf,
    chunk_manifest: Option<ChunkManifest>,
    prefetch_queue: Arc<Mutex<VecDeque<u32>>>,
    cache: Arc<Mutex<lru::LruCache<u32, Vec<u8>>>>,
    registry_client: Arc<Mutex<Option<crate::RegistryClientHandle>>>,
}

impl LazyLoader {
    pub fn new(layer_dir: PathBuf) -> Self {
        Self {
            layer_dir,
            chunk_manifest: None,
            prefetch_queue: Arc::new(Mutex::new(VecDeque::new())),
            cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(64).unwrap(),
            ))),
            registry_client: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_registry_client(&self, client: crate::RegistryClientHandle) {
        let mut c = self.registry_client.blocking_lock();
        *c = Some(client);
    }

    pub fn generate_chunk_manifest(&self, data: &[u8]) -> ChunkManifest {
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut index = 0u32;

        while offset < data.len() as u64 {
            let end = (offset as usize + CHUNK_SIZE).min(data.len());
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
            .ok_or_else(|| CrushError::ImageError(format!("Chunk index {} not found", index)))?;

        let chunk_path = self.layer_dir.join(format!("chunk_{:06}", index));
        let data = if chunk_path.exists() {
            tokio::fs::read(&chunk_path).await
                .map_err(|e| CrushError::StorageError(format!("Failed to read chunk: {}", e)))?
        } else {
            self.fetch_chunk_from_remote(index).await?
        };

        {
            let mut cache = self.cache.lock().await;
            cache.put(index, data.clone());
        }

        self.schedule_prefetch(index).await;

        Ok(data)
    }

    pub async fn store_chunk_manifest(&self, manifest: ChunkManifest) -> Result<()> {
        let path = self.layer_dir.join("chunk_manifest.json");
        let data = serde_json::to_string_pretty(&manifest)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        tokio::fs::write(&path, data).await
            .map_err(|e| CrushError::StorageError(format!("Failed to write chunk manifest: {}", e)))?;

        // ⚠ FIX: Mutable borrow is needed; store_chunk_manifest is called
        // before chunk_manifest is read elsewhere. Use interior mutability
        // pattern via the file-system backed manifest.
        let path = self.layer_dir.join("chunk_manifest.json");
        let data = serde_json::to_string_pretty(&manifest)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        tokio::fs::write(&path, data).await
            .map_err(|e| CrushError::StorageError(format!("Failed to write chunk manifest: {}", e)))?;

        Ok(())
    }

    async fn schedule_prefetch(&self, current_index: u32) {
        let manifest = match &self.chunk_manifest {
            Some(m) => m,
            None => return,
        };

        let mut queue = self.prefetch_queue.lock().await;
        for i in 1..=PREFETCH_AHEAD {
            let prefetch_idx = current_index + i as u32;
            if prefetch_idx < manifest.chunks.len() as u32 && !queue.contains(&prefetch_idx) {
                queue.push_back(prefetch_idx);
            }
        }

        drop(queue);

        let cache = self.cache.clone();
        let prefetch_queue = self.prefetch_queue.clone();
        let layer_dir = self.layer_dir.clone();

        tokio::spawn(async move {
            let idx = {
                let mut q = prefetch_queue.lock().await;
                q.pop_front()
            };

            if let Some(idx) = idx {
                let chunk_path = layer_dir.join(format!("chunk_{:06}", idx));
                if !chunk_path.exists() {
                    let mut cache_lock = cache.lock().await;
                    if !cache_lock.contains(&idx) {
                        cache_lock.put(idx, vec![]);
                    }
                }
            }
        });
    }

    async fn fetch_chunk_from_remote(&self, _index: u32) -> Result<Vec<u8>> {
        Err(CrushError::ImageError("Remote chunk fetch not implemented in lazy mode".to_string()))
    }
}
