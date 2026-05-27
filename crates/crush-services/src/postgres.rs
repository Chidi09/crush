use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::time::Duration;

use crate::driver::{ServiceDriver, RunningService, ServiceConfig, ServiceKind};
use crate::binary_cache::BinaryCache;

pub struct PostgresDriver {
    pub cache: BinaryCache,
}

impl PostgresDriver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache: BinaryCache::new(cache_dir) }
    }

    pub fn find_pg_ctl() -> Option<PathBuf> {
        // Check PATH first
        let probe = if cfg!(target_os = "windows") { "pg_ctl.exe" } else { "pg_ctl" };
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

        // Windows: scan common EDB install locations
        #[cfg(target_os = "windows")]
        for major in &["17", "16", "15", "14", "13"] {
            let candidate = PathBuf::from(format!(
                r"C:\Program Files\PostgreSQL\{}\bin\pg_ctl.exe",
                major
            ));
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }

    fn is_initialized(data_dir: &Path) -> bool {
        data_dir.join("PG_VERSION").exists()
    }
}

#[async_trait]
impl ServiceDriver for PostgresDriver {
    fn name(&self) -> &'static str { "postgres" }
    fn default_port(&self) -> u16 { 5432 }

    async fn ensure_ready(&self, _data_dir: &Path, _cache_dir: &Path) -> Result<()> {
        if Self::find_pg_ctl().is_none() {
            #[cfg(target_os = "windows")]
            anyhow::bail!(
                "PostgreSQL not found.\n  Install: winget install PostgreSQL.PostgreSQL\n  Or download from: https://www.enterprisedb.com/downloads/postgres-postgresql-downloads"
            );
            #[cfg(not(target_os = "windows"))]
            anyhow::bail!(
                "PostgreSQL not found.\n  Ubuntu/Debian: sudo apt install postgresql\n  macOS: brew install postgresql@16"
            );
        }
        Ok(())
    }

    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService> {
        let pg_ctl = Self::find_pg_ctl()
            .context("pg_ctl not found — install PostgreSQL first")?;

        fs::create_dir_all(data_dir).context("Failed to create Postgres data directory")?;

        // Initialise cluster on first run
        if !Self::is_initialized(data_dir) {
            let password = config.password.clone().unwrap_or_else(|| "postgres".to_string());
            let pwfile = data_dir.join(".crush_initpw");
            fs::write(&pwfile, &password)
                .context("Failed to write initdb password file")?;

            let status = tokio::process::Command::new(&pg_ctl)
                .args([
                    "initdb",
                    "-D", &data_dir.to_string_lossy(),
                    "-o", &format!(
                        "--pwfile={} --username=postgres --auth=md5",
                        pwfile.display()
                    ),
                    "--no-instructions",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await
                .context("Failed to run pg_ctl initdb")?;

            let _ = fs::remove_file(&pwfile);

            if !status.success() {
                anyhow::bail!("PostgreSQL cluster initialisation failed (pg_ctl initdb returned non-zero)");
            }
        }

        // Start server; -w waits until ready (up to -t 30 seconds)
        let log_file = data_dir.join("crush_pg.log");
        let port_opt = format!("-p {}", config.port);

        let status = tokio::process::Command::new(&pg_ctl)
            .args([
                "start",
                "-D", &data_dir.to_string_lossy(),
                "-l", &log_file.to_string_lossy(),
                "-o", &port_opt,
                "-w",
                "-t", "30",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .context("Failed to run pg_ctl start")?;

        if !status.success() {
            anyhow::bail!(
                "PostgreSQL failed to start — check log at {}",
                log_file.display()
            );
        }

        // Read PID from postmaster.pid (written by Postgres after -w succeeds)
        let pid = fs::read_to_string(data_dir.join("postmaster.pid"))
            .ok()
            .and_then(|s| s.lines().next()?.trim().parse::<u32>().ok())
            .unwrap_or(0);

        Ok(RunningService {
            name: "postgres".to_string(),
            pid,
            port: config.port,
            data_dir: data_dir.to_path_buf(),
            kind: ServiceKind::Postgres,
        })
    }

    async fn stop(&self, service: &RunningService) -> Result<()> {
        if let Some(pg_ctl) = Self::find_pg_ctl() {
            let _ = tokio::process::Command::new(&pg_ctl)
                .args([
                    "stop",
                    "-D", &service.data_dir.to_string_lossy(),
                    "-m", "fast",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        } else if service.pid > 0 {
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &service.pid.to_string()])
                .status();
            #[cfg(not(target_os = "windows"))]
            let _ = std::process::Command::new("kill")
                .arg(service.pid.to_string())
                .status();
        }
        Ok(())
    }

    async fn is_alive(&self, service: &RunningService) -> bool {
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", service.port))
            .await
            .is_ok()
    }

    async fn wait_ready(&self, service: &RunningService, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", service.port))
                .await
                .is_ok()
            {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }
}
