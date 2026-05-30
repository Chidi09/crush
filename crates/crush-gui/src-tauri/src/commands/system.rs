use serde::Serialize;
use std::path::Path;
use tauri::State;
use crate::state::AppState;
use crush_build::detect::CrushSpecDetector;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub runtime: String,
    pub version: String,
    pub framework: Option<String>,
    pub port: u16,
    pub confidence: f32,
    pub is_monorepo: bool,
    pub env_required: Vec<String>,
    pub service_count: usize,
    /// "turbo" | "fullstack" | "spa" | "backend" | null — drives the Run-button glow.
    pub stack_kind: Option<String>,
    pub external_services: Vec<crush_build::env::ExternalService>,
}

/// Heuristically detect the stack of a project directory — same detector the
/// `crush` run flow uses, so the dashboard shows what would actually run.
#[tauri::command]
pub async fn detect_project(path: String) -> Result<ProjectInfo, String> {
    let root = std::path::PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    let det = CrushSpecDetector::new().detect(&root);
    Ok(ProjectInfo {
        name: det.project_name,
        runtime: format!("{:?}", det.runtime_type),
        version: det.runtime_version,
        framework: if det.framework_detected && !det.framework_name.is_empty() {
            Some(det.framework_name)
        } else {
            None
        },
        port: det.port,
        confidence: det.confidence,
        is_monorepo: det.is_monorepo,
        env_required: det.env_required,
        service_count: det.services.len(),
        stack_kind: if det.stack_kind.is_empty() { None } else { Some(det.stack_kind) },
        external_services: det.external_services,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskSegment {
    pub label: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
    pub disk_used_bytes: u64,
    /// Per-top-level-subdir breakdown of the data dir, sorted largest-first,
    /// so the dashboard can render a segmented usage bar (images/services/…).
    pub disk_breakdown: Vec<DiskSegment>,
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            match e.metadata() {
                Ok(m) if m.is_dir() => total += dir_size(&e.path()),
                Ok(m) => total += m.len(),
                Err(_) => {}
            }
        }
    }
    total
}

/// Friendly label for a known data-dir subfolder; falls back to the raw name.
fn pretty_label(name: &str) -> String {
    match name {
        "images" => "Images".into(),
        "services" => "Services".into(),
        "builds" | "build" | "build-cache" => "Build cache".into(),
        "blobs" => "Layers".into(),
        "logs" => "Logs".into(),
        "tmp" | "temp" => "Temp".into(),
        other => other.to_string(),
    }
}

fn compute_breakdown(root: &Path) -> (u64, Vec<DiskSegment>) {
    let mut segs: Vec<DiskSegment> = Vec::new();
    let mut loose: u64 = 0; // size of files sitting directly in the data dir
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            match e.metadata() {
                Ok(m) if m.is_dir() => {
                    let bytes = dir_size(&e.path());
                    if bytes > 0 {
                        let name = e.file_name().to_string_lossy().to_string();
                        segs.push(DiskSegment { label: pretty_label(&name), bytes });
                    }
                }
                Ok(m) => loose += m.len(),
                Err(_) => {}
            }
        }
    }
    if loose > 0 {
        segs.push(DiskSegment { label: "Other".into(), bytes: loose });
    }
    segs.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    let total: u64 = segs.iter().map(|s| s.bytes).sum();
    (total, segs)
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceUsage {
    /// system-wide CPU utilisation, 0–100
    pub cpu_percent: f32,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
}

/// Live host CPU + memory usage (Task-Manager style) for the dashboard.
#[tauri::command]
pub async fn system_resources() -> Result<ResourceUsage, String> {
    tokio::task::spawn_blocking(|| {
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_memory();
        // CPU usage needs two samples spaced by the minimum interval.
        sys.refresh_cpu_usage();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_usage();
        ResourceUsage {
            cpu_percent: sys.global_cpu_usage(),
            mem_used_bytes: sys.used_memory(),
            mem_total_bytes: sys.total_memory(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn system_info(state: State<'_, AppState>) -> Result<SystemInfo, String> {
    let data_dir = state.data_dir.clone();
    let (disk, breakdown) = tokio::task::spawn_blocking({
        let d = data_dir.clone();
        move || compute_breakdown(&d)
    })
    .await
    .unwrap_or((0, Vec::new()));
    Ok(SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        data_dir: data_dir.to_string_lossy().to_string(),
        disk_used_bytes: disk,
        disk_breakdown: breakdown,
    })
}
