//! Android device / emulator mirroring for the mobile run view.
//!
//! The emulator runs as its own native OS window that can't be embedded in the
//! Tauri webview, so we mirror it instead: pull the framebuffer with
//! `adb exec-out screencap -p` (raw PNG) and hand it to the UI as a data URL,
//! and forward taps/swipes/keys back with `adb shell input`. Read-mostly, but
//! enough to *see and drive* the running app inside crush.
//!
//! Requires `adb` on PATH (ships with the Android SDK platform-tools — the same
//! prerequisite as `flutter run` on Android).

use base64::Engine;
use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct AdbDevice {
    pub serial: String,
    /// "device" (online), "offline", "unauthorized", "emulator", …
    pub state: String,
    pub is_emulator: bool,
}

/// Run `adb` with args, returning raw stdout bytes (errors map to String).
async fn adb(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("adb")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("adb not found (install Android platform-tools): {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(out.stdout)
}

/// List attached devices/emulators (`adb devices`).
#[tauri::command]
pub async fn adb_devices() -> Result<Vec<AdbDevice>, String> {
    let raw = adb(&["devices"]).await?;
    let text = String::from_utf8_lossy(&raw);
    let mut devices = Vec::new();
    // Skip the "List of devices attached" header line.
    for line in text.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() { continue; }
        let mut parts = line.split_whitespace();
        let (Some(serial), Some(state)) = (parts.next(), parts.next()) else { continue };
        devices.push(AdbDevice {
            serial: serial.to_string(),
            state: state.to_string(),
            is_emulator: serial.starts_with("emulator-"),
        });
    }
    Ok(devices)
}

/// Capture the device screen as a PNG `data:` URL the UI can put in an <img>.
/// `serial` empty → adb's default device.
#[tauri::command]
pub async fn device_screencap(serial: String) -> Result<String, String> {
    let png = if serial.is_empty() {
        adb(&["exec-out", "screencap", "-p"]).await?
    } else {
        adb(&["-s", &serial, "exec-out", "screencap", "-p"]).await?
    };
    if png.is_empty() { return Err("empty screencap (device asleep or unauthorized?)".into()); }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// Forward a tap at device pixel coordinates (`adb shell input tap x y`).
#[tauri::command]
pub async fn device_tap(serial: String, x: i32, y: i32) -> Result<(), String> {
    let (xs, ys) = (x.to_string(), y.to_string());
    let mut args = vec![];
    if !serial.is_empty() { args.extend(["-s", serial.as_str()]); }
    args.extend(["shell", "input", "tap", xs.as_str(), ys.as_str()]);
    adb(&args).await.map(|_| ())
}

/// Forward a swipe in device pixels over `ms` milliseconds.
#[tauri::command]
pub async fn device_swipe(serial: String, x1: i32, y1: i32, x2: i32, y2: i32, ms: u32) -> Result<(), String> {
    let (a, b, c, d, m) = (x1.to_string(), y1.to_string(), x2.to_string(), y2.to_string(), ms.to_string());
    let mut args = vec![];
    if !serial.is_empty() { args.extend(["-s", serial.as_str()]); }
    args.extend(["shell", "input", "swipe", a.as_str(), b.as_str(), c.as_str(), d.as_str(), m.as_str()]);
    adb(&args).await.map(|_| ())
}

/// Forward an Android keyevent (e.g. 4 = BACK, 3 = HOME, 187 = APP_SWITCH).
#[tauri::command]
pub async fn device_key(serial: String, keycode: i32) -> Result<(), String> {
    let kc = keycode.to_string();
    let mut args = vec![];
    if !serial.is_empty() { args.extend(["-s", serial.as_str()]); }
    args.extend(["shell", "input", "keyevent", kc.as_str()]);
    adb(&args).await.map(|_| ())
}
