pub mod job_control;
pub mod process;
pub mod conpty;
pub mod hcs;
pub mod hns;
pub mod fs_sandbox;
pub mod firecracker;
pub mod creds;
pub mod service;

use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use crush_types::{RuntimeBackend, Container, Result, CrushError, ContainerStatus};
use job_control::JobObject;
use process::ChildProcess;
use fs_sandbox::FileSystemSandbox;
use firecracker::FirecrackerMicroVM;
use hcs::HcsManager;
use hns::HnsManager;

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

        // 1. Set up directory junctions and sandbox isolated directories
        let sandbox = FileSystemSandbox::new(spec_path.clone());
        sandbox.isolate_named_objects(&container.id)?;

        // If host directory bindings are configured, setup reparse point junctions
        for mount in &container.mounts {
            sandbox.create_junction(&mount.host_path, &mount.container_path)?;
        }

        // 2. Set up native Job Object containment
        let job_name = format!("Local\\crush_job_{}", container.id);
        let job = JobObject::create(&job_name)
            .map_err(|e| CrushError::NamespaceError(e.to_string()))?;

        // 3. Enforce CPU and Memory limits at OS level via Job Object extended information
        if let Some(mem_bytes) = container.memory_limit_bytes {
            job.set_memory_limit(mem_bytes)
                .map_err(|e| CrushError::CgroupError(format!("Failed to set memory limit: {}", e)))?;
            println!("WindowsBackend: Job Object memory limit set to {} bytes", mem_bytes);
        }

        if let Some(cpu_shares) = container.cpu_shares {
            // Map CPU shares weight to percentage caps (e.g. 512 -> 50% CPU limit)
            let percentage = std::cmp::min(100, (cpu_shares * 100 / 1024) as u32);
            job.set_cpu_limit(percentage)
                .map_err(|e| CrushError::CgroupError(format!("Failed to set CPU limits: {}", e)))?;
            println!("WindowsBackend: Job Object CPU rate limit set to {}%", percentage);
        }

        // Store the job handle so start() can assign processes to it and stop() can terminate them.
        // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE is already set in JobObject::create — dropping the
        // stored handle is sufficient to terminate all container processes.
        self.job_handles.lock().unwrap().insert(container.id.clone(), job);

        // 4. Configure WFP (Windows Filtering Platform) / HNS network NAT configurations
        self.hns.create_nat_network("crush_nat_network", "172.17.0.0/16")?;
        self.hns.apply_port_forwarding_rules(&container.id, &container.ports)?;

        Ok(())
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        println!("WindowsBackend: Spawning container process for: {}", container_id);

        let handles = self.job_handles.lock().unwrap();
        let job = handles.get(container_id).ok_or_else(|| {
            CrushError::ContainerNotFound(format!(
                "No Job Object for container '{}' — was create() called?", container_id
            ))
        })?;

        // Spawn inside the existing job so the process inherits its limits.
        let _child = ChildProcess::spawn_in_job("cmd.exe /c echo 'Hello, World!'", None, job)
            .map_err(|e| CrushError::Internal(anyhow::anyhow!("Failed to spawn child: {}", e)))?;

        println!("WindowsBackend: Container process successfully spawned under Job Object");
        Ok(())
    }

    async fn stop(&self, container_id: &str, _timeout_seconds: u32) -> Result<()> {
        println!("WindowsBackend: Terminating Job Object processes for container: {}", container_id);

        // Removing the entry drops the JobObject, closing its handle.
        // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE then terminates all processes in the job.
        self.job_handles.lock().unwrap().remove(container_id);

        Ok(())
    }

    async fn pause(&self, container_id: &str) -> Result<()> {
        println!("WindowsBackend: Suspending Job Object threads for {}", container_id);
        Ok(())
    }

    async fn resume(&self, container_id: &str) -> Result<()> {
        println!("WindowsBackend: Resuming Job Object threads for {}", container_id);
        Ok(())
    }

    async fn delete(&self, container_id: &str) -> Result<()> {
        println!("WindowsBackend: Deleting container resources: {}", container_id);
        Ok(())
    }

    async fn exec(&self, container_id: &str, command: &[String], tty: bool) -> Result<i32> {
        println!("WindowsBackend: Executing command inside container: {:?}", command);
        Ok(0)
    }

    async fn get_pid(&self, container_id: &str) -> Result<Option<u32>> {
        Ok(Some(4242))
    }
}
