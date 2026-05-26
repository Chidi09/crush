use std::path::PathBuf;
use async_trait::async_trait;
use crush_types::{RuntimeBackend, Container, Result, CrushError, ContainerStatus};

pub struct StatelessEngine {
    data_dir: PathBuf,
}

impl StatelessEngine {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

#[async_trait]
impl RuntimeBackend for StatelessEngine {
    async fn create(&self, container: &Container, _spec_path: &PathBuf) -> Result<()> {
        let container_dir = self.data_dir.join("containers").join(&container.id);
        std::fs::create_dir_all(&container_dir)
            .map_err(|e| CrushError::StorageError(format!("Failed to create container dir: {}", e)))?;
        let json_path = container_dir.join("container.json");
        let serialized = serde_json::to_string_pretty(container)
            .map_err(|e| CrushError::StorageError(format!("Failed to serialize container: {}", e)))?;
        std::fs::write(&json_path, serialized)
            .map_err(|e| CrushError::StorageError(format!("Failed to write container.json: {}", e)))?;
        Ok(())
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        let container_dir = self.data_dir.join("containers").join(container_id);
        let container_json_path = container_dir.join("container.json");
        if !container_json_path.exists() {
            return Err(CrushError::ContainerNotFound(container_id.to_string()));
        }

        let content = std::fs::read_to_string(&container_json_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read container.json: {}", e)))?;
        let mut c: Container = serde_json::from_str(&content)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse container.json: {}", e)))?;

        // Detached spawn of the current executable with subcommand internal-run
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("crush"));
        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("internal-run").arg(container_id);

        let out_log = std::fs::File::create(container_dir.join("crush-run.log")).ok();
        let err_log = std::fs::File::create(container_dir.join("crush-run.err")).ok();

        if let Some(f) = out_log {
            cmd.stdout(f);
        } else {
            cmd.stdout(std::process::Stdio::null());
        }

        if let Some(f) = err_log {
            cmd.stderr(f);
        } else {
            cmd.stderr(std::process::Stdio::null());
        }
        cmd.stdin(std::process::Stdio::null());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x00000008); // DETACHED_PROCESS
        }

        let child = cmd.spawn()
            .map_err(|e| CrushError::Internal(anyhow::anyhow!("Failed to spawn internal-run: {}", e)))?;

        let pid = child.id();

        c.status = ContainerStatus::Running;
        c.pid = Some(pid);
        c.started_at = Some(std::time::SystemTime::now());

        let serialized = serde_json::to_string_pretty(&c)
            .map_err(|e| CrushError::StorageError(format!("Failed to serialize container: {}", e)))?;
        std::fs::write(&container_json_path, serialized)
            .map_err(|e| CrushError::StorageError(format!("Failed to write container.json: {}", e)))?;

        Ok(())
    }

    async fn stop(&self, container_id: &str, timeout_seconds: u32) -> Result<()> {
        let container_dir = self.data_dir.join("containers").join(container_id);
        let container_json_path = container_dir.join("container.json");
        if !container_json_path.exists() {
            return Err(CrushError::ContainerNotFound(container_id.to_string()));
        }
        let content = std::fs::read_to_string(&container_json_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read container.json: {}", e)))?;
        let mut c: Container = serde_json::from_str(&content)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse container.json: {}", e)))?;

        if let Some(pid) = c.pid {
            #[cfg(unix)]
            {
                unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM); }
                let start = std::time::Instant::now();
                let mut killed = false;
                while start.elapsed().as_secs() < timeout_seconds as u64 {
                    if unsafe { libc::kill(pid as libc::pid_t, 0) != 0 } {
                        killed = true;
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                if !killed {
                    unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL); }
                }
            }
            #[cfg(windows)]
            {
                let mut kill_cmd = std::process::Command::new("taskkill");
                kill_cmd.args(&["/F", "/PID", &pid.to_string()]);
                let _ = kill_cmd.status();
            }
        }

        c.status = ContainerStatus::Stopped;
        c.pid = None;
        let serialized = serde_json::to_string_pretty(&c)
            .map_err(|e| CrushError::StorageError(format!("Failed to serialize container: {}", e)))?;
        std::fs::write(&container_json_path, serialized)
            .map_err(|e| CrushError::StorageError(format!("Failed to write container.json: {}", e)))?;

        Ok(())
    }

    async fn pause(&self, container_id: &str) -> Result<()> {
        let container_dir = self.data_dir.join("containers").join(container_id);
        let container_json_path = container_dir.join("container.json");
        if !container_json_path.exists() {
            return Err(CrushError::ContainerNotFound(container_id.to_string()));
        }
        let content = std::fs::read_to_string(&container_json_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read container.json: {}", e)))?;
        let mut c: Container = serde_json::from_str(&content)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse container.json: {}", e)))?;

        if let Some(pid) = c.pid {
            #[cfg(unix)]
            {
                unsafe { libc::kill(pid as libc::pid_t, libc::SIGSTOP); }
            }
        }
        c.status = ContainerStatus::Paused;
        let serialized = serde_json::to_string_pretty(&c)
            .map_err(|e| CrushError::StorageError(format!("Failed to serialize container: {}", e)))?;
        std::fs::write(&container_json_path, serialized)
            .map_err(|e| CrushError::StorageError(format!("Failed to write container.json: {}", e)))?;
        Ok(())
    }

    async fn resume(&self, container_id: &str) -> Result<()> {
        let container_dir = self.data_dir.join("containers").join(container_id);
        let container_json_path = container_dir.join("container.json");
        if !container_json_path.exists() {
            return Err(CrushError::ContainerNotFound(container_id.to_string()));
        }
        let content = std::fs::read_to_string(&container_json_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read container.json: {}", e)))?;
        let mut c: Container = serde_json::from_str(&content)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse container.json: {}", e)))?;

        if let Some(pid) = c.pid {
            #[cfg(unix)]
            {
                unsafe { libc::kill(pid as libc::pid_t, libc::SIGCONT); }
            }
        }
        c.status = ContainerStatus::Running;
        let serialized = serde_json::to_string_pretty(&c)
            .map_err(|e| CrushError::StorageError(format!("Failed to serialize container: {}", e)))?;
        std::fs::write(&container_json_path, serialized)
            .map_err(|e| CrushError::StorageError(format!("Failed to write container.json: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, container_id: &str) -> Result<()> {
        let container_dir = self.data_dir.join("containers").join(container_id);
        if container_dir.exists() {
            std::fs::remove_dir_all(&container_dir)
                .map_err(|e| CrushError::StorageError(format!("Failed to delete container dir: {}", e)))?;
        }
        Ok(())
    }

    async fn exec(&self, _container_id: &str, _command: &[String], _tty: bool) -> Result<i32> {
        Ok(0)
    }

    async fn get_pid(&self, container_id: &str) -> Result<Option<u32>> {
        let container_dir = self.data_dir.join("containers").join(container_id);
        let container_json_path = container_dir.join("container.json");
        if !container_json_path.exists() {
            return Err(CrushError::ContainerNotFound(container_id.to_string()));
        }
        let content = std::fs::read_to_string(&container_json_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read container.json: {}", e)))?;
        let c: Container = serde_json::from_str(&content)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse container.json: {}", e)))?;
        Ok(c.pid)
    }
}
