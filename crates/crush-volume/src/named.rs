use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetadata {
    pub name: String,
    pub id: String,
    pub driver: String,
    pub mountpoint: PathBuf,
    pub created_at: String,
    pub labels: Vec<(String, String)>,
    pub size_bytes: Option<u64>,
    pub ref_count: u32,
}

pub struct NamedVolumeManager {
    base_dir: PathBuf,
    db: Arc<Mutex<sled::Db>>,
}

impl NamedVolumeManager {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let volumes_dir = base_dir.join("volumes");
        std::fs::create_dir_all(&volumes_dir)
            .map_err(|e| CrushError::StorageError(format!("Failed to create volumes dir: {}", e)))?;

        let db_path = base_dir.join("volumes.db");
        let db = sled::open(&db_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to open volume db: {}", e)))?;

        Ok(Self {
            base_dir,
            db: Arc::new(Mutex::new(db)),
        })
    }

    pub async fn create(&self, name: &str, driver: &str, labels: Vec<(String, String)>) -> Result<VolumeMetadata> {
        let mut db = self.db.lock().await;
        let key = format!("vol:{}", name);
        if db.contains_key(key.as_bytes())
            .map_err(|e| CrushError::StorageError(e.to_string()))? {
            return Err(CrushError::ContainerAlreadyExists(format!("Volume '{}' already exists", name)));
        }

        let volume_dir = self.base_dir.join("volumes").join(name);
        std::fs::create_dir_all(&volume_dir)
            .map_err(|e| CrushError::StorageError(format!("Failed to create volume dir: {}", e)))?;

        let meta = VolumeMetadata {
            name: name.to_string(),
            id: format!("vol_{}", uuid::Uuid::new_v4()),
            driver: driver.to_string(),
            mountpoint: volume_dir,
            created_at: Utc::now().to_rfc3339(),
            labels,
            size_bytes: None,
            ref_count: 0,
        };

        let value = serde_json::to_vec(&meta)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        db.insert(key.as_bytes(), value)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        db.flush()
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        Ok(meta)
    }

    pub async fn get(&self, name: &str) -> Result<VolumeMetadata> {
        let db = self.db.lock().await;
        let key = format!("vol:{}", name);
        let value = db.get(key.as_bytes())
            .map_err(|e| CrushError::StorageError(e.to_string()))?
            .ok_or_else(|| CrushError::ContainerNotFound(format!("Volume '{}' not found", name)))?;
        serde_json::from_slice(&value)
            .map_err(|e| CrushError::ImageError(format!("Deserialization error: {}", e)))
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let mut db = self.db.lock().await;
        let key = format!("vol:{}", name);

        let meta = self.get(name).await?;
        if meta.ref_count > 0 {
            return Err(CrushError::StorageError(format!(
                "Volume '{}' is in use by {} container(s). Force remove to override.", name, meta.ref_count
            )));
        }

        let vol_dir = self.base_dir.join("volumes").join(name);
        if vol_dir.exists() {
            std::fs::remove_dir_all(&vol_dir)
                .map_err(|e| CrushError::StorageError(format!("Failed to remove volume dir: {}", e)))?;
        }

        db.remove(key.as_bytes())
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        db.flush()
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<VolumeMetadata>> {
        let db = self.db.lock().await;
        let mut volumes = Vec::new();
        for result in db.scan_prefix("vol:") {
            let (_, value) = result
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            if let Ok(meta) = serde_json::from_slice::<VolumeMetadata>(&value) {
                volumes.push(meta);
            }
        }
        Ok(volumes)
    }

    pub async fn increment_ref(&self, name: &str) -> Result<()> {
        let mut db = self.db.lock().await;
        let key = format!("vol:{}", name);
        if let Some(value) = db.get(key.as_bytes())
            .map_err(|e| CrushError::StorageError(e.to_string()))? {
            let mut meta: VolumeMetadata = serde_json::from_slice(&value)
                .map_err(|e| CrushError::ImageError(e.to_string()))?;
            meta.ref_count += 1;
            let new_value = serde_json::to_vec(&meta)
                .map_err(|e| CrushError::ImageError(e.to_string()))?;
            db.insert(key.as_bytes(), new_value)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn decrement_ref(&self, name: &str) -> Result<()> {
        let mut db = self.db.lock().await;
        let key = format!("vol:{}", name);
        if let Some(value) = db.get(key.as_bytes())
            .map_err(|e| CrushError::StorageError(e.to_string()))? {
            let mut meta: VolumeMetadata = serde_json::from_slice(&value)
                .map_err(|e| CrushError::ImageError(e.to_string()))?;
            meta.ref_count = meta.ref_count.saturating_sub(1);
            let new_value = serde_json::to_vec(&meta)
                .map_err(|e| CrushError::ImageError(e.to_string()))?;
            db.insert(key.as_bytes(), new_value)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
        }
        Ok(())
    }

    pub fn volume_path(&self, name: &str) -> PathBuf {
        self.base_dir.join("volumes").join(name)
    }
}
