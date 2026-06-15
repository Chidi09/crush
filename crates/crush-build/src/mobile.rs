//! Mobile / app run path.
//!
//! Flutter and React Native apps don't run as OCI containers or host web
//! servers — they run on a device or emulator through their own toolchain
//! (`flutter`, `react-native`, `expo`). This module owns:
//!   1. dependency install (`flutter pub get` / `npm install`),
//!   2. device selection — reuse a running device/emulator, else boot one,
//!   3. launching the app and streaming its output as [`RunEvent`]s,
//! honoring abort (stop button / Ctrl-C) by tree-killing the toolchain.
//!
//! Emulator *acceleration* (KVM/HAXM/WHPX) is a host capability; this code is
//! verified for detection, device-list parsing, and command construction. The
//! actual emulator boot is exercised on a developer machine with virtualization.

use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{mpsc, oneshot};

use crate::run::{kill_tree, spawn_shell, RunEvent, Stream};

/// Entry point: dispatch a detected mobile stack to its runner.
pub async fn run_mobile(
    language: &str,
    project_root: &Path,
    _dev: bool,
    tx: &mpsc::Sender<RunEvent>,
    abort_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let lang = language.split(' ').next().unwrap_or("").to_lowercase();
    match lang.as_str() {
        "flutter" => run_flutter(project_root, tx, abort_rx).await,
        "react-native" => run_react_native(project_root, language, tx, abort_rx).await,
        other => anyhow::bail!("unsupported mobile stack: {other}"),
    }
}

// ── Flutter ────────────────────────────────────────────────────────────────

async fn run_flutter(
    root: &Path,
    tx: &mpsc::Sender<RunEvent>,
    abort_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    ensure_tool("flutter", "https://docs.flutter.dev/get-started/install").await?;

    // 1. Dependencies.
    run_step("flutter pub get", root, tx, abort_rx).await?;

    // 2. Pick (or boot) a device.
    let device = select_flutter_device(tx, abort_rx).await?;

    // 3. Launch on the device, streaming logs. `flutter run` stays attached and
    //    forwards device logs + hot-reload prompts on its stdout.
    let cmd = format!("flutter run -d {device}");
    let _ = tx.send(RunEvent::Spawning { command: cmd.clone(), port: 0, service_name: None }).await;
    stream_until_exit_or_abort(&cmd, root, tx, abort_rx).await
}

/// Choose a Flutter device id, booting an emulator when none is running.
/// Strategy: prefer a real/emulated mobile device already online; otherwise
/// launch the first available emulator and wait for it to come online; if
/// there's nothing to launch, fail with actionable setup steps.
async fn select_flutter_device(
    tx: &mpsc::Sender<RunEvent>,
    abort_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<String> {
    if let Some(id) = pick_device(&flutter_devices_json().await?) {
        let _ = tx.send(RunEvent::Warning { message: format!("using device {id}") }).await;
        return Ok(id);
    }

    // Nothing online — try to boot an emulator.
    let emus = flutter_emulators().await?;
    let Some(emu) = emus.first().cloned() else {
        anyhow::bail!(
            "no device or emulator available.\n   \u{21B3} create one with `flutter emulators --create` \
             (or Android Studio \u{2192} Device Manager), then re-run.\n   \u{21B3} or plug in a device with USB debugging enabled."
        );
    };
    let _ = tx.send(RunEvent::Warning { message: format!("booting emulator {emu}\u{2026}") }).await;
    // Launch detached — it keeps running independently of this call.
    let _ = spawn_shell(&format!("flutter emulators --launch {emu}"), Path::new("."), &[])
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch emulator {emu}: {e}"))?;

    // Poll for the emulator to come online (up to ~120s), bailing on abort.
    for _ in 0..120u32 {
        if abort_rx.try_recv().is_ok() { anyhow::bail!("aborted while waiting for emulator"); }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if let Ok(json) = flutter_devices_json().await {
            if let Some(id) = pick_device(&json) {
                let _ = tx.send(RunEvent::Warning { message: format!("emulator online: {id}") }).await;
                return Ok(id);
            }
        }
    }
    anyhow::bail!("emulator {emu} did not come online in time")
}

async fn flutter_devices_json() -> anyhow::Result<serde_json::Value> {
    let out = capture("flutter devices --machine").await?;
    Ok(serde_json::from_str(&out).unwrap_or(serde_json::Value::Array(vec![])))
}

async fn flutter_emulators() -> anyhow::Result<Vec<String>> {
    // `flutter emulators --machine` prints a JSON array of {id,name,...}.
    let out = capture("flutter emulators --machine").await.unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&out).unwrap_or(serde_json::Value::Array(vec![]));
    Ok(json.as_array().map(|arr| {
        arr.iter().filter_map(|e| e.get("id").and_then(|v| v.as_str()).map(String::from)).collect()
    }).unwrap_or_default())
}

