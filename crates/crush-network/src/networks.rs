use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};
use crate::bridge::BridgeManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNetwork {
    pub name: String,
    pub id: String,
    pub subnet: String,
    pub gateway: String,
    pub bridge_name: String,
    pub containers: Vec<String>,
}

pub struct NetworkManager {
    db: PathBuf,
    networks: Arc<Mutex<HashMap<String, UserNetwork>>>,
}

impl NetworkManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let db = data_dir.join("networks.json");
        let networks = Arc::new(Mutex::new(HashMap::new()));

        if db.exists() {
            if let Ok(content) = std::fs::read_to_string(&db) {
                if let Ok(nets) = serde_json::from_str::<Vec<UserNetwork>>(&content) {
                    let mut map = networks.blocking_lock();
                    for net in nets {
                        map.insert(net.name.clone(), net);
                    }
                }
            }
        }

        Self { db, networks }
    }

    pub async fn create(&self, name: &str, subnet: &str, gateway: &str) -> Result<UserNetwork> {
        let mut nets = self.networks.lock().await;
        if nets.contains_key(name) {
            return Err(CrushError::NetworkError(format!("Network '{}' already exists", name)));
        }

        let bridge_name = format!("br_{}", &name[..8.min(name.len())]);
        BridgeManager::ensure_bridge(&bridge_name, subnet)?;

        let network = UserNetwork {
            name: name.to_string(),
            id: format!("net_{}", hex_encode_random()),
            subnet: subnet.to_string(),
            gateway: gateway.to_string(),
            bridge_name,
            containers: Vec::new(),
        };

        nets.insert(name.to_string(), network.clone());
        self.persist().await?;

        Ok(network)
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let mut nets = self.networks.lock().await;
        if let Some(net) = nets.remove(name) {
            let _ = BridgeManager::delete_bridge(&net.bridge_name);
            self.persist().await?;
        }
        Ok(())
    }

    pub async fn connect(&self, network_name: &str, container_id: &str, container_ip: &str) -> Result<()> {
        let mut nets = self.networks.lock().await;
        if let Some(net) = nets.get_mut(network_name) {
            net.containers.push(format!("{}={}", container_id, container_ip));
            self.persist().await?;
        }
        Ok(())
    }

    pub async fn disconnect(&self, network_name: &str, container_id: &str) -> Result<()> {
        let mut nets = self.networks.lock().await;
        if let Some(net) = nets.get_mut(network_name) {
            net.containers.retain(|c| !c.starts_with(&format!("{}=", container_id)));
            self.persist().await?;
        }
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<UserNetwork>> {
        let nets = self.networks.lock().await;
        Ok(nets.values().cloned().collect())
    }

    pub async fn inspect(&self, name: &str) -> Result<UserNetwork> {
        let nets = self.networks.lock().await;
        nets.get(name).cloned()
            .ok_or_else(|| CrushError::ContainerNotFound(format!("Network '{}' not found", name)))
    }

    async fn persist(&self) -> Result<()> {
        let nets = self.networks.lock().await;
        let list: Vec<UserNetwork> = nets.values().cloned().collect();
        let data = serde_json::to_string_pretty(&list)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
        tokio::fs::write(&self.db, data).await
            .map_err(|e| CrushError::StorageError(format!("Write error: {}", e)))
    }
}

fn hex_encode_random() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}", nanos)
}
