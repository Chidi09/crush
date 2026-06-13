//! Types shared between the CLI's `Commands::Default` flow and the future
//! GUI (`crush-gui/src-tauri`). The CLI currently inlines the run flow in
//! `main.rs`; v0.7.72+ extracts that body into a `run_project()` function
//! returning a `RunHandle` and emitting `RunEvent`s into a channel.
//!
//! Defined ahead of the refactor so the GUI can wire `tauri::Window::emit`
//! against a stable schema, and so the agent implementing the refactor has
//! a fixed target signature.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use crate::detect::SubService;

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
        /// Number of dep services started alongside.
        #[serde(default)]
        dep_count: usize,
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
    BuildStarted {
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_name: Option<String>,
    },

    /// One captured stdio line from the build step.
    BuildOutput {
        line: String,
        stream: Stream,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_name: Option<String>,
    },

    /// Build step finished. `success: false` aborts the run.
    BuildFinished {
        duration_ms: u64,
        success: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_name: Option<String>,
    },

    /// Entry process about to spawn (e.g. `pnpm run dev`).
    Spawning {
        command: String,
        port: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_name: Option<String>,
    },

    /// One captured stdio line from the running app.
    AppOutput {
        line: String,
        stream: Stream,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_name: Option<String>,
    },

    /// The app's listen port responded — the canonical "ready" signal.
    /// `urls` are the surfaced doc/health/graphql links the CLI prints
    /// under "↳ open:".
    PortBound {
        port: u16,
        startup_ms: u64,
        total_ms: u64,
        urls: Vec<(String, String)>, // (label, url)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service_name: Option<String>,
    },

    /// The app exited. `code` is the OS exit code (Windows: u32 truncated).
    Exited { code: i32 },

    /// A non-fatal warning the UI can surface (e.g. "no response on :3000
    /// after 10s — app may still be starting").
    Warning { message: String },

    /// Warm-run marker: the image was fresh AND deps are current, so the
    /// CLI auto-proceeds past the prompt. CLI prints `↳ warm run — launching`.
    WarmRun,

    /// The node_modules / .venv / vendor / deps directory is newer than the
    /// project's lockfile, so the install step was skipped. CLI prints
    /// `✓ dependencies fresh — node_modules newer than lockfile (--rebuild to force)`.
    DepsFresh,
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

/// Handle to an in-progress run, returned by `run_project()`. The caller
/// reads `events` to stream the run output and drops or sends on `abort`
/// to stop the run.
pub struct RunHandle {
    pub run_id: uuid::Uuid,
    pub events: tokio::sync::mpsc::Receiver<RunEvent>,
    pub abort: tokio::sync::oneshot::Sender<()>,
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

// ── Windows Job Object helpers ──────────────────────────────────────────
// Lazy-init a kill-on-close job on first `assign_to_job` call.
// Uses the same patterns as the CLI's own job_object module.

#[cfg(target_os = "windows")]
mod job_imp {
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
        JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    struct Job(windows_sys::Win32::Foundation::HANDLE);
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}
    impl Drop for Job {
        fn drop(&mut self) { unsafe { CloseHandle(self.0); } }
    }

    static JOB: OnceLock<Option<Job>> = OnceLock::new();

    fn create() -> Option<Job> {
        unsafe {
            let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if h == 0 { return None; }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 { CloseHandle(h); return None; }
            Some(Job(h))
        }
    }

    pub fn get() -> Option<windows_sys::Win32::Foundation::HANDLE> {
        JOB.get_or_init(|| create()).as_ref().map(|j| j.0)
    }
}

