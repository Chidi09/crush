use std::path::PathBuf;
use crush_types::{Result, CrushError};

pub struct ConsoleConfig {
    vm_id: String,
    #[allow(dead_code)]
    pty_path: PathBuf,
}

impl ConsoleConfig {
    pub fn new(vm_id: &str) -> Self {
        let pty_dir = std::env::temp_dir().join("crush_console");
        std::fs::create_dir_all(&pty_dir).ok();

        Self {
            vm_id: vm_id.to_string(),
            pty_path: pty_dir.join(format!("{}.pty", vm_id)),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn create_device(&self) -> Result<objc2::rc::Retained<objc2_foundation::NSObject>> {
        use objc2::rc::Retained;
        use objc2_foundation::NSObject;
        use crate::bindings::*;

        let device: Retained<NSObject> = unsafe {
            VZVirtioConsoleDeviceConfiguration::init().upcast()
        };

        Ok(device)
    }
}
