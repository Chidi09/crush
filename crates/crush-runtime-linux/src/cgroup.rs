use std::fs;
use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError};

pub struct CgroupManager {
    cgroup_path: PathBuf,
}

impl CgroupManager {
    pub fn new(container_id: &str) -> Self {
        // ⚠ Sanitize container_id to prevent cgroup path traversal
        let safe_id: String = container_id.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .take(64)
            .collect();
        Self {
            cgroup_path: PathBuf::from("/sys/fs/cgroup/crush").join(safe_id),
        }
    }

    pub fn initialize_cgroup(&self) -> Result<()> {
        fs::create_dir_all(&self.cgroup_path)
            .map_err(|e| CrushError::CgroupError(format!("Failed to create cgroup: {}", e)))
    }

    pub fn enforce_memory_limit(&self, max_bytes: u64) -> Result<()> {
        self.write_file("memory.max", max_bytes.to_string())?;
        self.write_file("memory.oom.group", "1".to_string())
    }

    pub fn enforce_cpu_limit(&self, cpu_weight: u64) -> Result<()> {
        self.write_file("cpu.weight", cpu_weight.to_string())
    }

    pub fn enforce_pids_limit(&self, max_pids: u32) -> Result<()> {
        self.write_file("pids.max", max_pids.to_string())
    }

    pub fn set_freeze_state(&self, freeze: bool) -> Result<()> {
        self.write_file("cgroup.freeze", if freeze { "1".to_string() } else { "0".to_string() })
    }

    pub fn add_process_to_cgroup(&self, pid: u32) -> Result<()> {
        self.write_file("cgroup.procs", pid.to_string())
    }

    pub fn remove_cgroup(&self) -> Result<()> {
        if self.cgroup_path.exists() {
            fs::remove_dir(&self.cgroup_path)
                .map_err(|e| CrushError::CgroupError(format!("Failed to remove cgroup: {}", e)))
        } else { Ok(()) }
    }

    fn write_file(&self, name: &str, value: String) -> Result<()> {
        let path = self.cgroup_path.join(name);
        fs::write(&path, &value)
            .map_err(|e| CrushError::CgroupError(format!("Failed to write {}: {}", name, e)))
    }
}
