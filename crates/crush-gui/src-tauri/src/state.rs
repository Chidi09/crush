use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crush_image::ImageStore;
use crush_ai::AiEngine;
use uuid::Uuid;
use tokio::sync::mpsc;

pub type RunId = Uuid;

pub struct RunProcess {
    pub abort: tokio::sync::oneshot::Sender<()>,
}

pub struct LogTailer {
    pub shutdown: tokio::sync::oneshot::Sender<()>,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub store: Arc<ImageStore>,
    pub ai: Arc<AiEngine>,
    pub runs: Arc<RwLock<HashMap<RunId, RunProcess>>>,
    pub log_tailers: Arc<RwLock<HashMap<String, LogTailer>>>,
}

impl AppState {
    pub fn data_dir() -> PathBuf {
        let base = if cfg!(target_os = "linux") {
            PathBuf::from("/var/lib/crush")
        } else if cfg!(target_os = "windows") {
            let local_app_data = std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Local",
                    std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string())));
            PathBuf::from(local_app_data).join("Crush")
        } else {
            dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("crush")
        };
        std::fs::create_dir_all(&base).ok();
        base
    }
}
