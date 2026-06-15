use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::time::Duration;
use crate::driver::{ServiceDriver, RunningService, ServiceConfig, ServiceKind};

pub struct MysqlDriver {}

impl MysqlDriver {
    pub fn new() -> Self {
        Self {}
    }

    pub fn find_mysqld() -> Option<PathBuf> {
        let probe = if cfg!(target_os = "windows") { "mysqld.exe" } else { "mysqld" };
        if std::process::Command::new(probe)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(probe));
        }
        
        #[cfg(target_os = "windows")]
        for major in &["8.0", "8.1", "8.4"] {
            let candidate = PathBuf::from(format!(
                r"C:\Program Files\MySQL\MySQL Server {}\bin\mysqld.exe",
                major
            ));
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }

    fn is_initialized(data_dir: &Path) -> bool {
        data_dir.join("mysql").exists()
    }
}

#[async_trait]
impl ServiceDriver for MysqlDriver {
    fn name(&self) -> &'static str { "mysql" }
    fn default_port(&self) -> u16 { 3306 }

    async fn ensure_ready(&self, _data_dir: &Path, _cache_dir: &Path) -> Result<()> {
        if Self::find_mysqld().is_none() {
            anyhow::bail!("mysqld not found on PATH. Please install MySQL Server.");
        }
        Ok(())
    }

    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService> {
        let mysqld = Self::find_mysqld()
            .context("mysqld not found")?;

        if !Self::is_initialized(data_dir) {
            fs::create_dir_all(data_dir)?;
            let mut cmd = tokio::process::Command::new(&mysqld);
            cmd.arg("--initialize-insecure")
               .arg(format!("--datadir={}", data_dir.display()));
            
            let status = cmd.status().await?;
            if !status.success() {
                anyhow::bail!("Failed to initialize MySQL data directory");
            }
        }

        let mut cmd = tokio::process::Command::new(&mysqld);
        cmd.arg(format!("--datadir={}", data_dir.display()))
           .arg(format!("--port={}", config.port));

        if let Some(log) = &config.log_file {
            let log_file = std::fs::File::create(log)?;
            cmd.stdout(std::process::Stdio::from(log_file.try_clone()?));
            cmd.stderr(std::process::Stdio::from(log_file));
        } else {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        let child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);
        
        // Spawn a background task to await the child so it doesn't zombie
        tokio::spawn(async move {
            let mut c = child;
            let _ = c.wait().await;
        });

        Ok(RunningService {
            name: "mysql".into(),
            pid,
            port: config.port,
            data_dir: data_dir.to_path_buf(),
            kind: ServiceKind::MySQL,
            console_port: None,
        })
    }

    async fn stop(&self, service: &RunningService) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &service.pid.to_string(), "/F", "/T"])
                .status();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("kill")
                .arg("-15")
                .arg(service.pid.to_string())
                .status();
        }
        Ok(())
    }

    async fn is_alive(&self, service: &RunningService) -> bool {
        // Portable + dependency-free: a live mysqld accepts TCP connections on
        // its port. More meaningful than a PID check for a DB service, and avoids
        // a platform-specific process API (the previous Windows path pulled in an
        // undeclared `sysinfo`, which only broke on the Windows cross-compile).
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", service.port))
            .await
            .is_ok()
    }

    async fn wait_ready(&self, service: &RunningService, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let addr = format!("127.0.0.1:{}", service.port);

        while start.elapsed() < timeout {
            if tokio::net::TcpStream::connect(&addr).await.is_ok() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }
}
