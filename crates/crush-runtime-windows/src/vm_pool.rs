use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{anyhow, Result};
use crate::firecracker::FirecrackerRunner;
use crate::snapshot::SnapshotStore;
use crate::ext4_cache::Ext4Cache;

const POOL_SIZE: usize = 2;

pub struct PooledVm {
    pub runner: FirecrackerRunner,
    pub vm_id: String,
}

pub struct VmPool {
    idle: Arc<Mutex<VecDeque<PooledVm>>>,
    data_dir: PathBuf,
    fc_binary: PathBuf,
    kernel_path: PathBuf,
    snapshots: SnapshotStore,
    ext4: Ext4Cache,
}

impl VmPool {
    pub fn new(data_dir: &Path, fc_binary: PathBuf, kernel_path: PathBuf) -> Self {
        Self {
            idle: Arc::new(Mutex::new(VecDeque::new())),
            data_dir: data_dir.to_path_buf(),
            fc_binary,
            kernel_path,
            snapshots: SnapshotStore::new(data_dir),
            ext4: Ext4Cache::new(data_dir),
        }
    }

    /// Pre-warm the pool with `POOL_SIZE` idle VMs.
    /// Call once at `crush daemon` startup or on first `crush run`.
    /// Each VM is booted from the base snapshot, paused, and held ready.
    pub async fn warm(&self, base_image_digest: &str) -> Result<()> {
        let mut idle = self.idle.lock().await;
        while idle.len() < POOL_SIZE {
            let vm_id = format!("pool_{}", uuid::Uuid::new_v4().simple());
            let pipe_path = PathBuf::from(format!(r"\\.\pipe\fc_{}", &vm_id[..12]));

            // Use a base rootfs — the real drive is hot-swapped when claimed
            let base_drive = self.ext4.drive_path(base_image_digest);
            if !base_drive.exists() {
                // Cannot warm pool without a base drive — skip silently
                break;
            }

            let mut runner = FirecrackerRunner::new(
                vm_id.clone(),
                pipe_path,
                self.fc_binary.clone(),
                self.kernel_path.clone(),
                base_drive,
            );

            // Boot from snapshot if available, else cold boot and snapshot
            let mem_path = self.snapshots.mem_path(base_image_digest);
            let state_path = self.snapshots.state_path(base_image_digest);

            if self.snapshots.exists(base_image_digest) {
                runner.boot_or_restore(256, 2, &[], &[], &[], Some((&mem_path, &state_path))).await?;
            } else {
                runner.boot_or_restore(256, 2, &[], &[], &[], None).await?;
                // Wait for init to be ready (~400ms), then snapshot
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                runner.snapshot_create(&mem_path, &state_path).await?;
            }

            idle.push_back(PooledVm { runner, vm_id });
            println!("[VmPool] Warmed idle VM (pool size: {})", idle.len());
        }
        Ok(())
    }

    /// Claim a VM from the pool for a container run.
    /// Hot-swaps the drive to the container's ext4 image, updates kernel boot args,
    /// and resumes execution. Returns the vm_id so the caller can attach stdio.
    ///
    /// If the pool is empty, falls back to a cold-boot `FirecrackerRunner` directly.
    pub async fn claim(
        &self,
        image_digest: &str,
        cmd: &[String],
        env: &[String],
        memory_mib: u64,
        ports: &[crush_types::PortMapping],
    ) -> Result<String> {
        let drive_path = self.ext4.drive_path(image_digest);
        if !drive_path.exists() {
            return Err(anyhow!(
                "No ext4 drive cached for image {}. Run `crush pull` first.",
                image_digest
            ));
        }

        let mut idle = self.idle.lock().await;

        if let Some(mut vm) = idle.pop_front() {
            // Hot-swap: update the drive to this container's rootfs
            vm.runner.hot_swap_drive(&drive_path).await?;

            // Send the command to the guest via vsock control channel
            vm.runner.send_exec_config(cmd, env).await?;

            // Resume the paused VM — it starts executing immediately
            vm.runner.api_put("vm", &serde_json::json!({ "state": "Resumed" })).await?;

            let vm_id = vm.vm_id.clone();
            println!("[VmPool] Claimed VM {} from pool (~50ms path)", vm_id);

            // Spawn a background task to refill the pool slot
            let pool_idle = self.idle.clone();
            let data_dir = self.data_dir.clone();
            let fc_bin = self.fc_binary.clone();
            let kernel = self.kernel_path.clone();
            let digest = image_digest.to_string();
            tokio::spawn(async move {
                // Brief delay so the claimed VM is fully running before we spawn a replacement
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                let pool = VmPool::new(&data_dir, fc_bin, kernel);
                let _ = pool.warm(&digest).await;
                // Move the newly warmed VM into the idle queue
                let mut new_idle = pool.idle.lock().await;
                let mut target = pool_idle.lock().await;
                while let Some(warmed_vm) = new_idle.pop_front() {
                    target.push_back(warmed_vm);
                }
            });

            Ok(vm_id)
        } else {
            // Pool empty — cold boot fallback
            println!("[VmPool] Pool empty, cold-booting...");
            let vm_id = format!("fc_{}", uuid::Uuid::new_v4().simple());
            let pipe_path = PathBuf::from(format!(r"\\.\pipe\fc_{}", &vm_id[..12]));
            let mem_path = self.snapshots.mem_path(image_digest);
            let state_path = self.snapshots.state_path(image_digest);

            let mut runner = FirecrackerRunner::new(
                vm_id.clone(),
                pipe_path,
                self.fc_binary.clone(),
                self.kernel_path.clone(),
                drive_path,
            );

            let snapshot = if self.snapshots.exists(image_digest) {
                Some((mem_path.as_path(), state_path.as_path()))
            } else {
                None
            };

            runner.boot_or_restore(memory_mib, 2, cmd, env, ports, snapshot).await?;
            Ok(vm_id)
        }
    }
}
