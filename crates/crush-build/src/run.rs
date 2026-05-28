//! Types shared between the CLI's `Commands::Default` flow and the future
//! GUI (`crush-gui/src-tauri`). The CLI currently inlines the run flow in
//! `main.rs`; v0.7.72+ extracts that body into a `run_project()` function
//! returning a `RunHandle` and emitting `RunEvent`s into a channel.
//!
//! Defined ahead of the refactor so the GUI can wire `tauri::Window::emit`
//! against a stable schema, and so the agent implementing the refactor has
//! a fixed target signature.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which stdio stream a captured line came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Per-step events emitted by `run_project()`. The CLI's existing
/// progress prints map 1:1 onto these — refactoring is mostly a matter
/// of replacing `println!` with `tx.send(...)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RunEvent {
    /// Stack detection finished. Mirrors the current "↳ detected: ..." line.
    Detected {
        language: String,
        framework: String,
        confidence: f32,
        is_monorepo: bool,
        port: u16,
    },

    /// A dependency service started (native postgres, garnet, etc).
    DepStarted { name: String, image: String, native: bool },

    /// A dependency service failed to start. Run continues — caller decides.
    DepFailed { name: String, error: String },

    /// Image pack was skipped because the fingerprint matched the cache.
    ImageFresh { digest: String },

    /// Image pack ran. Emitted at completion, not start.
    ImagePacked { digest: String, size_bytes: u64, duration_ms: u64 },

    /// Build step (e.g. `pnpm install`, `mvn compile`) about to run.
    BuildStarted { command: String },

    /// One captured stdio line from the build step.
    BuildOutput { line: String, stream: Stream },

    /// Build step finished. `success: false` aborts the run.
    BuildFinished { duration_ms: u64, success: bool },

    /// Entry process about to spawn (e.g. `pnpm run dev`).
    Spawning { command: String, port: u16 },

    /// One captured stdio line from the running app.
    AppOutput { line: String, stream: Stream },

    /// The app's listen port responded — the canonical "ready" signal.
    /// `urls` are the surfaced doc/health/graphql links the CLI prints
    /// under "↳ open:".
    PortBound {
        port: u16,
        startup_ms: u64,
        total_ms: u64,
        urls: Vec<(String, String)>,  // (label, url)
    },

    /// The app exited. `code` is the OS exit code (Windows: u32 truncated).
    Exited { code: i32 },

    /// A non-fatal warning the UI can surface (e.g. "no response on :3000
    /// after 10s — app may still be starting").
    Warning { message: String },
}

/// Options that change the run flow's behaviour. Maps onto the CLI's
/// boolean/option flags (`--dev`, `--rebuild`, `--repack`, `--no-proxy`,
/// `--memory`, `--cpus`, `--priority`, `--watch`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunOptions {
    #[serde(default)]
    pub dev: bool,
    #[serde(default)]
    pub rebuild: bool,
    #[serde(default)]
    pub repack: bool,
    #[serde(default)]
    pub no_proxy: bool,
    #[serde(default)]
    pub watch: bool,
    #[serde(default)]
    pub memory_bytes: Option<u64>,
    #[serde(default)]
    pub cpu_fraction: Option<f32>,
    /// Windows priority class hint: "high" or "above-normal". Ignored elsewhere.
    #[serde(default)]
    pub priority: Option<String>,
    /// Skip the Y/n prompt and assume yes (default for GUI; CLI keeps its
    /// own warm-run heuristic).
    #[serde(default)]
    pub assume_yes: bool,
}

/// One entry written to `data_dir/build-history.json` on every build outcome.
/// The GUI's Build screen reads this file — keep the schema stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRecord {
    /// Unix ms timestamp of when the build finished.
    pub timestamp_ms: u64,
    /// Absolute path to the project root.
    pub project_path: String,
    /// Just the directory name (e.g. "gazillion-be-staging").
    pub project_name: String,
    /// Stack language string from detection (e.g. "python (FastAPI)").
    pub language: String,
    /// Framework name alone, if any (e.g. "FastAPI", "Spring Boot").
    pub framework: String,
    /// Total time spent in the build/pack step.
    pub duration_ms: u64,
    /// True if we hit the warm-run cache and skipped the pack.
    pub was_cached: bool,
    /// Resulting image size in bytes (0 if cached).
    pub size_bytes: u64,
    /// OCI digest of the produced image.
    pub digest: String,
    /// False on build-step failure (pnpm install / mvn compile exited non-zero).
    pub success: bool,
}

/// File name (relative to `data_dir`) where the history is written.
pub const BUILD_HISTORY_FILE: &str = "build-history.json";

/// Cap on number of entries kept. Oldest are trimmed when exceeded.
pub const BUILD_HISTORY_MAX_ENTRIES: usize = 200;

/// Append a record to the build history file. Idempotent in the sense that
/// the file is created on first use and trimmed in-place. Errors are
/// returned but the caller should typically ignore them — history is a
/// nice-to-have, not a correctness signal.
pub fn append_build_record(data_dir: &std::path::Path, record: BuildRecord) -> anyhow::Result<()> {
    use std::fs;
    let path = data_dir.join(BUILD_HISTORY_FILE);
    let mut history: Vec<BuildRecord> = match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    history.push(record);
    // Keep newest BUILD_HISTORY_MAX_ENTRIES.
    if history.len() > BUILD_HISTORY_MAX_ENTRIES {
        let excess = history.len() - BUILD_HISTORY_MAX_ENTRIES;
        history.drain(0..excess);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&history)?)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Read the build history file. Empty vec on missing file or parse error
/// (since corruption shouldn't break the GUI).
pub fn read_build_history(data_dir: &std::path::Path) -> Vec<BuildRecord> {
    let path = data_dir.join(BUILD_HISTORY_FILE);
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// ── Stable channel/handle API for the future run_project() ─────────────────
//
// Intentionally NOT implementing this yet — the body lives in `crush-cli`
// today. The signature is fixed here so the agent doing the extraction
// has a target, and the GUI can be designed against it.
//
// pub struct RunHandle {
//     pub run_id: uuid::Uuid,
//     pub events: tokio::sync::mpsc::Receiver<RunEvent>,
//     pub abort: tokio::sync::oneshot::Sender<()>,
// }
//
// pub async fn run_project(
//     project_root: PathBuf,
//     data_dir: PathBuf,
//     options: RunOptions,
// ) -> anyhow::Result<RunHandle>;
//
// Where it lives: this file (`crush-build/src/run.rs`).
// What it replaces: crush-cli/src/main.rs:1488-2710 (the Commands::Default arm).
// Bounded channel cap: 1024 (see CRUSH_V8_PLAN.md "Back-pressure").

/// The path of the run flow function. Documented here so the next agent
/// has a single source of truth for where the refactor lands.
pub const _FUTURE_RUN_FN: &str = "crush_build::run::run_project";

#[allow(dead_code)]
fn _project_path_ergonomics(p: PathBuf) -> String {
    p.to_string_lossy().to_string()
}