/// Pick the best device id from `flutter devices --machine` output: a running
/// mobile device/emulator (android/ios) wins over web/desktop targets. Pure
/// function over the parsed JSON so it can be unit-tested without a toolchain.
fn pick_device(json: &serde_json::Value) -> Option<String> {
    let arr = json.as_array()?;
    let is_mobile = |d: &serde_json::Value| {
        let platform = d.get("targetPlatform").and_then(|v| v.as_str()).unwrap_or("");
        let emulator = d.get("emulator").and_then(|v| v.as_bool()).unwrap_or(false);
        platform.starts_with("android") || platform.starts_with("ios") || emulator
    };
    // Prefer a mobile device; fall back to nothing (caller boots an emulator).
    arr.iter()
        .find(|d| is_mobile(d))
        .and_then(|d| d.get("id").and_then(|v| v.as_str()).map(String::from))
}

// ── React Native ─────────────────────────────────────────────────────────

async fn run_react_native(
    root: &Path,
    language: &str,
    tx: &mpsc::Sender<RunEvent>,
    abort_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    ensure_tool("npx", "https://nodejs.org/").await?;

    // 1. Dependencies.
    run_step("npm install", root, tx, abort_rx).await?;

    // 2. Make sure an Android emulator/device is online; RN's run-android
    //    expects one (it doesn't boot an AVD itself). Best-effort check via adb.
    if !android_device_online().await {
        let _ = tx.send(RunEvent::Warning {
            message: "no Android device/emulator detected — start one (Android Studio \u{2192} Device Manager, \
                      or `emulator -avd <name>`) so the app has somewhere to install.".to_string(),
        }).await;
    }

    // 3. Launch. Expo and bare RN use different CLIs.
    let cmd = if language.contains("Expo") {
        "npx expo run:android"
    } else {
        "npx react-native run-android"
    };
    let _ = tx.send(RunEvent::Spawning { command: cmd.to_string(), port: 0, service_name: None }).await;
    stream_until_exit_or_abort(cmd, root, tx, abort_rx).await
}

async fn android_device_online() -> bool {
    // `adb devices` lists "<serial>\tdevice" lines for online devices.
    match capture("adb devices").await {
        Ok(out) => out.lines().skip(1).any(|l| l.trim_end().ends_with("\tdevice") || l.split_whitespace().nth(1) == Some("device")),
        Err(_) => false,
    }
}

// ── shared helpers ─────────────────────────────────────────────────────────

/// Verify a CLI tool is runnable; otherwise fail with an install pointer.
async fn ensure_tool(tool: &str, install_url: &str) -> anyhow::Result<()> {
    let probe = spawn_shell(&format!("{tool} --version"), Path::new("."), &[])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    match probe {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("`{tool}` not found on PATH — install it: {install_url}"),
    }
}