#[cfg(target_os = "windows")]
pub fn assign_to_job(child: &tokio::process::Child) {
    use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
    if let Some(raw) = child.raw_handle() {
        if let Some(job) = job_imp::get() {
            unsafe {
                let _ = AssignProcessToJobObject(job, raw as windows_sys::Win32::Foundation::HANDLE);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn assign_to_job(_child: &tokio::process::Child) {}

/// Kill a child process **and all its descendants**. A dev launcher
/// (`npm run dev` → `node`/`vite`) spawns grandchildren that `Child::kill()`
/// leaves orphaned still holding the port, so on Windows tear down the whole
/// tree with `taskkill /T`.
#[cfg(target_os = "windows")]
async fn kill_tree(child: &mut tokio::process::Child) {
    if let Some(pid) = child.id() {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .await;
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[cfg(not(target_os = "windows"))]
async fn kill_tree(child: &mut tokio::process::Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

// ── Process spawning ────────────────────────────────────────────────────

/// Translate bash-style `$VAR` and `${VAR}` to cmd.exe-style `%VAR%`.
pub fn translate_env_refs_for_cmd(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' { out.push(c); continue; }
        match chars.peek() {
            Some(&'{') => {
                chars.next();
                let mut name = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == '}' { chars.next(); break; }
                    if nc.is_ascii_alphanumeric() || nc == '_' { name.push(nc); chars.next(); }
                    else { break; }
                }
                if !name.is_empty() {
                    out.push_str(&format!("%{}%", name));
                } else {
                    out.push('$'); out.push('{'); out.push_str(&name); out.push('}');
                }
            }
            Some(&nc) if nc.is_ascii_alphabetic() || nc == '_' => {
                let mut name = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' { name.push(nc); chars.next(); }
                    else { break; }
                }
                out.push_str(&format!("%{}%", name));
            }
            _ => out.push('$'),
        }
    }
    out
}

/// Spawns a command line through the OS shell so PATH lookups resolve `.cmd`,
/// `.bat`, and `.ps1` shims. On Unix, executes directly via the program parts.
pub fn spawn_shell(cmdline: &str, cwd: &Path, env: &[(String, String)]) -> tokio::process::Command {
    let cmdline = if cmdline.starts_with("java ") {
        if let Ok(jh) = std::env::var("JAVA_HOME") {
            let bin = if cfg!(target_os = "windows") {
                format!("{}\\bin\\java.exe", jh.trim_end_matches(['\\', '/']))
            } else {
                format!("{}/bin/java", jh.trim_end_matches('/'))
            };
            if std::path::Path::new(&bin).exists() {
                let prefix = if bin.contains(' ') {
                    format!("\"{}\"", bin)
                } else {
                    bin
                };
                format!("{} {}", prefix, &cmdline[5..])
            } else {
                cmdline.to_string()
            }
        } else {
            cmdline.to_string()
        }
    } else {
        cmdline.to_string()
    };
    let cmdline = cmdline.as_str();

    let cmdline = if cmdline.contains("target/*.jar") {
        let target = cwd.join("target");
        let jar = std::fs::read_dir(&target).ok().and_then(|entries| {
            entries.flatten()
                .filter_map(|e| {
                    let p = e.path();
                    let name = p.file_name()?.to_str()?.to_string();
                    if name.ends_with(".jar") && !name.ends_with(".jar.original") {
                        Some(name)
                    } else { None }
                })
                .next()
        });
        if let Some(jar) = jar {
            cmdline.replace("target/*.jar", &format!("target/{}", jar))
        } else {
            cmdline.to_string()
        }
    } else {
        cmdline.to_string()
    };
    let cmdline = cmdline.as_str();

    let mut cmd = if cfg!(target_os = "windows") {
        let fixed = if cmdline.starts_with("./") {
            format!(".\\{}", &cmdline[2..])
        } else {
            cmdline.to_string()
        };
        let fixed = translate_env_refs_for_cmd(&fixed);
        let mut c = tokio::process::Command::new("cmd");
        c.arg("/c").arg(fixed);
        c
    } else {
        let parts: Vec<&str> = cmdline.split_whitespace().collect();
        let mut c = tokio::process::Command::new(parts[0]);
        c.args(&parts[1..]);
        c
    };
    cmd.current_dir(cwd);
    for (k, v) in env { cmd.env(k, v); }
    // Suppress the cmd.exe console window that would otherwise flash on Windows.
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

// ── Helper functions extracted from main.rs ─────────────────────────────

pub type UrlSink = std::sync::Arc<tokio::sync::Mutex<Vec<String>>>;

/// Strip ANSI escape sequences from a string.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // consume until 'm' (SGR) or ']' (OSC)
            while let Some(n) = chars.next() {
                if n == 'm' || n == '\x07' || n == '\\' { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Record any URLs discovered in a line of output (for the "↳ open:" panel).
pub async fn record_urls(line: &str, sink: &UrlSink) {
    let clean = strip_ansi(line);
    let lower = clean.to_lowercase();
    let mut start = 0usize;
    while let Some(idx) = lower[start..].find("http") {
        let abs = start + idx;
        let rest = &clean[abs..];
        let scheme_len = if rest.starts_with("https://") { 8 }
                         else if rest.starts_with("http://") { 7 }
                         else { start = abs + 4; continue; };
        let after = &rest[scheme_len..];
        let host_ok = ["localhost", "127.0.0.1", "0.0.0.0", "[::]", "[::1]"]
            .iter().any(|h| after.starts_with(h));
        if !host_ok { start = abs + scheme_len; continue; }
        let end = rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '`' || c == ',' || c == ';')
            .unwrap_or(rest.len());
        let mut url = rest[..end].to_string();
        while matches!(url.chars().last(), Some('.') | Some(')') | Some(']')) { url.pop(); }
        let mut s = sink.lock().await;
        if !s.iter().any(|u| u == &url) {
            s.push(url);
        }
        start = abs + end;
    }
}

/// Check whether a path's build artifacts are up-to-date compared to sources.
pub fn build_freshness(root: &Path, language: &str) -> Option<String> {
    let lang = language.split(' ').next().unwrap_or("").to_lowercase();
    match lang.as_str() {
        "node" | "typescript" | "bun" | "deno" => {
            let lock = root.join("pnpm-lock.yaml");
            let nm = root.join("node_modules");
            if !nm.exists() { return None; }
            let lock_time = std::fs::metadata(&lock).and_then(|m| m.modified()).ok()?;
            let nm_time = std::fs::metadata(&nm).and_then(|m| m.modified()).ok()?;
            if nm_time >= lock_time { Some("node_modules newer than lockfile".into()) } else { None }
        }
        "python" => {
            let lock = if root.join("uv.lock").exists() { root.join("uv.lock") }
                       else { return None };
            let venv = root.join(".venv");
            if !venv.exists() { return None; }
            let lock_time = std::fs::metadata(&lock).and_then(|m| m.modified()).ok()?;
            let venv_time = std::fs::metadata(&venv).and_then(|m| m.modified()).ok()?;
            if venv_time >= lock_time { Some(".venv newer than lockfile".into()) } else { None }
        }
        "rust" => {
            let target = root.join("target");
            if !target.exists() { return None; }
            let src = root.join("src");
            let always = |_: &Path| true;
            let latest_src = latest_mtime_any(&src, &always);
            let target_time = std::fs::metadata(&target).and_then(|m| m.modified()).ok()?;
            if let Some(src_time) = latest_src {
                if target_time >= src_time { Some("target up-to-date".into()) } else { None }
            } else {
                Some("target exists".into())
            }
        }
        "go" => {
            let bin = root.join("target");
            if !bin.exists() { return None; }
            let go_pred = |p: &Path| p.extension().map_or(false, |e| e == "go");
            let latest_src = latest_mtime_any(root, &go_pred);
            let bin_time = std::fs::metadata(&bin).and_then(|m| m.modified()).ok()?;
            if let Some(src_time) = latest_src {
                if bin_time >= src_time { Some("binary up-to-date".into()) } else { None }
            } else {
                None
            }
        }
        "java" => {
            let target = root.join("target");
            if !target.exists() { return None; }
            let java_pred = |p: &Path| p.extension().map_or(false, |e| e == "java" || e == "kt" || e == "kts");
            let latest_src = latest_mtime_any(&root.join("src"), &java_pred);
            let target_time = std::fs::metadata(&target).and_then(|m| m.modified()).ok()?;
            if let Some(src_time) = latest_src {
                if target_time >= src_time { Some("target up-to-date".into()) } else { None }
            } else {
                Some("target exists".into())
            }
        }
        _ => None,
    }
}

fn latest_mtime_any(root: &Path, pred: &dyn Fn(&Path) -> bool) -> Option<std::time::SystemTime> {
    let mut latest: Option<std::time::SystemTime> = None;
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };
                if matches!(name, "node_modules" | ".next" | "target" | "dist" | "build" | ".turbo"
                    | ".venv" | "venv" | "__pycache__" | ".git" | ".cache" | "vendor" | "deps"
                    | "_build" | "out" | "bin" | "obj" | ".gradle" | ".mvn") { continue; }
                if let Some(sub) = latest_mtime_any(&path, pred) {
                    if latest.map_or(true, |l| sub > l) { latest = Some(sub); }
                }
            } else if pred(&path) {
                if let Ok(m) = std::fs::metadata(&path).and_then(|m| m.modified()) {
                    if latest.map_or(true, |l| m > l) { latest = Some(m); }
                }
            }
        }
    }
    latest
}

/// Check whether node_modules exists and is newer than the lockfile.
pub fn node_deps_fresh(root: &Path) -> bool {
    let nm = root.join("node_modules");
    if !nm.exists() { return false; }
    let nm_mtime = match std::fs::metadata(&nm).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    for lock_name in &["pnpm-lock.yaml", "yarn.lock", "package-lock.json"] {
        let lock = root.join(lock_name);
        if let Ok(lock_t) = std::fs::metadata(&lock).and_then(|m| m.modified()) {
            if nm_mtime >= lock_t { return true; }
        }
    }
    false
}

/// Probe well-known doc/health URLs on a port. Framework-narrowed to
/// reduce log noise.
pub async fn probe_service_links(port: u16) -> Vec<(String, String)> {
    probe_service_links_for(port, "").await
}

/// Framework-aware URL probing. Returns (label, url) pairs for each
/// responding endpoint.
pub async fn probe_service_links_for(port: u16, framework: &str) -> Vec<(String, String)> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(700))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let fw = framework.to_lowercase();
    let all: &[(&str, &str)] = &[
        ("/swagger-ui/index.html", "docs"),
        ("/swagger-ui.html", "docs"),
        ("/swagger/index.html", "docs"),
        ("/swagger", "docs"),
        ("/docs", "docs"),
        ("/api-docs", "docs"),
        ("/v3/api-docs", "openapi"),
        ("/openapi.json", "openapi"),
        ("/actuator/health", "health"),
        ("/health", "health"),
        ("/healthz", "health"),
        ("/graphql", "graphql"),
    ];
    let spring: &[(&str, &str)] = &[
        ("/swagger-ui/index.html", "docs"),
    ];
    let fastapi: &[(&str, &str)] = &[
        ("/docs", "docs"),
        ("/openapi.json", "openapi"),
    ];
    let express: &[(&str, &str)] = &[
        ("/api-docs", "docs"),
        ("/health", "health"),
    ];
    let nest: &[(&str, &str)] = &[
        ("/api", "api"),
        ("/docs", "docs"),
        ("/health", "health"),
    ];
    let fastify: &[(&str, &str)] = &[
        ("/docs", "docs"),
    ];

    let candidates: &[(&str, &str)] = if fw.contains("spring") || fw == "java" { spring }
        else if fw.contains("fastapi") || fw.contains("flask") { fastapi }
        else if fw.contains("express") { express }
        else if fw.contains("nest") { nest }
        else if fw.contains("fastify") { fastify }
        else { all };

    let base = format!("http://127.0.0.1:{}", port);
    let url = |path: &str| format!("{}{}", base, path);

    let mut results = Vec::new();
    for (path, label) in candidates {
        if let Ok(resp) = client.get(&url(path)).send().await {
            if resp.status().is_success() {
                results.push((label.to_string(), url(path)));
            }
        }
    }
    results
}

