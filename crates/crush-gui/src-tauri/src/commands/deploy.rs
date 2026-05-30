use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{Window, Emitter};

/// Allow-list of deploy CLIs we'll spawn. The frontend catalog drives *which*
/// command runs; this just bounds it to known provider tooling so an arbitrary
/// binary can't be launched via the IPC bridge.
const ALLOWED: &[&str] = &[
    "railway", "flyctl", "fly", "doctl", "gcloud", "aws", "az",
    "docker", "curl", "crush", "render", "magento-cloud",
    "vercel", "netlify", "hcloud", "ssh", "scp",
];

fn project_file(root: &Path, filename: &str) -> Result<PathBuf, String> {
    // Allow nested paths like ".do/app.yaml" but never traversal.
    if filename.contains("..") || filename.starts_with('/') || filename.contains(':') {
        return Err(format!("Invalid filename: {filename}"));
    }
    Ok(root.join(filename))
}

/// Write a generated provider config (railway.json, fly.toml, .do/app.yaml, …)
/// into the project. Returns the absolute path written.
#[tauri::command]
pub async fn write_project_file(path: String, filename: String, content: String) -> Result<String, String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    let target = project_file(&root, &filename)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&target, content).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().to_string())
}

/// True if a provider CLI is installed (`<program> <probe-arg>` exits 0).
#[tauri::command]
pub async fn cli_available(program: String, probe: String) -> Result<bool, String> {
    if !ALLOWED.contains(&program.as_str()) {
        return Ok(false);
    }
    Ok(tokio::process::Command::new(&program)
        .arg(probe)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false))
}

/// Run an allow-listed deploy command in the project dir, streaming each output
/// line to the frontend over `deploy-line`, then `deploy-exit` with the code.
/// `env` carries provider tokens (RAILWAY_TOKEN, FLY_API_TOKEN, …) for the child
/// only — they are not persisted.
#[tauri::command]
pub async fn run_deploy(
    path: String,
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    window: Window,
) -> Result<(), String> {
    if !ALLOWED.contains(&program.as_str()) {
        return Err(format!("Program not allowed: {program}"));
    }
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&args).current_dir(&root);
    for (k, v) in &env {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start `{program}` (is its CLI installed / on PATH?): {e}"))?;

    if let Some(out) = child.stdout.take() {
        let w = window.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(out).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = w.emit("deploy-line", serde_json::json!({ "stream": "stdout", "line": l }));
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let w = window.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(err).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = w.emit("deploy-line", serde_json::json!({ "stream": "stderr", "line": l }));
            }
        });
    }

    let w = window.clone();
    tokio::spawn(async move {
        let code = child.wait().await.ok().and_then(|s| s.code()).unwrap_or(-1);
        let _ = w.emit("deploy-exit", serde_json::json!({ "code": code }));
    });

    Ok(())
}

/// Pop a real OS console in the project dir running `command`, so the user can
/// complete interactive steps the GUI can't drive headlessly — browser-based
/// auth (`railway login`, `gcloud auth login`, …) and y/n / picker prompts.
/// The window stays open (`cmd /k`) so output and prompts are visible.
#[tauri::command]
pub async fn open_terminal(path: String, command: String) -> Result<(), String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        std::process::Command::new("cmd")
            .args(["/k", &command])
            .current_dir(&root)
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|e| format!("Failed to open terminal: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = command;
        Err("open_terminal is only implemented on Windows for now".into())
    }
}

/// One-shot allow-listed command (status / URL lookups). Returns combined
/// stdout+stderr so the frontend can JSON-parse the provider's output.
#[tauri::command]
pub async fn run_capture(
    path: String,
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
) -> Result<String, String> {
    if !ALLOWED.contains(&program.as_str()) {
        return Err(format!("Program not allowed: {program}"));
    }
    let out = tokio::process::Command::new(&program)
        .args(&args)
        .current_dir(PathBuf::from(&path))
        .envs(&env)
        .output()
        .await
        .map_err(|e| format!("Failed to run `{program}`: {e}"))?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        s.push('\n');
        s.push_str(&err);
    }
    Ok(s)
}
