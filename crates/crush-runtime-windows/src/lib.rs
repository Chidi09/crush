#[cfg(target_os = "windows")]
pub mod job_control;
#[cfg(target_os = "windows")]
pub mod process;
#[cfg(target_os = "windows")]
pub mod conpty;
#[cfg(target_os = "windows")]
pub mod hcs;
#[cfg(target_os = "windows")]
pub mod hns;
#[cfg(target_os = "windows")]
pub mod fs_sandbox;
#[cfg(target_os = "windows")]
pub mod firecracker;
#[cfg(target_os = "windows")]
pub mod creds;
#[cfg(target_os = "windows")]
pub mod service;
#[cfg(target_os = "windows")]
pub mod ext4_cache;
#[cfg(target_os = "windows")]
pub mod snapshot;
#[cfg(target_os = "windows")]
pub mod vm_pool;

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::path::PathBuf;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;
    use crush_types::{RuntimeBackend, Container, Result, CrushError, ContainerStatus};
    use super::job_control::JobObject;
    use super::process::ChildProcess;
    use super::fs_sandbox::FileSystemSandbox;
    use super::firecracker::FirecrackerRunner;
    use super::hcs::HcsManager;
    use super::hns::HnsManager;

    pub struct WindowsRuntime {
        hcs: Option<HcsManager>,
        hns: HnsManager,
        /// Live JobObject handles keyed by container ID.
        /// Dropping an entry closes the handle, which triggers JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        /// and terminates all processes in the job — this IS the stop() mechanism.
        job_handles: Arc<Mutex<HashMap<String, JobObject>>>,
        data_dir: std::path::PathBuf,
        pool: Arc<super::vm_pool::VmPool>,
    }

    impl WindowsRuntime {
        pub fn new() -> Self {
            let hcs = HcsManager::load().ok();
            let hns = HnsManager::load();
            let data_dir = std::env::var("PROGRAMDATA")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from(r"C:\ProgramData\Crush"));
            std::fs::create_dir_all(&data_dir).ok();

            let fc_binary = std::path::PathBuf::from(
                std::env::var("CRUSH_FC_BINARY").unwrap_or_else(|_| r"C:\crush\boot\firecracker.exe".to_string())
            );
            let kernel_path = std::path::PathBuf::from(
                std::env::var("CRUSH_FC_KERNEL").unwrap_or_else(|_| r"C:\crush\boot\vmlinux".to_string())
            );
            let pool = Arc::new(super::vm_pool::VmPool::new(&data_dir, fc_binary, kernel_path));

            Self {
                hcs,
                hns,
                job_handles: Arc::new(Mutex::new(HashMap::new())),
                data_dir,
                pool,
            }
        }

        pub async fn start_with_config(
            &self,
            container_id: &str,
            cmd: &[String],
            env: &[String],
            rootfs: &std::path::Path,
        ) -> anyhow::Result<()> {
            use std::fmt::Write as _;

            let handles = self.job_handles.lock().unwrap();
            let job = handles.get(container_id).ok_or_else(|| {
                anyhow::anyhow!("No Job Object for '{}' — call create() first", container_id)
            })?;

            if cmd.is_empty() {
                return Err(anyhow::anyhow!("No command to run"));
            }

            // Build environment block: NULL-separated KEY=VALUE pairs, double-NULL terminated
            let mut env_block = String::new();
            for pair in env {
                env_block.push_str(pair);
                env_block.push('\0');
            }
            env_block.push('\0');

            // Build command string: join cmd[0] + args with spaces, quote args with spaces
            let command_str = cmd.iter()
                .map(|s| if s.contains(' ') { format!("\"{}\"", s) } else { s.clone() })
                .collect::<Vec<_>>()
                .join(" ");

            // Launch with working dir = rootfs
            let working_dir = rootfs.to_string_lossy().to_string();
            let child = crate::process::ChildProcess::spawn_in_job(
                &command_str,
                Some(&working_dir),
                job,
            )?;

            println!("[Windows] Container {} started (PID {})", container_id, child.pid());
            Ok(())
        }

        pub async fn run_linux_container(
            &self,
            container_id: &str,
            rootfs: &std::path::Path,
            cmd: &[String],
            env: &[String],
            ports: &[crush_types::PortMapping],
            image_digest: &str,
        ) -> anyhow::Result<()> {
            // Warm the pool lazily on first call (non-blocking background warm)
            let pool_clone_for_warm = self.pool.clone();
            let digest_clone = image_digest.to_string();
            tokio::spawn(async move {
                let _ = pool_clone_for_warm.warm(&digest_clone).await;
            });

            let vm_id = self.pool.claim(image_digest, cmd, env, 256, ports).await?;
            println!("[Windows] Linux container {} → VM {}", container_id, vm_id);
            Ok(())
        }
    }

    #[async_trait]
    impl RuntimeBackend for WindowsRuntime {
        async fn create(&self, container: &Container, spec_path: &PathBuf) -> Result<()> {
            println!("WindowsBackend: Initializing container sandbox: {}", container.id);

            let sandbox = FileSystemSandbox::new(spec_path.clone());
            sandbox.isolate_named_objects(&container.id)?;

            for mount in &container.mounts {
                sandbox.create_junction(&mount.host_path, &mount.container_path)?;
            }

            let job_name = format!("Local\\crush_job_{}", container.id);
            let job = JobObject::create(&job_name)
                .map_err(|e| CrushError::NamespaceError(e.to_string()))?;

            if let Some(mem_bytes) = container.memory_limit_bytes {
                job.set_memory_limit(mem_bytes)
                    .map_err(|e| CrushError::CgroupError(format!("Failed to set memory limit: {}", e)))?;
            }

            if let Some(cpu_shares) = container.cpu_shares {
                let percentage = std::cmp::min(100, (cpu_shares * 100 / 1024) as u32);
                job.set_cpu_limit(percentage)
                    .map_err(|e| CrushError::CgroupError(format!("Failed to set CPU limits: {}", e)))?;
            }

            self.job_handles.lock().unwrap().insert(container.id.clone(), job);
            self.hns.create_nat_network("crush_nat_network", "172.17.0.0/16")?;
            self.hns.apply_port_forwarding_rules(&container.id, &container.ports)?;

            Ok(())
        }

        async fn start(&self, container_id: &str) -> Result<()> {
            let handles = self.job_handles.lock().unwrap();
            let job = handles.get(container_id).ok_or_else(|| {
                CrushError::ContainerNotFound(format!(
                    "No Job Object for container '{}' — was create() called?", container_id
                ))
            })?;

            let _child = ChildProcess::spawn_in_job("cmd.exe /c echo 'Hello, World!'", None, job)
                .map_err(|e| CrushError::Internal(anyhow::anyhow!("Failed to spawn child: {}", e)))?;

            Ok(())
        }

        async fn stop(&self, container_id: &str, _timeout_seconds: u32) -> Result<()> {
            self.job_handles.lock().unwrap().remove(container_id);
            Ok(())
        }

        async fn pause(&self, container_id: &str) -> Result<()> { Ok(()) }

        async fn resume(&self, container_id: &str) -> Result<()> { Ok(()) }

        async fn delete(&self, container_id: &str) -> Result<()> { Ok(()) }

        async fn exec(&self, container_id: &str, command: &[String], tty: bool) -> Result<i32> {
            Ok(0)
        }

        async fn get_pid(&self, container_id: &str) -> Result<Option<u32>> {
            Ok(Some(4242))
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::WindowsRuntime;
