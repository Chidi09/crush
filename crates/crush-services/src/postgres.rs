use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::time::Duration;

use pg_embed::postgres::{PgEmbed, PgSettings};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V16};
use pg_embed::pg_enums::PgAuthMethod;

use crate::driver::{ServiceDriver, RunningService, ServiceConfig, ServiceKind};
use crate::binary_cache::{BinaryCache, BinarySpec, ArchiveType};

#[cfg(target_os = "windows")]
static PGVECTOR_SPEC: BinarySpec = BinarySpec {
    service: "pgvector",
    version: "0.7.4",
    url: "https://github.com/crush-runtime/crush/releases/download/pgvector-windows/pgvector-0.7.4-pg16-windows-x64.zip",
    sha256: "",
    archive_type: ArchiveType::Zip,
};

pub struct PostgresDriver {
    cache: BinaryCache,
}

impl PostgresDriver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache: BinaryCache::new(cache_dir),
        }
    }

    fn get_pg_embed_cache_dir(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                return PathBuf::from(local_app_data).join("pg-embed");
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join("Library").join("Caches").join("pg-embed");
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
                return PathBuf::from(xdg_cache).join("pg-embed");
            } else if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(".cache").join("pg-embed");
            }
        }
        PathBuf::from(".cache").join("pg-embed")
    }

    fn find_pg_home(&self) -> Option<PathBuf> {
        let base_cache = self.get_pg_embed_cache_dir();
        self.find_postgres_bin(&base_cache)
    }

    fn find_postgres_bin(&self, dir: &Path) -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().and_then(|s| s.to_str()) == Some("bin") {
                        let pg_exe = path.join("postgres");
                        let pg_exe_win = path.join("postgres.exe");
                        let pg_exe_win_alt = path.join("postgresql.exe");
                        if pg_exe.exists() || pg_exe_win.exists() || pg_exe_win_alt.exists() {
                            return path.parent().map(|p| p.to_path_buf());
                        }
                    } else if let Some(res) = self.find_postgres_bin(&path) {
                        return Some(res);
                    }
                }
            }
        }
        None
    }

    fn find_files(&self, dir: &Path, extension: &str) -> Vec<PathBuf> {
        let mut results = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    results.extend(self.find_files(&path, extension));
                } else if path.extension().and_then(|s| s.to_str()) == Some(extension) {
                    results.push(path);
                }
            }
        }
        results
    }

    async fn install_pgvector_extension(&self, pg_home: &Path, pgvector_unpack_dir: &Path) -> Result<()> {
        let lib_dir = pg_home.join("lib");
        let ext_dir = pg_home.join("share").join("postgresql").join("extension");

        fs::create_dir_all(&lib_dir).ok();
        fs::create_dir_all(&ext_dir).ok();

        // 1. Copy library files (.dll, .so, .dylib)
        let lib_ext = if cfg!(target_os = "windows") {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };

        let lib_files = self.find_files(pgvector_unpack_dir, lib_ext);
        for f in lib_files {
            if let Some(name) = f.file_name() {
                let dest = lib_dir.join(name);
                fs::copy(&f, &dest).context(format!("Failed to copy library file {:?}", f))?;
            }
        }

        // 2. Copy extension metadata files (.control and .sql)
        let control_files = self.find_files(pgvector_unpack_dir, "control");
        for f in control_files {
            if let Some(name) = f.file_name() {
                let dest = ext_dir.join(name);
                fs::copy(&f, &dest).context(format!("Failed to copy control file {:?}", f))?;
            }
        }

        let sql_files = self.find_files(pgvector_unpack_dir, "sql");
        for f in sql_files {
            if let Some(name) = f.file_name() {
                let dest = ext_dir.join(name);
                fs::copy(&f, &dest).context(format!("Failed to copy sql file {:?}", f))?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ServiceDriver for PostgresDriver {
    fn name(&self) -> &'static str { "postgres" }
    fn default_port(&self) -> u16 { 5432 }

    async fn ensure_ready(&self, _data_dir: &Path, _cache_dir: &Path) -> Result<()> {
        // pg_embed downloads postgres itself on demand.
        // We only download pgvector on Windows.
        #[cfg(target_os = "windows")]
        {
            let unpack_dir = self.cache.ensure(&PGVECTOR_SPEC).await?;
            if let Some(pg_home) = self.find_pg_home() {
                self.install_pgvector_extension(&pg_home, &unpack_dir).await?;
            } else {
                // If Postgres isn't fetched yet, we'll download Postgres first
                // by initializing pg_embed briefly or we install pgvector on start.
            }
        }
        Ok(())
    }

    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService> {
        let pg_settings = PgSettings {
            database_dir: data_dir.to_path_buf(),
            port: config.port,
            user: "postgres".to_string(),
            password: config.password.clone().unwrap_or_else(|| "postgres".to_string()),
            auth_method: PgAuthMethod::Plain,
            persistent: true,
            timeout: Some(Duration::from_secs(15)),
            migration_dir: None,
        };

        let fetch_settings = PgFetchSettings {
            version: PG_V16,
            ..Default::default()
        };

        // Initialize pgvector if it wasn't done during ensure_ready because pg_home was missing
        let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
        pg.setup().await?;

        #[cfg(target_os = "windows")]
        {
            if let Some(pg_home) = self.find_pg_home() {
                let unpack_dir = self.cache.root.join("pgvector").join(PGVECTOR_SPEC.version);
                if unpack_dir.exists() {
                    let _ = self.install_pgvector_extension(&pg_home, &unpack_dir).await;
                }
            }
        }

        pg.start_db().await?;

        // Extract PID from lock file postmaster.pid inside data_dir
        let mut pid = 0u32;
        let pid_file = data_dir.join("postmaster.pid");
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if pid_file.exists() {
                if let Ok(content) = fs::read_to_string(&pid_file) {
                    if let Some(line) = content.lines().next() {
                        if let Ok(p) = line.trim().parse::<u32>() {
                            pid = p;
                            break;
                        }
                    }
                }
            }
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

        // Clean up the postmaster.pid file so it starts cleanly next time
        let pid_file = service.data_dir.join("postmaster.pid");
        if pid_file.exists() {
            let _ = fs::remove_file(pid_file);
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
