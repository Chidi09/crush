use serde::Serialize;
use tauri::{State, Window};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ImageSummary {
    pub id: String,
    pub tag: String,
    pub digest: String,
    pub size_bytes: u64,
    pub layer_count: usize,
    pub created_at: String,
}

#[tauri::command]
pub async fn list_images(state: State<'_, AppState>) -> Result<Vec<ImageSummary>, String> {
    let images = state.store.database().list_images().await.map_err(|e| e.to_string())?;
    let result = images.into_iter().map(|img| {
        let short_id: String = img.id.chars().take(12).collect();
        ImageSummary {
            id: img.id,
            tag: img.tag,
            digest: img.digest,
            size_bytes: img.size_bytes,
            layer_count: img.layers.len(),
            created_at: short_id,
        }
    }).collect();
    Ok(result)
}

#[tauri::command]
pub async fn pull_image(_reference: String, _window: Window, _state: State<'_, AppState>) -> Result<String, String> {
    // Registry pull with per-layer progress lands with the Images screen (v0.8.0 ship
    // phase); the alpha is list-only. See CRUSH_V8_PLAN.md.
    Err("Image pull is not yet implemented in this build".to_string())
}

#[tauri::command]
pub async fn remove_image(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.database().delete_image(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}
