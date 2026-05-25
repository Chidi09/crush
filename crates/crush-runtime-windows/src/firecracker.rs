use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::net::TcpListener;
use crush_types::{Result, CrushError};

pub struct FirecrackerMicroVM {
    vm_id: String,
    socket_path: PathBuf,
    process: Option<std::process::Child>,
}

impl FirecrackerMicroVM {
    pub fn new(vm_id: String, socket_path: PathBuf) -> Self {
        Self {
            vm_id,
            socket_path,
            process: None,
        }
    }

    pub fn start_hypervisor_process(&mut self, binary_path: &PathBuf) -> Result<()> {
        println!("Firecracker: Spawning hypervisor process for microVM ID: {}", self.vm_id);
        
        let child = Command::new(binary_path)
            .arg("--api-sock")
            .arg(&self.socket_path)
            .spawn()
            .map_err(|e| CrushError::WasmError(format!("Failed to spawn firecracker binary: {}", e)))?;

        self.process = Some(child);
        Ok(())
    }

    pub fn configure_vm_resources(&self, kernel_path: &PathBuf, rootfs_path: &PathBuf, memory_mb: u64, vcpus: u32) -> Result<()> {
        let boot_source = serde_json::json!({
            "kernel_image_path": kernel_path.to_string_lossy(),
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off nomodules root=/dev/vda ro"
        });

        let drives = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": rootfs_path.to_string_lossy(),
            "is_root_device": true,
            "is_read_only": true
        });

        let machine_config = serde_json::json!({
            "vcpu_count": vcpus,
            "mem_size_mib": memory_mb,
            "track_dirty_pages": false
        });

        let balloon = serde_json::json!({
            "amount_mib": 0,
            "deflate_on_oom": true,
            "stats_polling_interval_s": 1
        });

        let config_file = self.socket_path.with_extension("json");
        let full_config = serde_json::json!({
            "boot-source": boot_source,
            "drives": [drives],
            "machine-config": machine_config,
            "balloon": balloon
        });

        fs::write(&config_file, full_config.to_string())
            .map_err(|e| CrushError::WasmError(format!("Failed to write VM config JSON file: {}", e)))?;

        println!("Firecracker: Resource parameters successfully generated at {:?}", config_file);
        Ok(())
    }

    pub fn setup_vsock_comm(&self, host_port: u32, _guest_port: u32) -> Result<()> {
        println!("Firecracker: Initializing virtio-vsock listener socket on host port: {}", host_port);
        
        // Establishes a Winsock TCP listener to act as the vsock loopback proxy for communication
        let listener = TcpListener::bind(format!("127.0.0.1:{}", host_port))
            .map_err(|e| CrushError::NetworkError(format!("Failed to bind vsock host socket: {}", e)))?;
        
        // Disable blocking to integrate into tokio event loops
        listener.set_nonblocking(true)
            .map_err(|e| CrushError::NetworkError(format!("Failed to set vsock non-blocking: {}", e)))?;

        println!("Firecracker: Vsock control socket listener active on {:?}", listener.local_addr());
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        println!("Firecracker: Halting hypervisor and killing child process (ID: {})", self.vm_id);
        
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }
        
        Ok(())
    }
}
