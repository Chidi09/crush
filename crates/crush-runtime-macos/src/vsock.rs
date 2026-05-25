use std::path::PathBuf;
use crush_types::{Result, CrushError};

pub struct VsockConfig {
    vm_id: String,
    port: u32,
    socket_path: PathBuf,
}

impl VsockConfig {
    pub fn new(vm_id: &str) -> Result<Self> {
        // ⚠ FIX: Use SHA256 hash of vm_id for port to avoid collisions
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(vm_id.as_bytes());
        let hash = hasher.finalize();
        let port = (u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) % 50000) + 1024;
        let socket_dir = std::env::temp_dir().join("crush_vsock");
        std::fs::create_dir_all(&socket_dir).ok();

        Ok(Self {
            vm_id: vm_id.to_string(),
            port: port.max(1024),
            socket_path: socket_dir.join(format!("{}.sock", vm_id)),
        })
    }

    #[cfg(target_os = "macos")]
    pub fn create_device(&self) -> Result<objc2::rc::Retained<objc2_foundation::NSObject>> {
        use objc2::rc::Retained;
        use objc2_foundation::NSObject;
        use crate::bindings::*;

        let device: Retained<NSObject> = unsafe {
            VZVirtioSocketDeviceConfiguration::init().upcast()
        };

        Ok(device)
    }

    pub fn port(&self) -> u32 {
        self.port
    }

    #[allow(dead_code)]
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }
}
