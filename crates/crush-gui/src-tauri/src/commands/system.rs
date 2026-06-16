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
    let mut runtime_str = format!("{:?}", det.runtime_type);
    let mut fw = if det.framework_detected && !det.framework_name.is_empty() {
        Some(det.framework_name.clone())
    } else {
        None
    };
    if matches!(det.runtime_type, crush_build::detect::RuntimeType::Generic) && det.framework_name.contains(" · ") {
        runtime_str = det.framework_name.clone();
        fw = None;
    }
    Ok(ProjectInfo {
        name: det.project_name,
        runtime: runtime_str,
        version: det.runtime_version,
        framework: fw,
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

/// Find a project's *own* icon (favicon/logo) on disk and return it as a data URL.
/// This is what makes the dashboard + deployments rows show the real project brand
/// instead of falling back to the stack logo. Returns None if nothing suitable is
/// found, so the caller can render its stack-icon fallback.
#[tauri::command]
pub async fn find_project_icon(project_path: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || scan_project_icon(Path::new(&project_path)))
        .await
        .map_err(|e| e.to_string())
}

fn scan_project_icon(root: &Path) -> Option<String> {
    if !root.is_dir() {
        return None;
    }
    // Highest-fidelity brand assets first, generic favicons last.
    const CANDIDATES: &[&str] = &[
        "public/apple-touch-icon.png",
        "static/apple-touch-icon.png",
        "public/logo.svg",
        "static/logo.svg",
        "src/assets/logo.svg",
        "assets/logo.svg",
        "public/logo.png",
        "static/logo.png",
        "src/assets/logo.png",
        "assets/logo.png",
        "public/logo192.png",
        "public/icon.png",
        "static/icon.png",
        "public/icon.svg",
        "src-tauri/icons/128x128.png",
        "public/favicon.svg",
        "static/favicon.svg",
        "public/favicon.ico",
        "static/favicon.ico",
        "src/favicon.ico",
        "favicon.ico",
        "favicon.png",
    ];

    const MAX_BYTES: u64 = 1024 * 1024; // 1MB cap — icons are tiny; avoid loading art.

    for rel in CANDIDATES {
        let path = root.join(rel);
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_BYTES {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let mime = match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("ico") => "image/x-icon",
            _ => "application/octet-stream",
        };
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return Some(format!("data:{mime};base64,{b64}"));
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub ok: bool,
    pub status: u16,
    pub latency_ms: u64,
}

/// Probe a deployment URL to see if it's *actually live* right now (not just
/// "we recorded a deploy once"). A reachable response — even a 4xx — means the
/// host is up; only network failures/timeouts count as down.
#[tauri::command]
pub async fn probe_deployment(url: String) -> Result<ProbeResult, String> {
    if url.trim().is_empty() {
        return Ok(ProbeResult { ok: false, status: 0, latency_ms: 0 });
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .danger_accept_invalid_certs(true) // self-signed/edge certs shouldn't read as "down"
        .user_agent("crush-deploy-probe")
        .build()
        .map_err(|e| e.to_string())?;

    let started = std::time::Instant::now();
    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            Ok(ProbeResult {
                ok: status < 500,
                status,
                latency_ms: started.elapsed().as_millis() as u64,
            })
        }
        Err(_) => Ok(ProbeResult { ok: false, status: 0, latency_ms: started.elapsed().as_millis() as u64 }),
    }
}
