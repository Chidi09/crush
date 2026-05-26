pub mod blob;
pub mod compress;
pub mod layer;
pub mod db;
pub mod multiarch;
pub mod gc;
pub mod lazy;

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::io::Read;
use crush_types::{StorageBackend, Image, Result, CrushError};
use blob::BlobStore;
use db::ImageDatabase;
use compress::CompressionFormat;
use layer::LayerExtractor;

pub use crush_registry::RegistryClientHandle;

pub struct ImageStore {
    base_dir: PathBuf,
    blobs: BlobStore,
    db: ImageDatabase,
    registry_client: Arc<Mutex<RegistryClientHandle>>,
    lazy_mode: bool,
}

impl ImageStore {
    pub async fn new(base_dir: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&base_dir).await
            .map_err(|e| CrushError::StorageError(format!("Failed to create base dir: {}", e)))?;
        let db = ImageDatabase::new(&base_dir)?;
        let blobs = BlobStore::new(&base_dir);
        Ok(Self {
            base_dir,
            blobs,
            db,
            registry_client: Arc::new(Mutex::new(RegistryClientHandle::default())),
            lazy_mode: false,
        })
    }

    pub fn enable_lazy_mode(&mut self) { self.lazy_mode = true; }
    pub fn blob_store(&self) -> &BlobStore { &self.blobs }
    pub fn database(&self) -> &ImageDatabase { &self.db }

    pub fn registry_for_tag(tag: &str) -> (String, String, String) {
        let tag = tag.trim_start_matches("docker://");
        let (registry, rest) = if let Some(pos) = tag.find('/') {
            let possible_registry = &tag[..pos];
            if possible_registry.contains('.') || possible_registry.contains(':') {
                (possible_registry.to_string(), tag[pos + 1..].to_string())
            } else {
                ("registry-1.docker.io".to_string(), tag.to_string())
            }
        } else {
            ("registry-1.docker.io".to_string(), tag.to_string())
        };
        let (image, reference) = if let Some(pos) = rest.rfind(':') {
            (rest[..pos].to_string(), rest[pos + 1..].to_string())
        } else {
            (rest.clone(), "latest".to_string())
        };
        let image = if !image.contains('/') { format!("library/{}", image) } else { image };
        (registry, image, reference)
    }

    pub async fn check_rate_limit(&self, resp: &reqwest::Response) -> Result<()> {
        if let Some(remaining) = resp.headers().get("RateLimit-Remaining") {
            if let Ok(val) = remaining.to_str().unwrap_or("9999").parse::<u32>() {
                if val < 5 {
                    let reset_str = resp.headers()
                        .get("RateLimit-Reset")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("60");
                    let _reset_secs: u64 = reset_str.parse().unwrap_or(60);
                }
            }
        }
        Ok(())
    }

    pub async fn set_registry_client(&self, client: RegistryClientHandle) {
        let mut c = self.registry_client.lock().await;
        *c = client;
    }
}

#[async_trait]
impl StorageBackend for ImageStore {
    async fn pull_image(&self, tag: &str) -> Result<Image> {
        let (registry, image, reference) = Self::registry_for_tag(tag);

        // Fetch manifests, then release the lock before store_image_from_manifest,
        // which also needs to acquire the registry client lock for blob downloads.
        let (final_manifest, digest) = {
            let client = self.registry_client.lock().await;
            let manifest_json = client.fetch_manifest(&registry, &image, &reference).await?;

            if manifest_json.get("manifests").is_some() {
                let entry = multiarch::MultiArchResolver::resolve_manifest(&manifest_json)?;
                let plat_digest = entry["digest"].as_str().unwrap_or("").to_string();
                let plat_manifest = client.fetch_manifest(&registry, &image, &plat_digest).await?;
                (plat_manifest, plat_digest)
            } else {
                let digest = manifest_json["config"]["digest"].as_str().unwrap_or("").to_string();
                (manifest_json, digest)
            }
        };

        self.store_image_from_manifest(&registry, &image, tag, &final_manifest, &digest).await
    }

    async fn push_image(&self, image_id: &str, registry: &str) -> Result<()> {
        let image = match self.db.get_image_by_digest(image_id).await? {
            Some(img) => img,
            None => self.db.get_image_by_tag(image_id).await?
                .ok_or_else(|| CrushError::ContainerNotFound(format!("Image {} not found", image_id)))?,
        };

        let client = self.registry_client.lock().await;
        let (reg, img, _) = Self::registry_for_tag(registry);

        for layer_digest in &image.layers {
            if self.blobs.contains(layer_digest) {
                let blob_data = self.blobs.read_blob(layer_digest)?;
                let tmp = std::env::temp_dir().join(format!("crush_push_{}", hex::encode(&blob_data[..8.min(blob_data.len())])));
                std::fs::write(&tmp, &blob_data).ok();
                client.upload_blob(&reg, &img, &tmp).await?;
                let _ = std::fs::remove_file(&tmp);
            }
        }

        let manifest_path = self.base_dir.join("manifests").join(&image.id);
        if manifest_path.exists() {
            let manifest_str = tokio::fs::read_to_string(&manifest_path).await
                .map_err(|e| CrushError::StorageError(format!("Failed to read manifest: {}", e)))?;
            client.put_manifest(&reg, &img, &image.tag, &manifest_str).await?;
        }

        Ok(())
    }

