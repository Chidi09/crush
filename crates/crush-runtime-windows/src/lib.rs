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
mod windows_impl {
    use std::path::PathBuf;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;
    use crush_types::{RuntimeBackend, Container, Result, CrushError, ContainerStatus};
    use super::job_control::JobObject;
    use super::process::ChildProcess;
    use super::fs_sandbox::FileSystemSandbox;
    use super::firecracker::FirecrackerMicroVM;
    use super::hcs::HcsManager;
    use super::hns::HnsManager;

    pub struct WindowsRuntime {
        hcs: Option<HcsManager>,
        hns: HnsManager,
        /// Live JobObject handles keyed by container ID.
        /// Dropping an entry closes the handle, which triggers JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        /// and terminates all processes in the job — this IS the stop() mechanism.
        job_handles: Arc<Mutex<HashMap<String, JobObject>>>,
    }

    impl WindowsRuntime {
        pub fn new() -> Self {
            let hcs = HcsManager::load().ok();
            let hns = HnsManager::load();
            Self { hcs, hns, job_handles: Arc::new(Mutex::new(HashMap::new())) }
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
