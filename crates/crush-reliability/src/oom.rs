use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OomPolicy {
    Restart,
    ReportOnly,
}

pub struct OomMonitor {
    container_id: String,
    memory_events_path: String,
    policy: OomPolicy,
    last_oom_count: u64,
}

impl OomMonitor {
    pub fn new(container_id: &str, policy: OomPolicy) -> Self {
        let cgroup = format!("/sys/fs/cgroup/crush/{}", container_id);
        Self {
            container_id: container_id.to_string(),
            memory_events_path: format!("{}/memory.events", cgroup),
            policy,
            last_oom_count: 0,
        }
    }

    pub async fn poll(&mut self) -> Result<OomEvent> {
        let path = Path::new(&self.memory_events_path);
        if !path.exists() {
            return Ok(OomEvent::None);
        }

        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| CrushError::CgroupError(format!("Failed to read memory.events: {}", e)))?;

        for line in content.lines() {
            if let Some(count_str) = line.strip_prefix("oom_kill ") {
                if let Ok(count) = count_str.trim().parse::<u64>() {
                    if count > self.last_oom_count {
                        let delta = count - self.last_oom_count;
                        self.last_oom_count = count;
                        return Ok(OomEvent::OomKilled {
                            container_id: self.container_id.clone(),
                            peak_memory: self.read_peak_memory().await.unwrap_or(0),
                            count: delta,
                        });
                    }
                }
            }
        }

        Ok(OomEvent::None)
    }

    async fn read_peak_memory(&self) -> Result<u64> {
        let path = format!("/sys/fs/cgroup/crush/{}/memory.peak", self.container_id);
        let path = Path::new(&path);
        if path.exists() {
            let content = tokio::fs::read_to_string(path).await
                .map_err(|e| CrushError::CgroupError(e.to_string()))?;
            content.trim().parse::<u64>()
                .map_err(|e| CrushError::CgroupError(e.to_string()))
        } else {
            Ok(0)
        }
    }
}

#[derive(Debug, Clone)]
pub enum OomEvent {
    None,
    OomKilled { container_id: String, peak_memory: u64, count: u64 },
}
