use std::path::PathBuf;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EjectResult {
    pub dockerfile: String,
    pub compose: String,
}

/// Eject a project to standard Docker artifacts (Dockerfile + docker-compose.yml).
///
/// The generation logic lives in the `crush` CLI (`crush eject`), so we shell
/// out to it rather than duplicate the Dockerfile/compose templates — same
/// output as the CLI, single source of truth. Requires the `crush` binary on
/// PATH (the same one the user runs for `crush run`).
#[tauri::command]
pub async fn eject_project(path: String, force: bool) -> Result<EjectResult, String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    let mut args: Vec<String> = vec!["eject".into(), "--out".into(), ".".into()];
    if force {
        args.push("--force".into());
    }

    let output = tokio::process::Command::new("crush")
        .args(&args)
        .current_dir(&root)
        .output()
        .await
        .map_err(|e| format!("Failed to run `crush eject` (is the crush CLI on PATH?): {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = format!("{}{}", stdout, stderr);
        return Err(msg.trim().to_string());
    }

    Ok(EjectResult {
        dockerfile: root.join("Dockerfile").to_string_lossy().to_string(),
        compose: root.join("docker-compose.yml").to_string_lossy().to_string(),
    })
}
