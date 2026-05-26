use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::{VolumeDriver, VolumeInfo};

#[derive(Debug, Clone)]
pub struct LocalDriver {
    pub data_dir: PathBuf,
}

impl LocalDriver {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn volumes_dir(&self) -> PathBuf {
        self.data_dir.join("volumes")
    }

    fn volume_dir(&self, name: &str) -> PathBuf {
        self.volumes_dir().join(name)
    }

    pub fn volume_json_path(&self, name: &str) -> PathBuf {
        self.volume_dir(name).join("volume.json")
    }
}

#[async_trait]
impl VolumeDriver for LocalDriver {
    async fn create(&self, name: &str, labels: HashMap<String, String>) -> Result<VolumeInfo> {
        let vol_dir = self.volume_dir(name);
        tokio::fs::create_dir_all(&vol_dir).await?;

        let data_dir = vol_dir.join("data");
        tokio::fs::create_dir_all(&data_dir).await?;

        let info = VolumeInfo {
            name: name.to_string(),
            driver: "local".to_string(),
            mountpoint: data_dir,
            created_at: chrono::Utc::now(),
            labels,
            ref_count: 0,
        };

        let json_path = self.volume_json_path(name);
        let serialized = serde_json::to_string_pretty(&info)?;
        tokio::fs::write(&json_path, serialized).await?;

        Ok(info)
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let info = self.inspect(name).await?;
        if info.ref_count > 0 {
            return Err(anyhow!("volume in use"));
        }

        let vol_dir = self.volume_dir(name);
        if vol_dir.exists() {
            tokio::fs::remove_dir_all(&vol_dir).await?;
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<VolumeInfo>> {
        let v_dir = self.volumes_dir();
        if !v_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        let mut entries = tokio::fs::read_dir(&v_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Ok(info) = self.inspect(&name).await {
                    list.push(info);
                }
            }
        }
        Ok(list)
    }

    async fn inspect(&self, name: &str) -> Result<VolumeInfo> {
        let json_path = self.volume_json_path(name);
        if !json_path.exists() {
            return Err(anyhow!("volume {} not found", name));
        }

        let content = tokio::fs::read_to_string(&json_path).await?;
        let info: VolumeInfo = serde_json::from_str(&content)?;
        Ok(info)
    }

    async fn path(&self, name: &str) -> Result<PathBuf> {
        Ok(self.volume_dir(name).join("data"))
    }
}