// ── Multi-service sub-build ─────────────────────────────────────────────

/// Run a single sub-service build command and stream its output.
pub async fn run_sub_build(
    sub: &crate::detect::SubService,
    cmdline: &str,
    cwd: &Path,
    dep_env: &[(String, String)],
    tx: &tokio::sync::mpsc::Sender<RunEvent>,
) -> anyhow::Result<()> {
    let t0 = std::time::Instant::now();
    let _ = tx.send(RunEvent::BuildStarted {
        command: cmdline.to_string(),
        service_name: Some(sub.name.clone()),
    }).await;

    let mut cmd = spawn_shell(cmdline, cwd, dep_env);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| anyhow::anyhow!("spawn build for {} failed: {}", sub.name, e))?;

    assign_to_job(&child);

    if let Some(stdout) = child.stdout.take() {
        let n = sub.name.clone();
        let tx_c = tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx_c.send(RunEvent::BuildOutput {
                    line,
                    stream: Stream::Stdout,
                    service_name: Some(n.clone()),
                }).await;
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let n = sub.name.clone();
        let tx_c = tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx_c.send(RunEvent::BuildOutput {
                    line,
                    stream: Stream::Stderr,
                    service_name: Some(n.clone()),
                }).await;
            }
        });
    }

    let status = child.wait().await
        .map_err(|e| anyhow::anyhow!("build for {} wait failed: {}", sub.name, e))?;
    let dur_ms = t0.elapsed().as_millis() as u64;
    let ok = status.success();
    let _ = tx.send(RunEvent::BuildFinished {
        duration_ms: dur_ms,
        success: ok,
        service_name: Some(sub.name.clone()),
    }).await;

    if ok { Ok(()) } else { anyhow::bail!("Build failed for {}", sub.name) }
}

// ── The refactored run flow ─────────────────────────────────────────────

/// Start the project run flow (compose → detect → build → spawn) in a
/// background task. Returns a `RunHandle` the caller can use to stream
/// events and abort.
pub async fn run_project(
    project_root: PathBuf,
    data_dir: PathBuf,
    options: RunOptions,
) -> anyhow::Result<RunHandle> {
    let (tx, rx) = tokio::sync::mpsc::channel::<RunEvent>(1024);
    let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel::<()>();

    let run_id = uuid::Uuid::new_v4();
    let handle = RunHandle { run_id, events: rx, abort: abort_tx };

    tokio::spawn(async move {
        if let Err(e) = run_project_inner(&project_root, &data_dir, &options, &tx, &mut abort_rx).await {
            let msg = e.to_string();
            if !msg.is_empty() {
                let _ = tx.send(RunEvent::Warning { message: msg }).await;
            }
        }
    });

    Ok(handle)
}

