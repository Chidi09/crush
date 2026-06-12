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
        crush_types::dirs_or_default()
    }
}
