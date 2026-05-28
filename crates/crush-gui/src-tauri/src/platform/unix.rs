use std::path::Path;

pub fn xdg_data_dir() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("crush"))
}

pub fn ensure_desktop_file(path: &Path) {
    // Placeholder for .desktop file creation on Linux
}
