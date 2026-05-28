use std::path::PathBuf;

#[tauri::command]
pub async fn pick_project_directory() -> Result<Option<String>, String> {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Select project directory")
        .pick_folder()
        .await;
    Ok(folder.map(|p| p.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

#[tauri::command]
pub async fn reveal_in_explorer(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    open::that(p.parent().unwrap_or(&p)).map_err(|e| format!("Failed to open explorer: {}", e))
}
