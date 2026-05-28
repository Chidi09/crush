use serde::Serialize;
use tauri::{Emitter, Runtime};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LogLine {
    pub ts: String,
    pub stream: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PullProgress {
    pub layer: String,
    pub current_bytes: u64,
    pub total_bytes: u64,
}

pub fn emit_log_line<R: Runtime>(emitter: &impl Emitter<R>, container_id: &str, ts: &str, stream: &str, text: &str) {
    let _ = emitter.emit(&format!("log-line::{}", container_id), LogLine {
        ts: ts.to_string(),
        stream: stream.to_string(),
        text: text.to_string(),
    });
}

pub fn emit_pull_progress<R: Runtime>(emitter: &impl Emitter<R>, image: &str, layer: &str, current: u64, total: u64) {
    let _ = emitter.emit(&format!("pull-progress::{}", image), PullProgress {
        layer: layer.to_string(),
        current_bytes: current,
        total_bytes: total,
    });
}

pub fn emit_container_state_changed<R: Runtime>(emitter: &impl Emitter<R>) {
    let _ = emitter.emit("container-state-changed", ());
}

pub fn emit_service_state_changed<R: Runtime>(emitter: &impl Emitter<R>) {
    let _ = emitter.emit("service-state-changed", ());
}
