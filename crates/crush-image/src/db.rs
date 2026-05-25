use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError, Image};

pub struct ImageDatabase {
    db: Arc<Mutex<sled::Db>>,
    base_dir: std::path::PathBuf,
}

impl ImageDatabase {
    pub fn new(base_dir: &Path) -> Result<Self> {
        let db_path = base_dir.join("db");
        let db = sled::open(&db_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to open sled db: {}", e)))?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            base_dir: base_dir.to_path_buf(),
        })
    }

    pub async fn put_image(&self, image: &Image) -> Result<()> {
        let db = self.db.lock().await;

        let tag_key = format!("tag:{}", image.tag);
        let tag_value = serde_json::to_vec(image)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        db.insert(tag_key.as_bytes(), tag_value.as_slice())
            .map_err(|e| CrushError::StorageError(format!("DB insert error: {}", e)))?;

        let digest_key = format!("digest:{}", image.digest);
        db.insert(digest_key.as_bytes(), tag_value.as_slice())
            .map_err(|e| CrushError::StorageError(format!("DB insert error: {}", e)))?;

        let id_key = format!("id:{}", image.id);
        let id_value = serde_json::to_vec(&image.tag)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        db.insert(id_key.as_bytes(), id_value)
            .map_err(|e| CrushError::StorageError(format!("DB insert error: {}", e)))?;

        db.flush()
            .map_err(|e| CrushError::StorageError(format!("DB flush error: {}", e)))?;

        Ok(())
    }

    pub async fn get_image_by_tag(&self, tag: &str) -> Result<Option<Image>> {
        let db = self.db.lock().await;
        let key = format!("tag:{}", tag);
        if let Some(value) = db.get(key.as_bytes())
            .map_err(|e| CrushError::StorageError(format!("DB get error: {}", e)))? {
            let image: Image = serde_json::from_slice(&value)
                .map_err(|e| CrushError::ImageError(format!("Deserialization error: {}", e)))?;
            Ok(Some(image))
        } else {
            Ok(None)
        }
    }

    pub async fn get_image_by_digest(&self, digest: &str) -> Result<Option<Image>> {
        let db = self.db.lock().await;
        let key = format!("digest:{}", digest);
        if let Some(value) = db.get(key.as_bytes())
            .map_err(|e| CrushError::StorageError(format!("DB get error: {}", e)))? {
            let image: Image = serde_json::from_slice(&value)
                .map_err(|e| CrushError::ImageError(format!("Deserialization error: {}", e)))?;
            Ok(Some(image))
        } else {
            Ok(None)
        }
    }

    pub async fn list_images(&self) -> Result<Vec<Image>> {
        let db = self.db.lock().await;
        let mut images = Vec::new();
        for result in db.scan_prefix("tag:") {
            let (_, value) = result
                .map_err(|e| CrushError::StorageError(format!("DB scan error: {}", e)))?;
            if let Ok(image) = serde_json::from_slice::<Image>(&value) {
                images.push(image);
            }
        }
        Ok(images)
    }

    pub async fn delete_image(&self, tag_or_digest: &str) -> Result<()> {
        let db = self.db.lock().await;

        let tag_key = format!("tag:{}", tag_or_digest);
        let digest_key = format!("digest:{}", tag_or_digest);

        if let Some(value) = db.get(tag_key.as_bytes())
            .map_err(|e| CrushError::StorageError(format!("DB get error: {}", e)))? {
            if let Ok(image) = serde_json::from_slice::<Image>(&value) {
                db.remove(format!("tag:{}", image.tag).as_bytes())
                    .map_err(|e| CrushError::StorageError(format!("DB remove error: {}", e)))?;
                db.remove(format!("digest:{}", image.digest).as_bytes())
                    .map_err(|e| CrushError::StorageError(format!("DB remove error: {}", e)))?;
                db.remove(format!("id:{}", image.id).as_bytes())
                    .map_err(|e| CrushError::StorageError(format!("DB remove error: {}", e)))?;
                return Ok(());
            }
        }

        if let Some(value) = db.get(digest_key.as_bytes())
            .map_err(|e| CrushError::StorageError(format!("DB get error: {}", e)))? {
            if let Ok(image) = serde_json::from_slice::<Image>(&value) {
                db.remove(format!("tag:{}", image.tag).as_bytes())
                    .map_err(|e| CrushError::StorageError(format!("DB remove error: {}", e)))?;
                db.remove(format!("digest:{}", image.digest).as_bytes())
                    .map_err(|e| CrushError::StorageError(format!("DB remove error: {}", e)))?;
                db.remove(format!("id:{}", image.id).as_bytes())
                    .map_err(|e| CrushError::StorageError(format!("DB remove error: {}", e)))?;
                return Ok(());
            }
        }

        Err(CrushError::ContainerNotFound(format!("Image {} not found", tag_or_digest)))
    }

    pub async fn image_count(&self) -> Result<u64> {
        let db = self.db.lock().await;
        let mut count = 0u64;
        for _ in db.scan_prefix("tag:") {
            count += 1;
        }
        Ok(count)
    }
}
