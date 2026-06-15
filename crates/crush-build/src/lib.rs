pub mod detect;
pub mod run;
pub mod mobile;
pub mod version;
pub mod env;
pub mod multiservice;
pub mod crushfile;
pub mod parser;
pub mod cache;
pub mod pipeline;
pub mod stages;
pub mod secrets;
pub mod ignore;
pub mod cross;
pub mod sbom;
pub mod attest;
pub mod progress;
pub mod analysis;
pub mod service_orchestrator;
pub mod proxy;

use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

pub use detect::{CrushSpecDetector, Detection, RuntimeType, SubService};
pub use parser::{CrushfileParser, Crushfile, CrushfileStage, CrushfileSecret};
pub use cache::BuildCache;
pub use pipeline::{BuildPipeline, PipelineResult};
pub use stages::MultiStageGraph;
pub use secrets::BuildSecrets;
pub use progress::BuildProgress;
pub use service_orchestrator::{
    ParsedCompose, DepService, AppServiceHints,
    BackendKind, ServiceState, RunningContainer,
    detect_backend, parse_compose,
    start_dep_service, stop_dep_service,
    rewrite_env_hostnames,
    save_service_state, load_service_state, clear_service_state,
    StartedService, start_dep_service_smart, native_driver_for,
    synthesize_dep_env, parse_spring_config,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredStack {
    pub language: String,
    pub runtime_version: String,
    pub build_command: String,
    #[serde(default)]
    pub dev_install_command: String,
    pub entry_point: String,
    #[serde(default)]
    pub dev_entry_point: String,
    pub default_port: u16,
    pub confidence: f32,
    pub base_image: String,
    #[serde(default)]
    pub is_monorepo: bool,
    #[serde(default)]
    pub services: Vec<crate::detect::SubService>,
    #[serde(default)]
    pub generic_subdir_hint: Vec<String>,
}

pub struct BuildOutcome {
    pub digest: String,
    pub was_cached: bool,
    pub size_bytes: u64,
    pub duration_ms: u64,
}

/// Returns a SHA-256 content fingerprint for the project based on sorted
/// `(relative_path, mtime_ns)` tuples. Skips the same patterns as `latest_mtime`.
pub fn project_fingerprint(root: &Path) -> Result<String> {
    fn is_skip(name: &str) -> bool {
        matches!(name,
            "node_modules" | ".next" | "target" | "dist" | "build" | ".turbo" |
            ".venv" | "venv" | "__pycache__" | ".git" | ".cache" | ".pnpm" |
            "vendor" | "deps" | "_build" | "out" | "bin" | "obj" | ".gradle" | ".mvn")
    }

    // Read `.crushignore` once — additive to the hardcoded skip list.
    // Format: one pattern per line, # for comments. Supports plain names
    // (matches any dir with that name) and trailing-slash form (dir only).
    // No glob/regex — keep parsing minimal and fast.
    let extra_skips: Vec<String> = fs::read_to_string(root.join(".crushignore"))
        .ok()
        .map(|content| content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.trim_end_matches('/').to_string())
            .collect())
        .unwrap_or_default();
    let is_user_skip = |name: &str| extra_skips.iter().any(|s| s == name);

    let mut entries: Vec<(String, u128)> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.starts_with('.') && name != ".env" {
                continue;
            }
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_dir() {
                if is_skip(&name) || is_user_skip(&name) {
                    continue;
                }
                stack.push(path);
            } else if ft.is_file() {
                if is_user_skip(&name) {
                    continue;
                }
                let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string();
                let mtime_ns = match entry.metadata().ok().and_then(|m| m.modified().ok()) {
                    Some(t) => t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos(),
                    None => 0,
                };
                entries.push((rel, mtime_ns));
            }
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    for (rel, mtime_ns) in &entries {
        hasher.update(rel.as_bytes());
        hasher.update(&mtime_ns.to_le_bytes());
    }

    Ok(hex::encode(hasher.finalize()))
}

impl From<Detection> for InferredStack {
    fn from(d: Detection) -> Self {
        Self {
            language: format!("{} ({})", d.runtime_type.as_str(), d.framework_name),
            runtime_version: d.runtime_version,
            build_command: d.build_command,
            dev_install_command: d.dev_install_command,
            entry_point: d.entry_point,
            dev_entry_point: d.dev_entry_point,
            default_port: d.port,
            confidence: d.confidence,
            base_image: d.base_image,
            is_monorepo: d.is_monorepo,
            services: d.services,
            generic_subdir_hint: d.generic_subdir_hint,
        }
    }
}

pub struct StackDetector;

impl StackDetector {
    pub fn new() -> Self { Self }
    pub async fn detect(&self, project_root: &PathBuf) -> Result<InferredStack> {
        let d = CrushSpecDetector::new().detect(project_root);
        Ok(InferredStack::from(d))
    }
}

