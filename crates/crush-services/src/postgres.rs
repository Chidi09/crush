use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::time::Duration;

use crate::driver::{ServiceDriver, RunningService, ServiceConfig, ServiceKind};
use crate::binary_cache::{BinaryCache, BinarySpec, ArchiveType};

// EDB portable binaries — plain zip, no installer required.
// Extracts to pgsql/bin/pg_ctl.exe (and friends).
#[cfg(target_os = "windows")]
const POSTGRES_PORTABLE_VERSION: &str = "17.5-1";

#[cfg(target_os = "windows")]
static POSTGRES_PORTABLE_SPEC: BinarySpec = BinarySpec {
    service: "postgres",
    version: "17.5-1",
    url: "https://get.enterprisedb.com/postgresql/postgresql-17.5-1-windows-x64-binaries.zip",
    sha256: "",
    archive_type: ArchiveType::Zip,
};

pub struct PostgresDriver {
    pub cache: BinaryCache,
}

impl PostgresDriver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache: BinaryCache::new(cache_dir) }
    }

    // Check PATH first, then common install locations, then crush's own cache.
    pub fn find_pg_ctl_with_cache(&self) -> Option<PathBuf> {
        if let Some(p) = Self::find_pg_ctl() {
            return Some(p);
        }
        // Cached portable install (Windows only)
        #[cfg(target_os = "windows")]
        {
            let cached = self.cache.root
                .join("postgres")
                .join(POSTGRES_PORTABLE_VERSION)
                .join("pgsql")
                .join("bin")
                .join("pg_ctl.exe");
            if cached.exists() {
                return Some(cached);
            }
        }
        None
    }

    pub fn find_pg_ctl() -> Option<PathBuf> {
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
        if self.find_pg_ctl_with_cache().is_none() {
            #[cfg(target_os = "windows")]
            {
                println!("   ↳ PostgreSQL not found — downloading portable binaries (~30 MB)...");
                self.cache.ensure(&POSTGRES_PORTABLE_SPEC).await
                    .context("Failed to download portable PostgreSQL")?;
                println!("   ↳ PostgreSQL ready.");
            }
            #[cfg(not(target_os = "windows"))]
            anyhow::bail!(
                "PostgreSQL not found.\n  Ubuntu/Debian: sudo apt install postgresql\n  macOS: brew install postgresql@17"
            );
        }
        Ok(())
    }

    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService> {
        let pg_ctl = self.find_pg_ctl_with_cache()
            .context("pg_ctl not found — run crush again to re-download")?;

        fs::create_dir_all(data_dir).context("Failed to create Postgres data directory")?;

        if !Self::is_initialized(data_dir) {
            // A previous failed init may have left a non-empty dir without PG_VERSION.
            // initdb refuses to work on non-empty dirs, so clean it first.
            if data_dir.exists() {
                fs::remove_dir_all(data_dir).context("Failed to clean partial Postgres data dir")?;
            }
            fs::create_dir_all(data_dir).context("Failed to create Postgres data directory")?;

            let password = config.password.clone().unwrap_or_else(|| "postgres".to_string());
            let username = config.user.clone().unwrap_or_else(|| "postgres".to_string());
            // Write the pwfile to the PARENT of data_dir — initdb requires the
            // data directory to be completely empty.
            let pwfile = data_dir.parent()
                .unwrap_or(data_dir)
                .join(".crush_initpw");
            fs::write(&pwfile, &password)
                .context("Failed to write initdb password file")?;

            // Call initdb directly (same bin/ dir as pg_ctl) so each arg is
            // passed cleanly — no quoting issues with paths that contain spaces.
            let initdb = pg_ctl.parent()
                .map(|p| p.join(if cfg!(target_os = "windows") { "initdb.exe" } else { "initdb" }))
                .unwrap_or_else(|| PathBuf::from("initdb"));

            let output = tokio::process::Command::new(&initdb)
                .args([
                    "-D", &data_dir.to_string_lossy(),
                    "--pwfile", &pwfile.to_string_lossy(),
                    "--username", &username,
                    "--auth", "md5",
                    "--no-instructions",
                ])
                .stdout(std::process::Stdio::null())
                .output()
                .await
                .context("Failed to run initdb")?;

            let _ = fs::remove_file(&pwfile);

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("PostgreSQL cluster initialisation failed:\n{}", stderr.trim());
            }
        }

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
            // pg_ctl returns non-zero when the server is already running.
            // Verify by attempting a TCP connection before treating it as failure.
            let already_up = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", config.port))
                .await
                .is_ok();
            if !already_up {
                anyhow::bail!(
                    "PostgreSQL failed to start — check log at {}",
                    log_file.display()
                );
            }
        }

        let pid = fs::read_to_string(data_dir.join("postmaster.pid"))
            .ok()
            .and_then(|s| s.lines().next()?.trim().parse::<u32>().ok())
            .unwrap_or(0);

        let username = config.user.clone().unwrap_or_else(|| "postgres".to_string());
        let password = config.password.clone().unwrap_or_default();
        let psql = pg_ctl.parent()
            .map(|p| p.join(if cfg!(target_os = "windows") { "psql.exe" } else { "psql" }))
            .unwrap_or_else(|| PathBuf::from("psql"));

        // Bring the cluster into agreement with the requested credentials.
        // The data dir may have been initdb'd long ago with different creds —
        // earlier crush versions ran initdb as `postgres/postgres` regardless
        // of application.yml. Run an idempotent CREATE/ALTER USER as whichever
        // superuser actually exists.
        let target_role = format!(
            "DO $$ BEGIN \
                IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '{u}') THEN \
                    CREATE ROLE \"{u}\" LOGIN SUPERUSER PASSWORD '{p}'; \
                ELSE \
                    ALTER ROLE \"{u}\" WITH LOGIN SUPERUSER PASSWORD '{p}'; \
                END IF; END $$;",
            u = username.replace('\'', "''"),
            p = password.replace('\'', "''"),
        );

        // Try the configured user first; if its password no longer matches,
        // fall back to `postgres/postgres` (the previous default), then to
        // peer auth on Unix sockets via the OS user `postgres`.
        let candidates = [
            (username.clone(), password.clone()),
            ("postgres".to_string(), "postgres".to_string()),
            ("postgres".to_string(), String::new()),
        ];
        for (try_user, try_pass) in &candidates {
            let status = tokio::process::Command::new(&psql)
                .args([
                    "-h", "localhost",
                    "-p", &config.port.to_string(),
                    "-U", try_user,
                    "-d", "postgres",
                    "-c", &target_role,
                ])
                .env("PGPASSWORD", try_pass)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
            if let Ok(s) = status { if s.success() { break; } }
        }

        // Ensure the database exists (idempotent CREATE DATABASE).
        if let Some(ref db) = config.database {
            let target_db = format!(
                "SELECT 'CREATE DATABASE \"{d}\" OWNER \"{u}\"' \
                 WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = '{d}')\\gexec",
                d = db.replace('"', ""),
                u = username.replace('"', ""),
            );
            let _ = tokio::process::Command::new(&psql)
                .args([
                    "-h", "localhost",
                    "-p", &config.port.to_string(),
                    "-U", &username,
                    "-d", "postgres",
                    "-c", &target_db,
                ])
                .env("PGPASSWORD", &password)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }

        Ok(RunningService {
            name: "postgres".to_string(),
            pid,
            port: config.port,
            data_dir: data_dir.to_path_buf(),
            kind: ServiceKind::Postgres,
        })
    }

    async fn stop(&self, service: &RunningService) -> Result<()> {
        if let Some(pg_ctl) = self.find_pg_ctl_with_cache() {
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
