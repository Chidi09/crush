use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use chrono::Utc;
use serde::Serialize;
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub container_id: Option<String>,
    pub image: Option<String>,
    pub message: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    ContainerCreate,
    ContainerStart,
    ContainerStop,
    ContainerExec,
    ContainerDelete,
    ImagePull,
    ImagePush,
    VolumeMount,
    SecretAccess,
    HealthCheckFailure,
    OomKill,
    RestartAttempt,
}

pub struct AuditLogger {
    log_path: PathBuf,
    max_file_size: u64,
    max_rotations: u32,
}

impl AuditLogger {
    pub fn new(base_dir: &Path) -> Self {
        let log_dir = base_dir.join("audit");
        fs::create_dir_all(&log_dir).ok();
        Self {
            log_path: log_dir.join("audit.log"),
            max_file_size: 100 * 1024 * 1024,
            max_rotations: 5,
        }
    }

    pub fn log(&self, event: AuditEvent) -> Result<()> {
        self.rotate_if_needed()?;

        let line = serde_json::to_string(&event)
            .map_err(|e| CrushError::StorageError(format!("Audit serialization: {}", e)))?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(|e| CrushError::StorageError(format!("Audit open: {}", e)))?;

        writeln!(file, "{}", line)
            .map_err(|e| CrushError::StorageError(format!("Audit write: {}", e)))?;

        Ok(())
    }

    fn rotate_if_needed(&self) -> Result<()> {
        if !self.log_path.exists() { return Ok(()); }

        let len = fs::metadata(&self.log_path)
            .map(|m| m.len()).unwrap_or(0);

        if len < self.max_file_size { return Ok(()); }

        for i in (1..self.max_rotations).rev() {
            let src = self.log_path.with_extension(format!("log.{}", i));
            let dst = self.log_path.with_extension(format!("log.{}", i + 1));
            if src.exists() { let _ = fs::rename(&src, &dst); }
        }

        let rotated = self.log_path.with_extension("log.1");
        let _ = fs::rename(&self.log_path, &rotated);

        Ok(())
    }

    pub fn event(
        event_type: AuditEventType,
        container_id: Option<String>,
        image: Option<String>,
        message: String,
    ) -> AuditEvent {
        AuditEvent {
            timestamp: Utc::now().to_rfc3339(),
            event_type,
            container_id,
            image,
            message,
            metadata: serde_json::Value::Null,
        }
    }
}
