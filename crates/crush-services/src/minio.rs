use std::path::{Path, PathBuf};
use std::fs::{self};
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::time::Duration;

use crate::driver::{ServiceDriver, RunningService, ServiceConfig, ServiceKind};
use crate::binary_cache::{BinaryCache, BinarySpec, ArchiveType};

#[cfg(target_os = "windows")]
static MINIO_SPEC: BinarySpec = BinarySpec {
    service: "minio",
    version: "latest",
    url: "https://dl.min.io/server/minio/release/windows-amd64/minio.exe",
    sha256: "",
    archive_type: ArchiveType::Exe,
};

#[cfg(not(target_os = "windows"))]
static MINIO_SPEC: BinarySpec = BinarySpec {
    service: "minio",
    version: "latest",
    url: "https://dl.min.io/server/minio/release/linux-amd64/minio",
    sha256: "",
    archive_type: ArchiveType::Exe,
};

pub struct MinioDriver {
    cache: BinaryCache,
}

impl MinioDriver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache: BinaryCache::new(cache_dir),
        }
    }

    fn find_system_minio(&self) -> Option<String> {
        let mut command = std::process::Command::new("minio");
        command.arg("--version");
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
        if let Ok(status) = command.status() {
            if status.success() {
                return Some("minio".to_string());
            }
        }
        None
    }

    fn get_executable_path(&self, dest_dir: &Path) -> Option<PathBuf> {
        let exe_name = if cfg!(target_os = "windows") {
            "minio.exe"
        } else {
            "minio"
        };
        let path = dest_dir.join(exe_name);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

#[async_trait]
impl ServiceDriver for MinioDriver {
    fn name(&self) -> &'static str { "minio" }
    fn default_port(&self) -> u16 { 9000 }

    async fn ensure_ready(&self, _data_dir: &Path, _cache_dir: &Path) -> Result<()> {
        if self.find_system_minio().is_none() {
            let _ = self.cache.ensure(&MINIO_SPEC).await?;
        }
        Ok(())
    }

    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService> {
        let (bin_path, _is_system) = {
            if let Some(sys_cmd) = self.find_system_minio() {
                (PathBuf::from(sys_cmd), true)
            } else {
                let dest_dir = self.cache.root.join("minio").join(MINIO_SPEC.version);
                let path = self.get_executable_path(&dest_dir)
                    .context("MinIO binary not found in cache")?;

                // Ensure executable permissions on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(&path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&path, perms);
                    }
                }
                (path, false)
            }
        };

        // Ensure the data directory exists
        fs::create_dir_all(data_dir).context("Failed to create MinIO data directory")?;

        let console_port = config.port + 1;

        let mut cmd = tokio::process::Command::new(bin_path);
        cmd.arg("server")
            .arg(data_dir)
            .arg("--address")
            .arg(format!(":{}", config.port))
            .arg("--console-address")
            .arg(format!(":{}", console_port));

        // Configure credentials
        let user = config.user.clone().unwrap_or_else(|| "minioadmin".to_string());
        let password = config.password.clone().unwrap_or_else(|| "minioadmin".to_string());
        cmd.env("MINIO_ROOT_USER", &user);
        cmd.env("MINIO_ROOT_PASSWORD", &password);

        // Apply extra environments
        for (k, v) in &config.extra_env {
            cmd.env(k, v);
        }

        cmd.current_dir(data_dir);

        // Log redirect setup
        if let Some(ref log_path) = config.log_file {
            if let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) {
                if let Ok(dup) = file.try_clone() {
                    cmd.stdout(std::process::Stdio::from(file));
                    cmd.stderr(std::process::Stdio::from(dup));
                } else {
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());
                }
            } else {
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
            }
        } else {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        let child = cmd.spawn().context("Failed to start MinIO process")?;
        let pid = child.id().unwrap_or(0);

        Ok(RunningService {
            name: "minio".to_string(),
            pid,
            port: config.port,
            data_dir: data_dir.to_path_buf(),
            kind: ServiceKind::ObjectStore,
            console_port: Some(console_port),
        })
    }

    async fn stop(&self, service: &RunningService) -> Result<()> {
        if service.pid > 0 {
            #[cfg(target_os = "windows")]
            {
                let mut cmd = std::process::Command::new("taskkill");
                cmd.args(["/F", "/PID", &service.pid.to_string()]);
                let _ = cmd.status();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let mut cmd = std::process::Command::new("kill");
                cmd.arg(service.pid.to_string());
                let _ = cmd.status();
            }
        }
        Ok(())
    }

    async fn is_alive(&self, service: &RunningService) -> bool {
        if service.pid == 0 {
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            let mut cmd = std::process::Command::new("tasklist");
            cmd.args(["/FI", &format!("PID eq {}", service.pid)]);
            if let Ok(output) = cmd.output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&service.pid.to_string())
            } else {
                false
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut cmd = std::process::Command::new("kill");
            cmd.arg("-0").arg(service.pid.to_string());
            if let Ok(status) = cmd.status() {
                status.success()
            } else {
                false
            }
        }
    }

    async fn wait_ready(&self, service: &RunningService, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", service.port)).await.is_ok() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }
}
