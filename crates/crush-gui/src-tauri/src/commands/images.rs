use serde::Serialize;
use tauri::{State, Window};
use crate::state::AppState;
use crate::events;

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
    let images = state.store.list_images().await.map_err(|e| e.to_string())?;
    let result = images.into_iter().map(|img| ImageSummary {
        id: img.id,
        tag: img.tag,
        digest: img.digest,
        size_bytes: img.size_bytes,
        layer_count: img.layers.len(),
        created_at: img.id.chars().take(12).collect(),
    }).collect();
    Ok(result)
}

#[tauri::command]
pub async fn pull_image(reference: String, window: Window, state: State<'_, AppState>) -> Result<String, String> {
    let img = state.store.pull_image(&reference).await.map_err(|e| e.to_string())?;
    events::emit_container_state_changed(&window);
    Ok(img.id)
}

#[tauri::command]
pub async fn remove_image(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.delete_image(&id).await.map_err(|e| e.to_string())?;
    Ok(())
}
