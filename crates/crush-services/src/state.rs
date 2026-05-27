use std::path::{Path};
use std::fs;
use anyhow::{Result, Context};
use crate::driver::RunningService;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NativeServiceState {
    pub project: String,
    pub services: Vec<RunningService>,
    pub started_at: u64,
}

pub fn save_native_state(state_dir: &Path, state: &NativeServiceState) -> Result<()> {
    let native_dir = state_dir.join("native");
    fs::create_dir_all(&native_dir).context("Failed to create native state directory")?;
    let file_path = native_dir.join(format!("{}.json", state.project));
    let serialized = serde_json::to_string_pretty(state)?;
    fs::write(file_path, serialized)?;
    Ok(())
}

pub fn load_native_state(state_dir: &Path, project: &str) -> Option<NativeServiceState> {
    let file_path = state_dir.join("native").join(format!("{}.json", project));
    if !file_path.exists() {
        return None;
    }
    let content = fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn clear_native_state(state_dir: &Path, project: &str) {
    let file_path = state_dir.join("native").join(format!("{}.json", project));
    let _ = fs::remove_file(file_path);
}