pub struct BuildEngine {
    cache_dir: PathBuf,
}

impl BuildEngine {
    pub fn new(cache_dir: PathBuf) -> Self {
        fs::create_dir_all(&cache_dir).ok();
        Self {
            cache_dir,
        }
    }

    pub async fn execute_layered_build(&self, project_root: &PathBuf, stack: &InferredStack) -> Result<BuildOutcome> {
        use sha2::{Sha256, Digest};
        let t0 = std::time::Instant::now();

        let mut hasher = Sha256::new();
        let mut all_content = Vec::new();
        self.collect_files(project_root, &mut all_content).ok();
        for content in &all_content { hasher.update(content); }
        hasher.update(stack.language.as_bytes());
        hasher.update(stack.build_command.as_bytes());

        let digest = format!("sha256:{}", hex::encode(hasher.finalize()));
        let layer_file = self.layer_path(&digest);

        if layer_file.exists() {
            let size_bytes = fs::metadata(&layer_file).map(|m| m.len()).unwrap_or(0);
            return Ok(BuildOutcome {
                digest,
                was_cached: true,
                size_bytes,
                duration_ms: t0.elapsed().as_millis() as u64,
            });
        }

        fs::create_dir_all(layer_file.parent().unwrap())
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        let mut tar_builder = tar::Builder::new(Vec::new());
        self.add_files_to_tar(project_root, &mut tar_builder, &mut Vec::new()).ok();
        let tar_data = tar_builder.into_inner()
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        let compressed = {
            let mut encoder = zstd::Encoder::new(Vec::new(), 3)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            std::io::Write::write_all(&mut encoder, &tar_data)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            encoder.finish()
                .map_err(|e| CrushError::StorageError(e.to_string()))?
        };

        let size_bytes = compressed.len() as u64;
        fs::write(&layer_file, &compressed)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        Ok(BuildOutcome {
            digest,
            was_cached: false,
            size_bytes,
            duration_ms: t0.elapsed().as_millis() as u64,
        })
    }

    fn layer_path(&self, digest: &str) -> PathBuf {
        self.cache_dir().join("layers").join(digest.replace(':', "_"))
    }

    fn cache_dir(&self) -> PathBuf {
        self.cache_dir.clone()
    }

    fn collect_files(&self, dir: &PathBuf, result: &mut Vec<Vec<u8>>) -> Result<()> {
        if !dir.is_dir() { return Ok(()); }
        for entry in fs::read_dir(dir).map_err(|e| CrushError::StorageError(e.to_string()))? {
            let entry = entry.map_err(|e| CrushError::StorageError(e.to_string()))?;
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if matches!(name.as_str(),
                    "target" | "node_modules" | ".git" | ".venv" | "venv" | "env"
                    | "__pycache__" | ".mypy_cache" | ".pytest_cache" | ".ruff_cache"
                    | ".tox" | ".nox" | "dist" | "build" | ".next" | ".nuxt"
                    | ".cache" | ".idea" | ".vscode"
                ) { continue; }
                self.collect_files(&path, result)?;
            } else if path.is_file() {
                if let Ok(data) = fs::read(&path) { result.push(data); }
            }
        }
        Ok(())
    }

    fn add_files_to_tar(&self, dir: &PathBuf, tar: &mut tar::Builder<Vec<u8>>, prefix: &mut Vec<String>) -> Result<()> {
        if !dir.is_dir() { return Ok(()); }
        for entry in fs::read_dir(dir).map_err(|e| CrushError::StorageError(e.to_string()))? {
            let entry = entry.map_err(|e| CrushError::StorageError(e.to_string()))?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if matches!(name.as_str(),
                "target" | "node_modules" | ".git" | ".venv" | "venv" | "env"
                | "__pycache__" | ".mypy_cache" | ".pytest_cache" | ".ruff_cache"
                | ".tox" | ".nox" | "dist" | "build" | ".next" | ".nuxt"
                | ".cache" | ".idea" | ".vscode"
            ) { continue; }
            if path.is_dir() {
                prefix.push(name);
                self.add_files_to_tar(&path, tar, prefix)?;
                prefix.pop();
            } else if path.is_file() {
                let data = fs::read(&path).map_err(|e| CrushError::StorageError(e.to_string()))?;
                let full_name = prefix.iter().cloned().chain(std::iter::once(name)).collect::<Vec<_>>().join("/");
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                header.set_mtime(0);
                header.set_entry_type(tar::EntryType::Regular);
                tar.append_data(&mut header, &full_name, &data[..])
                    .map_err(|e| CrushError::StorageError(e.to_string()))?;
            }
        }
        Ok(())
    }
}
