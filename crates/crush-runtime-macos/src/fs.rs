use std::path::PathBuf;
use crush_types::{Result, CrushError};

pub struct VirtioFsConfig {
    tag: String,
    host_path: PathBuf,
    _guest_path: PathBuf,
}

impl VirtioFsConfig {
    pub fn new(host_path: &PathBuf, guest_path: &PathBuf, vm_id: &str) -> Result<Self> {
        if !host_path.exists() {
            return Err(CrushError::StorageError(format!(
                "Host path does not exist for VirtioFS share: {:?}",
                host_path
            )));
        }

        let tag = format!("crush_{}_{}", vm_id, guest_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(16)
            .collect::<String>()
        );

        Ok(Self {
            tag,
            host_path: host_path.clone(),
            _guest_path: guest_path.clone(),
        })
    }

    #[cfg(target_os = "macos")]
    pub fn create_device(&self) -> Result<objc2::rc::Retained<objc2_foundation::NSObject>> {
        use objc2::rc::Retained;
        use objc2::msg_send_id;
        use objc2_foundation::{NSObject, NSString};
        use crate::bindings::*;

        let device: Retained<NSObject> = unsafe {
            let fs_dev = VZVirtioFileSystemDeviceConfiguration::initWithTag(
                &NSString::from_str(&self.tag),
            );

            let url = ns_url_from_path(&self.host_path);
            let shared_dir = VZSharedDirectory::initWithURL(&url, false);
            let directory_share = VZSingleDirectoryShare::initWithDirectory(&shared_dir);

            fs_dev.setDirectoryShare(&directory_share);
            fs_dev.upcast()
        };

        Ok(device)
    }
}
