use std::path::{Path, PathBuf};
use std::fs::{self};
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::time::Duration;

use crate::driver::{ServiceDriver, RunningService, ServiceConfig, ServiceKind};
use crate::binary_cache::{BinaryCache, BinarySpec, ArchiveType};

/// Best-effort prefetch: download the Garnet / Valkey binary into the cache
/// without starting the server. Called early from the CLI so the binary is on
/// disk by the time `start_dep_service_smart` runs. Errors are silently ignored
/// — the real start path will re-download on a miss.
pub async fn prefetch(cache_dir: PathBuf) -> Result<()> {
    let cache = BinaryCache::new(cache_dir);
    #[cfg(target_os = "windows")]
    { let _ = cache.ensure(&GARNET_SPEC).await; }
    #[cfg(not(target_os = "windows"))]
    { let _ = cache.ensure(&VALKEY_SPEC).await; }
    Ok(())
}

#[cfg(target_os = "windows")]
static GARNET_SPEC: BinarySpec = BinarySpec {
    service: "garnet",
    version: "1.1.9",
    url: "https://github.com/microsoft/garnet/releases/download/v1.1.9/win-x64-based-readytorun.zip",
    sha256: "",
    archive_type: ArchiveType::Zip,
};

#[cfg(not(target_os = "windows"))]
static VALKEY_SPEC: BinarySpec = BinarySpec {
    service: "valkey",
    version: "7.2.5",
    url: "https://github.com/valkey-io/valkey/releases/download/7.2.5/valkey-7.2.5-linux-x86_64.tar.gz",
    sha256: "",
    archive_type: ArchiveType::TarGz,
};

pub struct RedisCompatDriver {
    cache: BinaryCache,
}

impl RedisCompatDriver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache: BinaryCache::new(cache_dir),
        }
    }

    fn find_system_server(&self) -> Option<String> {
        for cmd in &["valkey-server", "redis-server"] {
            let mut command = std::process::Command::new(cmd);
            command.arg("--version");
            command.stdout(std::process::Stdio::null());
            command.stderr(std::process::Stdio::null());
            if let Ok(status) = command.status() {
                if status.success() {
                    return Some(cmd.to_string());
                }
            }
        }
        None
    }

    fn find_binary_in_dir(&self, dir: &Path, name: &str) -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(p) = self.find_binary_in_dir(&path, name) {
                        return Some(p);
                    }
                } else if path.file_name().and_then(|s| s.to_str()) == Some(name) {
                    return Some(path);
                }
            }
        }
        None
    }

    fn get_executable_path(&self, dest_dir: &Path) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            // Microsoft Garnet's win-x64 zip ships GarnetServer.exe at the
            // archive root; older single-binary releases used garnet.exe.
            for name in &["GarnetServer.exe", "garnet.exe"] {
                let exe = dest_dir.join(name);
                if exe.exists() { return Some(exe); }
            }
            // Some packaging variants nest the binary under a subdir.
            if let Some(p) = self.find_binary_in_dir(dest_dir, "GarnetServer.exe") {
                return Some(p);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(valkey) = self.find_binary_in_dir(dest_dir, "valkey-server") {
                return Some(valkey);
            }
            if let Some(redis) = self.find_binary_in_dir(dest_dir, "redis-server") {
                return Some(redis);
            }
        }
        None
    }
}

#[async_trait]
impl ServiceDriver for RedisCompatDriver {
    fn name(&self) -> &'static str { "redis" }
    fn default_port(&self) -> u16 { 6379 }

    async fn ensure_ready(&self, _data_dir: &Path, _cache_dir: &Path) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let _ = self.cache.ensure(&GARNET_SPEC).await?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            if self.find_system_server().is_none() {
                let _ = self.cache.ensure(&VALKEY_SPEC).await?;
            }
        }
        Ok(())
    }

    async fn start(&self, config: &ServiceConfig, data_dir: &Path) -> Result<RunningService> {
        #[cfg(target_os = "windows")]
        let (bin_path, is_system) = {
            let dest_dir = self.cache.root.join("garnet").join(GARNET_SPEC.version);
            let path = self.get_executable_path(&dest_dir)
                .context("Microsoft Garnet binary not found in cache")?;
            (path, false)
        };

        #[cfg(not(target_os = "windows"))]
        let (bin_path, is_system) = {
            if let Some(sys_cmd) = self.find_system_server() {
                (PathBuf::from(sys_cmd), true)
            } else {
                let dest_dir = self.cache.root.join("valkey").join(VALKEY_SPEC.version);
                let path = self.get_executable_path(&dest_dir)
                    .context("Valkey binary not found in cache")?;
                
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

        let mut cmd = tokio::process::Command::new(bin_path);
        cmd.arg("--port").arg(config.port.to_string());

        if cfg!(target_os = "windows") && !is_system {
            // Microsoft Garnet authentication
            if let Some(ref pw) = config.password {
                cmd.arg("--auth").arg("Password").arg("--password").arg(pw);
            }
        } else {
            // Valkey/Redis authentication
            if let Some(ref pw) = config.password {
                cmd.arg("--requirepass").arg(pw);
            }
        }

        // Set standard env variables
        for (k, v) in &config.extra_env {
            cmd.env(k, v);
        }

        cmd.current_dir(data_dir);
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

        let child = cmd.spawn().context("Failed to start Redis-compat process")?;
        let pid = child.id().unwrap_or(0);

        Ok(RunningService {
            name: "redis".to_string(),
            pid,
            port: config.port,
            data_dir: data_dir.to_path_buf(),
            kind: ServiceKind::RedisCompat,
            console_port: None,
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
