#[cfg(target_os = "macos")]
pub mod bindings;
#[cfg(target_os = "macos")]
pub mod vm;
#[cfg(target_os = "macos")]
pub mod boot;
#[cfg(target_os = "macos")]
pub mod network;
#[cfg(target_os = "macos")]
pub mod storage;
#[cfg(target_os = "macos")]
pub mod fs;
#[cfg(target_os = "macos")]
pub mod vsock;
#[cfg(target_os = "macos")]
pub mod console;
#[cfg(target_os = "macos")]
pub mod rosetta;
#[cfg(target_os = "macos")]
pub mod sandbox;
#[cfg(target_os = "macos")]
pub mod image;
#[cfg(target_os = "macos")]
pub mod port_forward;

#[cfg(target_os = "macos")]
mod macos_impl {
    use std::path::PathBuf;
    use std::sync::Arc;
    use async_trait::async_trait;
    use tokio::sync::Mutex;
    use crush_types::{RuntimeBackend, Container, Result, CrushError};
    use super::vm::VirtualMachineManager;
    use super::storage::StorageManager;

    pub struct MacOsRuntime {
        vms: Arc<Mutex<std::collections::HashMap<String, VirtualMachineManager>>>,
        storage: StorageManager,
    }

    impl MacOsRuntime {
        pub fn new(data_dir: PathBuf) -> Self {
            let storage = StorageManager::new(data_dir.join("vms"));
            Self {
                vms: Arc::new(Mutex::new(std::collections::HashMap::new())),
                storage,
            }
        }
    }

    #[async_trait]
    impl RuntimeBackend for MacOsRuntime {
        async fn create(&self, container: &Container, spec_path: &PathBuf) -> Result<()> {
            let _vm_dir = self.storage.prepare_vm_directory(&container.id)?;
            let kernel_path = self.storage.ensure_kernel()?;
            let initrd_path = self.storage.ensure_initrd()?;
            let disk_path = self.storage.create_overlay_disk(&container.id)?;

            let memory_mb = container.memory_limit_bytes
                .map(|b| (b / 1024 / 1024) as u64)
                .unwrap_or(512);
            let cpu_count = container.cpu_shares
                .map(|s| ((s as f64 / 1024.0) * 4.0).ceil() as u64)
                .unwrap_or(2)
                .max(1)
                .min(8);

            let vm = VirtualMachineManager::new(
                &container.id,
                &kernel_path,
                &initrd_path,
                &disk_path,
                memory_mb,
                cpu_count,
            );

            vm.configure_boot()?;
            vm.configure_storage(&disk_path)?;

            if !container.ports.is_empty() {
                vm.configure_network()?;
            }

            for mount in &container.mounts {
                vm.configure_shared_directory(&mount.host_path, &mount.container_path)?;
            }

            vm.configure_vsock()?;
            vm.configure_console()?;

            if std::env::consts::ARCH == "aarch64" {
                vm.configure_rosetta_if_needed()?;
            }

            self.vms.lock().await.insert(container.id.clone(), vm);
            Ok(())
        }

        async fn start(&self, container_id: &str) -> Result<()> {
            let mut vms = self.vms.lock().await;
            let vm = vms.get_mut(container_id)
                .ok_or_else(|| CrushError::ContainerNotFound(container_id.to_string()))?;
            vm.start().await
        }

        async fn stop(&self, container_id: &str, timeout_seconds: u32) -> Result<()> {
            let mut vms = self.vms.lock().await;
            if let Some(vm) = vms.get_mut(container_id) {
                vm.stop(timeout_seconds).await?;
            }
            Ok(())
        }

        async fn pause(&self, container_id: &str) -> Result<()> {
            let mut vms = self.vms.lock().await;
            if let Some(vm) = vms.get_mut(container_id) {
                vm.pause()?;
            }
            Ok(())
        }

        async fn resume(&self, container_id: &str) -> Result<()> {
            let mut vms = self.vms.lock().await;
            if let Some(vm) = vms.get_mut(container_id) {
                vm.resume()?;
            }
            Ok(())
        }

        async fn delete(&self, container_id: &str) -> Result<()> {
            self.vms.lock().await.remove(container_id);
            self.storage.cleanup_vm(container_id)?;
            Ok(())
        }

        async fn exec(&self, container_id: &str, command: &[String], _tty: bool) -> Result<i32> {
            let vms = self.vms.lock().await;
            let vm = vms.get(container_id)
                .ok_or_else(|| CrushError::ContainerNotFound(container_id.to_string()))?;
            vm.send_command_over_vsock(command).await
        }

        async fn get_pid(&self, container_id: &str) -> Result<Option<u32>> {
            let vms = self.vms.lock().await;
            Ok(vms.get(container_id).map(|vm| vm.process_identifier()))
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos_impl::MacOsRuntime;
