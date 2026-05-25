use std::path::PathBuf;
use crush_types::{Result, CrushError};
use crate::bindings;

pub struct LinuxBootConfig {
    kernel_path: PathBuf,
    initrd_path: Option<PathBuf>,
    cmdline: String,
}

impl LinuxBootConfig {
    pub fn new(kernel_path: &PathBuf, initrd_path: &PathBuf) -> Self {
        Self {
            kernel_path: kernel_path.clone(),
            initrd_path: Some(initrd_path.clone()),
            cmdline: "console=hvc0 root=/dev/vda rw panic=1".to_string(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if !self.kernel_path.exists() {
            return Err(CrushError::StorageError(format!(
                "Kernel not found at {:?}",
                self.kernel_path
            )));
        }
        if let Some(ref initrd) = self.initrd_path {
            if !initrd.exists() {
                return Err(CrushError::StorageError(format!(
                    "Initrd not found at {:?}",
                    initrd
                )));
            }
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub fn create_boot_loader(&self) -> Result<objc2::rc::Retained<objc2_foundation::NSObject>> {
        use objc2::rc::Retained;
        use objc2::{msg_send_id, sel};
        use objc2_foundation::{NSObject, NSString};
        use crate::bindings::*;

        let kernel_url = bindings::ns_url_from_path(&self.kernel_path);

        let loader: Retained<NSObject> = unsafe {
            let vz_loader = msg_send_id![VZLinuxBootLoader::class(), alloc];
            let loader: Retained<VZLinuxBootLoader> = vz_loader.initWithKernelURL(&kernel_url);

            if let Some(ref initrd) = self.initrd_path {
                let initrd_url = bindings::ns_url_from_path(initrd);
                loader.setInitialRamdiskURL(Some(&initrd_url));
            }

            loader.setCommandLine(&NSString::from_str(&self.cmdline));
            loader.upcast()
        };

        Ok(loader)
    }
}