#[allow(clippy::too_many_lines)]
async fn run_project_inner(
    project_root: &Path,
    data_dir: &Path,
    options: &RunOptions,
    tx: &tokio::sync::mpsc::Sender<RunEvent>,
    abort_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let overall_start = std::time::Instant::now();
    let project_name = project_root.file_name()
        .map(|n| n.to_string_lossy().to_lowercase().replace([' ', '-'], "_"))
        .unwrap_or_else(|| "app".into());

    use crate::{StackDetector, BuildEngine, service_orchestrator::*};

    // ── 1. Compose: start dep services, extract app hints ────────────────
    let compose_files = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
    let compose_dirs = [".", "infra", "docker", ".docker", "deploy", "ops", "devops"];
    let mut compose_path: Option<PathBuf> = None;
    for d in &compose_dirs {
        for f in &compose_files {
            let candidate = project_root.join(d).join(f);
            if candidate.exists() {
                compose_path = Some(candidate);
                break;
            }
        }
        if compose_path.is_some() { break; }
    }
    if std::env::var("CRUSH_DEBUG_COMPOSE").is_ok() {
        eprintln!("[debug] project_root = {}", project_root.display());
        eprintln!("[debug] compose_path = {:?}", compose_path);
    }

    let mut dep_env: Vec<(String, String)> = Vec::new();
    let mut dep_service_names: Vec<String> = Vec::new();
    let mut app_command_override: Option<String> = None;
    let mut port_override: Option<u16> = None;

    if let Some(ref cp) = compose_path {
        match parse_compose(cp) {
            Ok(parsed) => {
                if !parsed.dep_services.is_empty() {
                    let backend = detect_backend();
                    let state_dir = data_dir.join("services");

                    // Fire-and-forget Garnet prefetch alongside compose startup
                    let prefetch_dir = data_dir.to_path_buf();
                    tokio::spawn(async move {
                        let _ = crush_services::prefetch(prefetch_dir.join("cache").join("binaries")).await;
                    });

                    let dep_futures: Vec<_> = parsed.dep_services.iter()
                        .map(|dep| {
                            let dep = dep.clone();
                            let pname = project_name.clone();
                            let dd = data_dir.to_path_buf();
                            async move {
                                let res = start_dep_service_smart(&dep, &pname, &dd).await;
                                (dep, res)
                            }
                        })
                        .collect();
                    let dep_results = futures::future::join_all(dep_futures).await;

                    let mut running_containers = Vec::new();
                    let mut running_natives = Vec::new();

                    for (dep, result) in dep_results {
                        match result {
                            Ok(StartedService::Native(running)) => {
                                let native = running.kind != crush_services::ServiceKind::Postgres
                                    && cfg!(target_os = "windows");
                                let _ = tx.send(RunEvent::DepStarted {
                                    name: dep.name.clone(),
                                    image: dep.image.clone(),
                                    native,
                                }).await;
                                dep_service_names.push(dep.name.clone());
                                dep_env.extend(synthesize_dep_env(&dep));
                                running_natives.push(running);
                            }
                            Ok(StartedService::Container(cname)) => {
                                let _ = tx.send(RunEvent::DepStarted {
                                    name: dep.name.clone(),
                                    image: dep.image.clone(),
                                    native: false,
                                }).await;
                                dep_service_names.push(dep.name.clone());
                                dep_env.extend(synthesize_dep_env(&dep));
                                running_containers.push(RunningContainer {
                                    service_name: dep.name.clone(),
                                    container_name: cname,
                                    ports: dep.ports.clone(),
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(RunEvent::DepFailed {
                                    name: dep.name.clone(),
                                    error: e.to_string(),
                                }).await;
                            }
                        }
                    }

                    if !running_containers.is_empty() {
                        let state = ServiceState {
                            project: project_name.clone(),
                            backend: backend.as_str().to_string(),
                            containers: running_containers,
                            started_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_secs(),
                        };
                        let _ = save_service_state(&state_dir, &state);
                    }

                    if !running_natives.is_empty() {
                        let _ = crush_services::save_native_state(&state_dir, &crush_services::NativeServiceState {
                            project: project_name.clone(),
                            services: running_natives,
                            started_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_secs(),
                        });
                    }
                }
                if let Some(hints) = parsed.app_hints {
                    let rewritten = rewrite_env_hostnames(&hints.env, &dep_service_names);
                    for (k, v) in rewritten {
                        if let Some(slot) = dep_env.iter_mut().find(|(ek, _)| ek == &k) {
                            slot.1 = v;
                        } else {
                            dep_env.push((k, v));
                        }
                    }
                    app_command_override = hints.command;
                    port_override = hints.port;
                }
            }
            Err(e) => {
                let _ = tx.send(RunEvent::Warning { message: format!("compose parse warning: {} — proceeding with stack detection", e) }).await;
            }
        }
    }

    // ── 2b. Spring Boot fallback ──
    if dep_service_names.is_empty() {
        let spring_deps = parse_spring_config(project_root);
        if !spring_deps.is_empty() {
            let state_dir = data_dir.join("services");
            let mut running_natives = Vec::new();
            let mut running_containers = Vec::new();
            let backend = detect_backend();

            let spring_futures: Vec<_> = spring_deps.iter()
                .map(|dep| {
                    let dep = dep.clone();
                    let pname = project_name.clone();
                    let dd = data_dir.to_path_buf();
                    async move {
                        let res = start_dep_service_smart(&dep, &pname, &dd).await;
                        (dep, res)
                    }
                })
                .collect();
            let spring_results = futures::future::join_all(spring_futures).await;

            for (dep, result) in spring_results {
                match result {
                    Ok(StartedService::Native(running)) => {
                        let _ = tx.send(RunEvent::DepStarted {
                            name: dep.name.clone(),
                            image: dep.image.clone(),
                            native: true,
                        }).await;
                        dep_service_names.push(dep.name.clone());
                        dep_env.extend(synthesize_dep_env(&dep));
                        running_natives.push(running);
                    }
                    Ok(StartedService::Container(cname)) => {
                        let _ = tx.send(RunEvent::DepStarted {
                            name: dep.name.clone(),
                            image: dep.image.clone(),
                            native: false,
                        }).await;
                        dep_service_names.push(dep.name.clone());
                        dep_env.extend(synthesize_dep_env(&dep));
                        running_containers.push(RunningContainer {
                            service_name: dep.name.clone(),
                            container_name: cname,
                            ports: dep.ports.clone(),
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(RunEvent::DepFailed {
                            name: dep.name.clone(),
                            error: e.to_string(),
                        }).await;
                    }
                }
            }
            if !running_natives.is_empty() {
                let _ = crush_services::save_native_state(&state_dir, &crush_services::NativeServiceState {
                    project: project_name.clone(),
                    services: running_natives,
                    started_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                });
            }
            if !running_containers.is_empty() {
                let _ = save_service_state(&state_dir, &ServiceState {
                    project: project_name.clone(),
                    backend: backend.as_str().to_string(),
                    containers: running_containers,
                    started_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                });
            }
        }
    }

    // ── 3. Stack Detection ────────────────
    let detector = StackDetector::new();
    let project_root_buf = project_root.to_path_buf();
    let stack = detector.detect(&project_root_buf).await?;

    if stack.language.starts_with("generic")
        && !project_root.join("entrypoint.sh").exists()
        && !stack.generic_subdir_hint.is_empty()
    {
        anyhow::bail!(
            "Couldn't detect a project at {}\n   ↳ found project-looking subdirectories:\n       {}",
            project_root.display(),
            stack.generic_subdir_hint.iter().map(|s| format!("cd {} && crush", s)).collect::<Vec<_>>().join("\n       ")
        );
    }


    let root_is_generic = stack.language.starts_with("generic")
        || stack.entry_point == "entrypoint.sh"
        || stack.entry_point.is_empty();
    let is_multi_service = stack.is_monorepo
        && stack.services.len() >= 2
        && root_is_generic;

    let _ = tx.send(RunEvent::Detected {
        language: stack.language.clone(),
        framework: stack.language.split('(').nth(1).map(|s| s.trim_end_matches(')').to_string()).unwrap_or_default(),
        confidence: stack.confidence,
        is_monorepo: is_multi_service,
        port: stack.default_port,
        dep_count: dep_service_names.len(),
    }).await;

    // ── 4. Build ──────────────────────────────────────────────────────────
    let cache_dir = data_dir.join("cache");
    let engine = BuildEngine::new(cache_dir.clone());

    let project_hash = crate::project_fingerprint(project_root)?;
    let hash_path = cache_dir.join("last-image").join(format!("{project_name}.hash"));
    let prev_hash = std::fs::read_to_string(&hash_path).ok();

    let build_start = std::time::Instant::now();
    let outcome = if prev_hash.as_deref() == Some(&project_hash) && !options.repack {
        let _ = tx.send(RunEvent::ImageFresh { digest: project_hash.clone() }).await;
        crate::BuildOutcome {
            was_cached: true,
            digest: project_hash.clone(),
            size_bytes: 0,
            duration_ms: 0,
        }
    } else {
        let o = engine.execute_layered_build(&project_root_buf, &stack).await?;
        let _ = std::fs::create_dir_all(hash_path.parent().unwrap());
        let _ = std::fs::write(&hash_path, &project_hash);
        o
    };
    let build_elapsed = build_start.elapsed();

    if !outcome.was_cached {
        let _ = tx.send(RunEvent::ImagePacked {
            digest: outcome.digest.clone(),
            size_bytes: outcome.size_bytes,
            duration_ms: build_elapsed.as_millis() as u64,
        }).await;
    }

    // Append build history
    let _ = append_build_record(data_dir, BuildRecord {
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        project_path: project_root.to_string_lossy().to_string(),
        project_name: project_name.clone(),
        language: stack.language.clone(),
        framework: stack.language
            .split('(')
            .nth(1)
            .map(|s| s.trim_end_matches(')').to_string())
            .unwrap_or_default(),
        duration_ms: build_elapsed.as_millis() as u64,
        was_cached: outcome.was_cached,
        size_bytes: outcome.size_bytes,
        digest: outcome.digest.clone(),
        success: true,
    });

    // ── 5. Prompt decision ────────────────────────────────────────────
    let lang_for_prompt = stack.language.split(' ').next().unwrap_or("").to_lowercase();
    let is_node_like = matches!(lang_for_prompt.as_str(), "node" | "typescript" | "bun" | "deno");
    let warm_run = outcome.was_cached
        && (!is_node_like || node_deps_fresh(project_root));

    let should_run = if options.assume_yes {
        true
    } else if warm_run {
        let _ = tx.send(RunEvent::WarmRun).await;
        true
    } else {
        let answer = tokio::task::spawn_blocking(|| {
            use std::io::Write;
            print!("   run it now? [Y/n] ");
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            let t = input.trim().to_lowercase();
            t.is_empty() || t == "y" || t == "yes"
        }).await.unwrap_or(false);
        answer
    };

    if !should_run {
        return Ok(());
    }

    // ── 6. Multi-service branch ─────────────────────────────────────
    if is_multi_service {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let url_sink: UrlSink = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        // Phase A: parallel builds
        let sem = Arc::new(Semaphore::new(
            std::thread::available_parallelism().map(|p| p.get().min(4)).unwrap_or(2)
        ));
        let mut build_handles = Vec::new();
        for sub in &stack.services {
            let sub_path = PathBuf::from(&sub.path);
            let build_cmd = if options.dev {
                if sub.dev_install_command.is_empty() {
                    None
                } else {
                    let needs_install = match sub.runtime_type.as_str() {
                        "node" => !sub_path.join("node_modules").exists(),
                        "python" => !sub_path.join(".venv").exists(),
                        "php" => !sub_path.join("vendor").exists(),
                        "elixir" => !sub_path.join("deps").exists(),
                        _ => true,
                    };
                    if needs_install { Some(sub.dev_install_command.clone()) } else { None }
                }
            } else {
                if sub.build_command.is_empty() {
                    None
                } else if !options.rebuild {
                    if let Some(_reason) = build_freshness(&sub_path, &sub.runtime_type) {
                        None
                    } else {
                        Some(sub.build_command.clone())
                    }
                } else {
                    Some(sub.build_command.clone())
                }
            };

            if let Some(icmd) = build_cmd.clone() {
                let sem = sem.clone();
                let sub = sub.clone();
                let sub_path = sub_path.clone();
                let dep_env = dep_env.clone();
                let tx_c = tx.clone();
                build_handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.ok();
                    run_sub_build(&sub, &icmd, &sub_path, &dep_env, &tx_c).await
                }));
            }
        }

        if !build_handles.is_empty() {
            let results = futures::future::join_all(build_handles).await;
            if results.iter().any(|r| matches!(r, Ok(Err(_)) | Err(_))) {
                anyhow::bail!("one or more sub-service builds failed");
            }
        }

        // Phase B: start services in dependency order
        let sorted_services = topological_sort(&stack.services);
        let mut children: Vec<(String, u16, tokio::process::Child)> = Vec::new();
        let mut ready: Vec<(String, u16)> = Vec::new();

        for sub in &sorted_services {
            let sub_path = PathBuf::from(&sub.path);
            let run = if options.dev { sub.dev_entry_point.clone() } else { sub.entry_point.clone() };
            let run = run.replace("$PORT", &sub.port.to_string());

            let mut sub_env = dep_env.clone();
            
            // Read .env files of this sub-service and inject env vars, replacing any port conflicts
            for env_file in &[".env", ".env.local", ".env.example", ".env.development"] {
                let p = sub_path.join(env_file);
                if p.exists() {
                    if let Ok(dotenv_content) = fs::read_to_string(&p) {
                        for line in dotenv_content.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with('#') { continue; }
                            if let Some((k, v)) = line.split_once('=') {
                                let key = k.trim().to_string();
                                let mut val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                                
                                for other in &stack.services {
                                    if other.name == sub.name { continue; }
                                    let port_patterns = [
                                        format!("localhost:{}", other.original_port),
                                        format!("127.0.0.1:{}", other.original_port),
                                        format!("{}:{}", other.name, other.original_port),
                                    ];
                                    for pat in &port_patterns {
                                        if val.contains(pat) {
                                            val = val.replace(pat, &format!("localhost:{}", other.port));
                                        }
                                    }
                                    if val == other.name {
                                        val = format!("localhost:{}", other.port);
                                    }
                                }
                                sub_env.push((key, val));
                            }
                        }
                    }
                }
            }

            let _ = tx.send(RunEvent::Spawning {
                command: run.clone(),
                port: sub.port,
                service_name: Some(sub.name.clone()),
            }).await;

            let mut cmd = spawn_shell(&run, &sub_path, &sub_env);
            cmd.env("PORT", sub.port.to_string());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            match cmd.spawn() {
                Ok(mut child) => {
                    assign_to_job(&child);
                    if let Some(stdout) = child.stdout.take() {
                        let n = sub.name.clone();
                        let sink = url_sink.clone();
                        let tx_c = tx.clone();
                        tokio::spawn(async move {
                            use tokio::io::AsyncBufReadExt;
                            let reader = tokio::io::BufReader::new(stdout);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                record_urls(&line, &sink).await;
                                let _ = tx_c.send(RunEvent::AppOutput {
                                    line,
                                    stream: Stream::Stdout,
                                    service_name: Some(n.clone()),
                                }).await;
                            }
                        });
                    }
                    if let Some(stderr) = child.stderr.take() {
                        let n = sub.name.clone();
                        let sink = url_sink.clone();
                        let tx_c = tx.clone();
                        tokio::spawn(async move {
                            use tokio::io::AsyncBufReadExt;
                            let reader = tokio::io::BufReader::new(stderr);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                record_urls(&line, &sink).await;
                                let _ = tx_c.send(RunEvent::AppOutput {
                                    line,
                                    stream: Stream::Stderr,
                                    service_name: Some(n.clone()),
                                }).await;
                            }
                        });
                    }
                    
                    if wait_for_port(sub.port, 15).await {
                        ready.push((sub.name.clone(), sub.port));
                    }
                    
                    children.push((sub.name.clone(), sub.port, child));
                }
                Err(e) => {
                    let _ = tx.send(RunEvent::Warning {
                        message: format!("{}: spawn failed: {}", sub.name, e),
                    }).await;
                }
            }
            if abort_rx.try_recv().is_ok() { break; }
        }

        // Probe each ready service for URLs
        let mut probed: Vec<(String, u16, Vec<(String, String)>)> = Vec::new();
        for (name, port) in &ready {
            let links = probe_service_links(*port).await;
            probed.push((name.clone(), *port, links));
        }

        // Proxy
        let proxy_shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>;
        if !options.no_proxy {
            if let Some(proxy_cfg) = crate::proxy::infer_routes(&stack) {
                let (ptx, prx) = tokio::sync::oneshot::channel::<()>();
                match crate::proxy::run_proxy(proxy_cfg, prx).await {
                    Ok(port) => {
                        proxy_shutdown_tx = Some(ptx);
                        let _ = tx.send(RunEvent::PortBound {
                            port,
                            startup_ms: 0,
                            total_ms: overall_start.elapsed().as_millis() as u64,
                            urls: vec![],
                            service_name: None,
                        }).await;
                    }
                    Err(e) => {
                        let _ = tx.send(RunEvent::Warning { message: format!("proxy failed to start: {}", e) }).await;
                        proxy_shutdown_tx = None;
                    }
                }
            } else {
                proxy_shutdown_tx = None;
            }
        } else {
            proxy_shutdown_tx = None;
        }


        for (name, port, links) in &probed {
            let _ = tx.send(RunEvent::PortBound {
                port: *port,
                startup_ms: 0,
                total_ms: overall_start.elapsed().as_millis() as u64,
                urls: links.iter().map(|(l, u)| (l.to_string(), u.clone())).collect(),
                service_name: Some(name.clone()),
            }).await;
        }

        let discovered = url_sink.lock().await.clone();
        let known_ports: std::collections::HashSet<u16> = probed.iter().map(|(_, p, _)| *p).collect();
        let extras: Vec<String> = discovered.iter()
            .filter(|u| {
                if let Some(rest) = u.splitn(2, "://").nth(1) {
                    if let Some(port_str) = rest.split(|c: char| c == '/' || c == '?').next()
                        .and_then(|hp| hp.rsplit(':').next()) {
                        if let Ok(p) = port_str.parse::<u16>() {
                            return !known_ports.contains(&p);
                        }
                    }
                }
                true
            })
            .cloned()
            .collect();
        for u in extras {
            let _ = tx.send(RunEvent::Warning { message: format!("also: {}", u) }).await;
        }

        // Watch mode
        if options.watch {
            let skip_dirs = [
                "node_modules", ".next", "target", "dist", "build", ".turbo",
                ".venv", "venv", "__pycache__", ".git", ".cache", ".pnpm",
                "vendor", "deps", "_build", "out", "bin", "obj", ".gradle", ".mvn",
            ];

            let (change_tx, mut change_rx) = tokio::sync::mpsc::channel::<Vec<PathBuf>>(256);

            let watch_root = project_root.to_path_buf();
            let change_tx_w = change_tx.clone();
            tokio::spawn(async move {
                use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
                let (nw_tx, mut nw_rx) = tokio::sync::mpsc::channel(256);
                let mut watcher = match RecommendedWatcher::new(
                    move |res| { let _ = nw_tx.blocking_send(res); },
                    Config::default(),
                ) {
                    Ok(w) => w,
                    Err(_) => return,
                };
                let _ = watcher.watch(&watch_root, RecursiveMode::Recursive);

                let mut pending = Vec::new();
                loop {
                    tokio::select! {
                        Some(Ok(event)) = nw_rx.recv() => {
                            match event.kind {
                                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                                    for path in event.paths {
                                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                        if skip_dirs.iter().any(|s| *s == name)
                                            || path.extension().and_then(|e| e.to_str()).map_or(false, |e| {
                                                matches!(e, "log" | "lock" | "tmp" | "swp" | "swpx")
                                            })
                                        { continue; }
                                        let mut skip = false;
                                        for ancestor in path.ancestors() {
                                            if let Some(anc_name) = ancestor.file_name().and_then(|n| n.to_str()) {
                                                if skip_dirs.iter().any(|s| *s == anc_name) { skip = true; break; }
                                            }
                                        }
                                        if skip { continue; }
                                        pending.push(path);
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                            if !pending.is_empty() {
                                let _ = change_tx_w.send(std::mem::take(&mut pending)).await;
                            }
                        }
                    }
                }
            });

            let services_info: Vec<_> = stack.services.iter().map(|s|
                (s.name.clone(), PathBuf::from(&s.path), s.build_command.clone(), s.entry_point.clone(), s.port)
            ).collect();

            let mut named_children: Vec<(String, tokio::process::Child)> =
                children.into_iter().map(|(n, _, c)| (n, c)).collect();

            loop {
                tokio::select! {
                    Some(changed) = change_rx.recv() => {
                        let mut affected: Option<usize> = None;
                        for path in &changed {
                            for (i, (_, ref svc_path, ..)) in services_info.iter().enumerate() {
                                if path.starts_with(svc_path) || path == svc_path.as_path() {
                                    affected = Some(i);
                                    break;
                                }
                            }
                            if affected.is_some() { break; }
                        }
                        if let Some(idx) = affected {
                            let (ref name, ref svc_path, ref build_cmd, ref entry, port) = services_info[idx];

                            if let Some(pos) = named_children.iter().position(|(n, _)| n == name) {
                                let (_, mut old_child) = named_children.remove(pos);
                                let _ = old_child.kill().await;
                                let _ = old_child.wait().await;
                            }

                            if !build_cmd.is_empty() {
                                if let Err(e) = run_sub_build(
                                    &stack.services[idx],
                                    build_cmd,
                                    svc_path,
                                    &dep_env,
                                    tx,
                                ).await {
                                    let _ = tx.send(RunEvent::Warning {
                                        message: format!("{}: rebuild failed: {}", name, e),
                                    }).await;
                                    continue;
                                }
                            }

                            let run = entry.replace("$PORT", &port.to_string());
                            let mut new_cmd = spawn_shell(&run, svc_path, &dep_env);
                            new_cmd.env("PORT", port.to_string());
                            new_cmd.stdout(std::process::Stdio::piped());
                            new_cmd.stderr(std::process::Stdio::piped());
                            match new_cmd.spawn() {
                                Ok(mut child) => {
                                    assign_to_job(&child);
                                    if let Some(stdout) = child.stdout.take() {
                                        let n = name.clone();
                                        let tx_c = tx.clone();
                                        tokio::spawn(async move {
                                            use tokio::io::AsyncBufReadExt;
                                            let reader = tokio::io::BufReader::new(stdout);
                                            let mut lines = reader.lines();
                                            while let Ok(Some(line)) = lines.next_line().await {
                                                let _ = tx_c.send(RunEvent::AppOutput {
                                                    line,
                                                    stream: Stream::Stdout,
                                                    service_name: Some(n.clone()),
                                                }).await;
                                            }
                                        });
                                    }
                                    if let Some(stderr) = child.stderr.take() {
                                        let n = name.clone();
                                        let tx_c = tx.clone();
                                        tokio::spawn(async move {
                                            use tokio::io::AsyncBufReadExt;
                                            let reader = tokio::io::BufReader::new(stderr);
                                            let mut lines = reader.lines();
                                            while let Ok(Some(line)) = lines.next_line().await {
                                                let _ = tx_c.send(RunEvent::AppOutput {
                                                    line,
                                                    stream: Stream::Stderr,
                                                    service_name: Some(n.clone()),
                                                }).await;
                                            }
                                        });
                                    }
                                    named_children.push((name.clone(), child));
                                }
                                Err(e) => {
                                    let _ = tx.send(RunEvent::Warning {
                                        message: format!("{}: re-spawn failed: {}", name, e),
                                    }).await;
                                }
                            }
                        }
                    }
                    _ = &mut *abort_rx => { break; }
                    else => { break; }
                }
            }
            // Cleanup (watch branch)
            if let Some(tx_p) = proxy_shutdown_tx { let _ = tx_p.send(()); }
            for (_, mut c) in named_children { kill_tree(&mut c).await; }
            return Ok(());
        } else {
            // Block until any child exits OR abort
            let exited: Option<(String, Option<i32>)>;
            loop {
                let mut hit: Option<(usize, Option<i32>)> = None;
                for (i, (_, _, c)) in children.iter_mut().enumerate() {
                    if let Ok(Some(status)) = c.try_wait() {
                        hit = Some((i, status.code()));
                        break;
                    }
                }
                if let Some((i, code)) = hit {
                    exited = Some((children[i].0.clone(), code));
                    break;
                }
                if abort_rx.try_recv().is_ok() { exited = None; break; }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            if let Some((_name, code)) = exited {
                let _ = tx.send(RunEvent::Exited { code: code.unwrap_or(-1) }).await;
            }
            // Cleanup (non-watch branch)
            if let Some(tx_p) = proxy_shutdown_tx { let _ = tx_p.send(()); }
            for (_, _, mut c) in children { kill_tree(&mut c).await; }
            return Ok(());
        }
    }

    // ── 7. Single-service branch ──────────────────────────────────────
    let port = port_override.unwrap_or(stack.default_port);
    let entry = if options.dev { &stack.dev_entry_point } else { &stack.entry_point };
    let install = if options.dev { &stack.dev_install_command } else { &stack.build_command };

    let entry_str = app_command_override.as_deref().unwrap_or(entry);
    let parts: Vec<&str> = entry_str.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    let lang = stack.language.split(' ').next().unwrap_or("").to_lowercase();

    let is_install_only = (lang == "node" || lang == "typescript" || lang == "bun" || lang == "deno")
        && !install.is_empty() && !install.contains("&&");
    let node_skip = is_install_only && !options.rebuild && node_deps_fresh(project_root);

    // The "dependencies fresh — node_modules newer than lockfile" line in
    // the CLI fires exactly when node_skip caused us to drop the install.
    if node_skip {
        let _ = tx.send(RunEvent::DepsFresh).await;
    }

    let install_cmd: Option<String> = if options.dev {
        if install.is_empty() {
            None
        } else if node_skip {
            None
        } else {
            let needs_install = match lang.as_str() {
                "node" | "typescript" | "bun" | "deno" => !project_root.join("node_modules").exists(),
                "python" => !project_root.join(".venv").exists(),
                "php" => !project_root.join("vendor").exists(),
                "elixir" => !project_root.join("deps").exists(),
                _ => true,
            };
            if needs_install { Some(install.clone()) } else { None }
        }
    } else {
        if install.is_empty() {
            None
        } else if node_skip {
            None
        } else if !options.rebuild {
            if let Some(_reason) = build_freshness(project_root, &stack.language) {
                None
            } else {
                Some(install.clone())
            }
        } else {
            Some(install.clone())
        }
    };

    if let Some(ref icmd) = install_cmd {
        let _ = tx.send(RunEvent::BuildStarted {
            command: icmd.clone(),
            service_name: None,
        }).await;

        let mut cmd = spawn_shell(icmd, project_root, &dep_env);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                assign_to_job(&child);
                if let Some(stdout) = child.stdout.take() {
                    let tx_c = tx.clone();
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = tx_c.send(RunEvent::BuildOutput {
                                line,
                                stream: Stream::Stdout,
                                service_name: None,
                            }).await;
                        }
                    });
                }
                if let Some(stderr) = child.stderr.take() {
                    let tx_c = tx.clone();
                    tokio::spawn(async move {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stderr);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = tx_c.send(RunEvent::BuildOutput {
                                line,
                                stream: Stream::Stderr,
                                service_name: None,
                            }).await;
                        }
                    });
                }

                let status = child.wait().await
                    .map_err(|e| anyhow::anyhow!("Failed to run `{}`: {}", icmd, e))?;
                let build_dur = build_start.elapsed().as_millis() as u64;
                let ok = status.success();
                let _ = tx.send(RunEvent::BuildFinished {
                    duration_ms: build_dur,
                    success: ok,
                    service_name: None,
                }).await;
                if !ok { anyhow::bail!("Build failed: `{}`", icmd); }
            }
            Err(e) => anyhow::bail!("Failed to spawn `{}`: {}", icmd, e),
        }
    }

    // Spawn
    let spawn_start = std::time::Instant::now();
    let _ = tx.send(RunEvent::Spawning {
        command: entry_str.to_string(),
        port,
        service_name: None,
    }).await;

    let mut cmd = spawn_shell(entry_str, project_root, &dep_env);
    cmd.env("PORT", port.to_string());
    if matches!(lang.as_str(), "python") {
        cmd.env("PYTHONUTF8", "1");
        cmd.env("PYTHONUNBUFFERED", "1");
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start `{}`: {}", entry_str, e))?;
    assign_to_job(&child);

    let url_sink: UrlSink = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    if let Some(stdout) = child.stdout.take() {
        let sink = url_sink.clone();
        let tx_c = tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                record_urls(&line, &sink).await;
                let _ = tx_c.send(RunEvent::AppOutput {
                    line,
                    stream: Stream::Stdout,
                    service_name: None,
                }).await;
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sink = url_sink.clone();
        let tx_c = tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                record_urls(&line, &sink).await;
                let _ = tx_c.send(RunEvent::AppOutput {
                    line,
                    stream: Stream::Stderr,
                    service_name: None,
                }).await;
            }
        });
    }

    // Port probe
    let mut port_ready = false;
    let mut aborted = false;
    for _ in 0..100u32 {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await.is_ok() {
            port_ready = true;
            break;
        }
        if let Ok(Some(_)) = child.try_wait() { break; }
        if abort_rx.try_recv().is_ok() { aborted = true; break; }
    }

    // Aborted during startup → kill the tree and bail (don't fall through to a
    // blocking wait that would orphan the dev server).
    if aborted {
        kill_tree(&mut child).await;
        let _ = tx.send(RunEvent::Exited { code: 130 }).await;
        return Ok(());
    }

    if port_ready {
        let startup_ms = spawn_start.elapsed().as_millis() as u64;
        let total_ms = overall_start.elapsed().as_millis() as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        let probed = probe_service_links_for(port, &stack.language).await;
        let urls = url_sink.lock().await;
        let mut all_urls: Vec<(String, String)> = probed.iter().map(|(l, u)| (l.to_string(), u.clone())).collect();
        for u in urls.iter() {
            all_urls.push(("discovered".to_string(), u.clone()));
        }

        let _ = tx.send(RunEvent::PortBound {
            port,
            startup_ms,
            total_ms,
            urls: all_urls,
            service_name: None,
        }).await;
    } else if let Ok(Some(status)) = child.try_wait() {
        let _ = tx.send(RunEvent::Warning {
            message: format!("app exited before binding :{} (exit code {})", port, status.code().unwrap_or(-1)),
        }).await;
    } else {
        let _ = tx.send(RunEvent::Warning {
            message: format!("no response on :{} after 10s — app may still be starting or bound to a different port", port),
        }).await;
    }

    // Wait for exit OR abort. On abort, kill the whole tree (npm → node/vite)
    // so the dev server doesn't orphan and keep holding the port.
    tokio::select! {
        status = child.wait() => {
            let code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
            let _ = tx.send(RunEvent::Exited { code }).await;
        }
        _ = &mut *abort_rx => {
            kill_tree(&mut child).await;
            let _ = tx.send(RunEvent::Exited { code: 130 }).await;
        }
    }

    Ok(())
}

fn topological_sort(services: &[SubService]) -> Vec<SubService> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut temp = std::collections::HashSet::new();

    fn visit(
        name: &str,
        services: &[SubService],
        visited: &mut std::collections::HashSet<String>,
        temp: &mut std::collections::HashSet<String>,
        result: &mut Vec<SubService>,
    ) {
        if temp.contains(name) {
            return;
        }
        if !visited.contains(name) {
            temp.insert(name.to_string());
            if let Some(sub) = services.iter().find(|s| s.name == name) {
                for dep in &sub.depends_on {
                    visit(dep, services, visited, temp, result);
                }
                result.push(sub.clone());
            }
            temp.remove(name);
            visited.insert(name.to_string());
        }
    }

    for service in services {
        visit(&service.name, services, &mut visited, &mut temp, &mut result);
    }

    result
}

async fn wait_for_port(port: u16, timeout_secs: u64) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout_secs {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            return true;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    false
}