/// Run a command and capture its stdout (trimmed). Used for `--machine` queries.
async fn capture(cmdline: &str) -> anyhow::Result<String> {
    let out = spawn_shell(cmdline, Path::new("."), &[])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("`{cmdline}` failed to run: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A build/install step: stream output as BuildOutput, honor abort, bail on
/// non-zero exit. Mirrors the main run path's build semantics.
async fn run_step(
    cmdline: &str,
    cwd: &Path,
    tx: &mpsc::Sender<RunEvent>,
    abort_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let _ = tx.send(RunEvent::BuildStarted { command: cmdline.to_string(), service_name: None }).await;
    let start = std::time::Instant::now();

    let mut cmd = spawn_shell(cmdline, cwd, &[]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("failed to start `{cmdline}`: {e}"))?;
    pump(child.stdout.take(), tx.clone(), Stream::Stdout, false);
    pump(child.stderr.take(), tx.clone(), Stream::Stderr, false);

    let status = tokio::select! {
        s = child.wait() => s.map_err(|e| anyhow::anyhow!("`{cmdline}` wait failed: {e}"))?,
        _ = &mut *abort_rx => {
            kill_tree(&mut child).await;
            let _ = tx.send(RunEvent::Exited { code: 130 }).await;
            anyhow::bail!("aborted");
        }
    };
    let ok = status.success();
    let _ = tx.send(RunEvent::BuildFinished {
        duration_ms: start.elapsed().as_millis() as u64,
        success: ok,
        service_name: None,
    }).await;
    if !ok { anyhow::bail!("`{cmdline}` failed"); }
    Ok(())
}

/// Run the long-lived app command, streaming output as AppOutput until it exits
/// or the run is aborted (on abort, tree-kill so the toolchain + emulator log
/// taps don't orphan).
async fn stream_until_exit_or_abort(
    cmdline: &str,
    cwd: &Path,
    tx: &mpsc::Sender<RunEvent>,
    abort_rx: &mut oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let mut cmd = spawn_shell(cmdline, cwd, &[]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("failed to start `{cmdline}`: {e}"))?;
    crate::run::assign_to_job(&child);
    pump(child.stdout.take(), tx.clone(), Stream::Stdout, true);
    pump(child.stderr.take(), tx.clone(), Stream::Stderr, true);

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

/// Forward a child stdio stream line-by-line as RunEvents.
fn pump(
    pipe: Option<impl tokio::io::AsyncRead + Unpin + Send + 'static>,
    tx: mpsc::Sender<RunEvent>,
    stream: Stream,
    app: bool,
) {
    let Some(pipe) = pipe else { return };
    tokio::spawn(async move {
        let mut lines = tokio::io::BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let ev = if app {
                RunEvent::AppOutput { line, stream: stream.clone(), service_name: None }
            } else {
                RunEvent::BuildOutput { line, stream: stream.clone(), service_name: None }
            };
            if tx.send(ev).await.is_err() { break; }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_running_android_emulator_over_web() {
        let json = serde_json::json!([
            { "id": "chrome", "targetPlatform": "web-javascript", "emulator": false },
            { "id": "emulator-5554", "targetPlatform": "android-x64", "emulator": true }
        ]);
        assert_eq!(pick_device(&json).as_deref(), Some("emulator-5554"));
    }

    #[test]
    fn picks_physical_device() {
        let json = serde_json::json!([
            { "id": "RF8M2", "targetPlatform": "android-arm64", "emulator": false }
        ]);
        assert_eq!(pick_device(&json).as_deref(), Some("RF8M2"));
    }

    #[test]
    fn no_mobile_device_returns_none() {
        let json = serde_json::json!([
            { "id": "chrome", "targetPlatform": "web-javascript", "emulator": false },
            { "id": "linux", "targetPlatform": "linux-x64", "emulator": false }
        ]);
        assert_eq!(pick_device(&json), None);
    }

    #[test]
    fn empty_device_list_returns_none() {
        assert_eq!(pick_device(&serde_json::json!([])), None);
    }
}
