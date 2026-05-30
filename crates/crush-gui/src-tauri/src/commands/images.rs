use serde::Serialize;
use tauri::{State, Window};
use crate::state::AppState;
use crate::events;
use crush_types::StorageBackend;

#[derive(Debug, Clone, Serialize)]
pub struct ImageSummary {
    pub id: String,
    pub tag: String,
    pub digest: String,
    pub size_bytes: u64,
    pub layer_count: usize,
    pub os: String,
    pub arch: String,
}

/// Full image config — the data behind the inspect drawer (like `docker inspect`).
#[derive(Debug, Clone, Serialize)]
pub struct ImageDetail {
    pub id: String,
    pub tag: String,
    pub digest: String,
    pub size_bytes: u64,
    pub os: String,
    pub arch: String,
    pub entrypoint: Vec<String>,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
    pub layers: Vec<String>,
    pub config_digest: Option<String>,
}

#[tauri::command]
pub async fn list_images(state: State<'_, AppState>) -> Result<Vec<ImageSummary>, String> {
    let images = state.store.database().list_images().await.map_err(|e| e.to_string())?;
    let result = images.into_iter().map(|img| ImageSummary {
        id: img.id,
        tag: img.tag,
        digest: img.digest,
        size_bytes: img.size_bytes,
        layer_count: img.layers.len(),
        os: img.os,
        arch: img.architecture,
    }).collect();
    Ok(result)
}

#[tauri::command]
pub async fn inspect_image(id: String, state: State<'_, AppState>) -> Result<ImageDetail, String> {
    let images = state.store.database().list_images().await.map_err(|e| e.to_string())?;
    let img = images.into_iter().find(|i| i.id == id || i.digest == id)
        .ok_or_else(|| format!("Image not found: {id}"))?;
    Ok(ImageDetail {
        id: img.id,
        tag: img.tag,
        digest: img.digest,
        size_bytes: img.size_bytes,
        os: img.os,
        arch: img.architecture,
        entrypoint: img.entrypoint,
        cmd: img.cmd,
        env: img.env,
        layers: img.layers,
        config_digest: img.config_digest,
    })
}

#[tauri::command]
pub async fn pull_image(reference: String, window: Window, state: State<'_, AppState>) -> Result<String, String> {
    // Real pull via the same StorageBackend the CLI uses — single source of truth.
    // Per-blob streaming progress would need a callback hook in crush-image; until
    // then we emit an indeterminate "in progress" then a completion the UI can show.
    events::emit_pull_progress(&window, &reference, "downloading", 0, 0);
    let img = state.store.pull_image(&reference).await.map_err(|e| e.to_string())?;
    let layers = img.layers.len() as u64;
    events::emit_pull_progress(&window, &reference, "complete", layers, layers);
    Ok(img.id)
}

#[tauri::command]
pub async fn remove_image(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.database().delete_image(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}