    async fn list_images(&self) -> Result<Vec<Image>> {
        self.db.list_images().await
    }

    async fn delete_image(&self, image_id: &str) -> Result<()> {
        self.db.delete_image(image_id).await
    }

    async fn extract_layers(&self, image_id: &str, destination: &PathBuf) -> Result<()> {
        let image = match self.db.get_image_by_digest(image_id).await? {
            Some(img) => img,
            None => self.db.get_image_by_tag(image_id).await?
                .ok_or_else(|| CrushError::ContainerNotFound(format!("Image {} not found", image_id)))?,
        };

        tokio::fs::create_dir_all(destination).await
            .map_err(|e| CrushError::StorageError(format!("Failed to create destination: {}", e)))?;

        for layer_digest in &image.layers {
            if !self.blobs.contains(layer_digest) {
                continue;
            }
            let blob_file = self.blobs.read_blob_stream(layer_digest)?;
            // ⚠ FIX: Actually read blob bytes for format detection, not zero-initialized
            let mut header = [0u8; 2];
            let format = if (&blob_file).read_exact(&mut header).is_ok() {
                compress::detect_format(&header)
            } else {
                CompressionFormat::Gzip
            };

            let blob_file = self.blobs.read_blob_stream(layer_digest)?;
            let extractor = LayerExtractor::new(destination);
            extractor.extract_layer_streamed(blob_file, format)?;
        }

        Ok(())
    }
}

impl ImageStore {
    async fn store_image_from_manifest(
        &self, registry: &str, image_name: &str, tag: &str,
        manifest: &serde_json::Value, digest: &str,
    ) -> Result<Image> {
        let layers: Vec<String> = manifest["layers"].as_array()
            .map(|a| a.iter().filter_map(|l| l["digest"].as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let config_digest = manifest["config"]["digest"].as_str().unwrap_or("").to_string();

        if !self.blobs.contains(digest) {
            let manifest_str = serde_json::to_string(manifest)
                .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
            self.blobs.atomic_write(manifest_str.as_bytes())?;
        }

        // Fetch config blob and parse OCI image config for entrypoint/cmd/env
        let mut entrypoint = Vec::new();
        let mut cmd_vec = Vec::new();
        let mut env_vec = Vec::new();

        if !config_digest.is_empty() {
            let config_data = if !self.blobs.contains(&config_digest) {
                let client = self.registry_client.lock().await;
                match client.fetch_blob(registry, image_name, &config_digest).await {
                    Ok(data) => { self.blobs.atomic_write(&data).ok(); Some(data) }
                    Err(_) => None,
                }
            } else {
                self.blobs.read_blob(&config_digest).ok()
            };

            if let Some(data) = config_data {
                if let Ok(cfg) = serde_json::from_slice::<serde_json::Value>(&data) {
                    if let Some(arr) = cfg["config"]["Entrypoint"].as_array() {
                        entrypoint = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(arr) = cfg["config"]["Cmd"].as_array() {
                        cmd_vec = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(arr) = cfg["config"]["Env"].as_array() {
                        env_vec = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                }
            }
        }

        // Download layer blobs, store as-is (OCI blobs are content-addressed by their sha256,
        // so atomic_write returns the OCI digest unchanged). No re-compression — that would
        // create a double-compressed zstd(gzip(tar)) that extract_layers can't handle.
        let mut stored_digests = Vec::new();
        for layer_digest in &layers {
            if self.blobs.contains(layer_digest) {
                stored_digests.push(layer_digest.clone());
                continue;
            }

            let client = self.registry_client.lock().await;
            let blob_data = client.fetch_blob(registry, image_name, layer_digest).await?;
            drop(client);

            let stored = self.blobs.atomic_write(&blob_data)?;
            stored_digests.push(stored);
        }

        let total_size: u64 = stored_digests.iter()
            .filter_map(|d| std::fs::metadata(self.blobs.path_for_digest(d)).ok().map(|m| m.len()))
            .sum();

        let image_id = if digest.is_empty() {
            stored_digests.first().cloned()
                .unwrap_or_else(|| format!("sha256:{}", hex::encode(tag.as_bytes())))
        } else {
            digest.to_string()
        };

        let image = Image {
            id: image_id.clone(),
            tag: tag.to_string(),
            digest: image_id,
            size_bytes: total_size,
            layers: stored_digests,
            architecture: "amd64".to_string(),
            os: "linux".to_string(),
            entrypoint,
            cmd: cmd_vec,
            env: env_vec,
            config_digest: if config_digest.is_empty() { None } else { Some(config_digest) },
        };

        self.db.put_image(&image).await?;

        Ok(image)
    }
}
