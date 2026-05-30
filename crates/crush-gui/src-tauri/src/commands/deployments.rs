use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use crate::state::AppState;

/// A persisted record of one `crush run`, modelled after a Vercel deployment:
/// each run is frozen with its own build/runtime logs and (optionally) a cached
/// preview screenshot, so past runs can be replayed without re-running.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub id: String,
    pub project: String,
    pub project_path: String,
    pub created_ms: i64,
    #[serde(default)]
    pub ended_ms: Option<i64>,
    #[serde(default)]
    pub duration_ms: i64,
    /// "running" | "ready" | "failed"
    pub status: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub build_log: String,
    #[serde(default)]
    pub runtime_log: String,
    #[serde(default)]
    pub has_screenshot: bool,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub commit_short: Option<String>,
    #[serde(default)]
    pub commit_message: Option<String>,
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

fn project_dir(data_dir: &PathBuf, project: &str) -> PathBuf {
    data_dir.join("deployments").join(sanitize(project))
}

#[tauri::command]
pub async fn save_deployment(record: DeploymentRecord, state: State<'_, AppState>) -> Result<(), String> {
    let dir = project_dir(&state.data_dir, &record.project);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", sanitize(&record.id)));
    // Preserve a screenshot flag already set by capture_preview.
    let mut rec = record;
    if dir.join(format!("{}.png", sanitize(&rec.id))).exists() {
        rec.has_screenshot = true;
    }
    let json = serde_json::to_string_pretty(&rec).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_deployments(project: String, state: State<'_, AppState>) -> Result<Vec<DeploymentRecord>, String> {
    let dir = project_dir(&state.data_dir, &project);
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "json").unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    if let Ok(mut rec) = serde_json::from_str::<DeploymentRecord>(&text) {
                        // Keep list payloads light — drop log bodies (loaded on open).
                        rec.build_log = String::new();
                        rec.runtime_log = String::new();
                        out.push(rec);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_ms.cmp(&a.created_ms));
    Ok(out)
}

/// Every deployment across all projects (newest first), for the global
/// Deployments view. Log bodies are dropped to keep the payload light.
#[tauri::command]
pub async fn list_all_deployments(state: State<'_, AppState>) -> Result<Vec<DeploymentRecord>, String> {
    let root = state.data_dir.join("deployments");
    let mut out = Vec::new();
    if let Ok(projects) = std::fs::read_dir(&root) {
        for proj in projects.flatten() {
            if !proj.path().is_dir() { continue; }
            if let Ok(entries) = std::fs::read_dir(proj.path()) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().map(|x| x == "json").unwrap_or(false) {
                        if let Ok(text) = std::fs::read_to_string(&p) {
                            if let Ok(mut rec) = serde_json::from_str::<DeploymentRecord>(&text) {
                                rec.build_log = String::new();
                                rec.runtime_log = String::new();
                                out.push(rec);
                            }
                        }
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_ms.cmp(&a.created_ms));
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct DeploymentDetail {
    #[serde(flatten)]
    pub record: DeploymentRecord,
    /// data: URL of the cached screenshot (if any)
    pub screenshot: Option<String>,
}

#[tauri::command]
pub async fn get_deployment(project: String, id: String, state: State<'_, AppState>) -> Result<DeploymentDetail, String> {
    let dir = project_dir(&state.data_dir, &project);
    let json_path = dir.join(format!("{}.json", sanitize(&id)));
    let text = std::fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
    let record: DeploymentRecord = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let png_path = dir.join(format!("{}.png", sanitize(&id)));
    let screenshot = std::fs::read(&png_path).ok().map(|bytes| {
        use base64::Engine;
        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
    });

    Ok(DeploymentDetail { record, screenshot })
}

#[tauri::command]
pub async fn delete_deployment(project: String, id: String, state: State<'_, AppState>) -> Result<(), String> {
    let dir = project_dir(&state.data_dir, &project);
    let _ = std::fs::remove_file(dir.join(format!("{}.json", sanitize(&id))));
    let _ = std::fs::remove_file(dir.join(format!("{}.png", sanitize(&id))));
    Ok(())
}

/// Grab the preview region of the window (client-area coords in physical px) and
/// store it as the deployment's cached screenshot. Returns the data: URL on
/// success, or None if capture is unsupported / failed (caller shows a fallback).
#[tauri::command]
pub async fn capture_preview(
    project: String,
    id: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    if w <= 0 || h <= 0 { return Ok(None); }
    // Client-area origin in physical screen pixels (no HWND FFI needed).
    let pos = window.inner_position().map_err(|e| e.to_string())?;
    let rgba = capture_region(pos.x + x, pos.y + y, w, h);
    let Some(rgba) = rgba else { return Ok(None); };

    // Encode PNG
    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png_bytes, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
    }

    // Persist alongside the record + flip the flag.
    let dir = project_dir(&state.data_dir, &project);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{}.png", sanitize(&id))), &png_bytes).map_err(|e| e.to_string())?;
    let json_path = dir.join(format!("{}.json", sanitize(&id)));
    if let Ok(text) = std::fs::read_to_string(&json_path) {
        if let Ok(mut rec) = serde_json::from_str::<DeploymentRecord>(&text) {
            rec.has_screenshot = true;
            if let Ok(j) = serde_json::to_string_pretty(&rec) {
                let _ = std::fs::write(&json_path, j);
            }
        }
    }

    use base64::Engine;
    let url = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&png_bytes));
    Ok(Some(url))
}

/// Capture an (x,y,w,h) screen rectangle into top-down RGBA bytes via GDI.
#[cfg(target_os = "windows")]
fn capture_region(src_x: i32, src_y: i32, w: i32, h: i32) -> Option<Vec<u8>> {
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };
    unsafe {
        let screen_dc = GetDC(0);
        if screen_dc == 0 { return None; }
        let mem_dc = CreateCompatibleDC(screen_dc);
        let bmp = CreateCompatibleBitmap(screen_dc, w, h);
        let old = SelectObject(mem_dc, bmp);
        let blit = BitBlt(mem_dc, 0, 0, w, h, screen_dc, src_x, src_y, SRCCOPY);

        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = -h; // negative = top-down
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB as u32;
        let scan = GetDIBits(mem_dc, bmp, 0, h as u32, buf.as_mut_ptr() as *mut _, &mut bi, DIB_RGB_COLORS);

        SelectObject(mem_dc, old);
        DeleteObject(bmp);
        DeleteDC(mem_dc);
        ReleaseDC(0, screen_dc);

        if blit == 0 || scan == 0 { return None; }
        // GDI gives BGRA; convert to RGBA and force opaque.
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        Some(buf)
    }
}

#[cfg(not(target_os = "windows"))]
fn capture_region(_src_x: i32, _src_y: i32, _w: i32, _h: i32) -> Option<Vec<u8>> {
    None
}
