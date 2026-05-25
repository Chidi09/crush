use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError};

#[cfg(target_os = "linux")]
use nix::sys::stat::{mknod, Mode, SFlag};
#[cfg(target_os = "linux")]
use nix::mount::{mount, MsFlags};

pub struct DeviceNodeManager {
    dev_path: PathBuf,
}

impl DeviceNodeManager {
    pub fn new(root_dir: &Path) -> Self {
        Self {
            dev_path: root_dir.join("dev"),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn populate_minimal_dev(&self) -> Result<()> {
        let null_node = self.dev_path.join("null");
        let zero_node = self.dev_path.join("zero");
        let random_node = self.dev_path.join("random");
        let urandom_node = self.dev_path.join("urandom");

        // Major/Minor device numbers mapping:
        // null character: major 1, minor 3
        // zero character: major 1, minor 5
        // random character: major 1, minor 8
        // urandom character: major 1, minor 9
        
        let char_mode = Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP | Mode::S_IROTH | Mode::S_IWOTH;
        
        mknod(&null_node, SFlag::S_IFCHR, char_mode, libc::makedev(1, 3))
            .map_err(|e| CrushError::StorageError(format!("Failed to mknod /dev/null: {}", e)))?;

        mknod(&zero_node, SFlag::S_IFCHR, char_mode, libc::makedev(1, 5))
            .map_err(|e| CrushError::StorageError(format!("Failed to mknod /dev/zero: {}", e)))?;

        mknod(&random_node, SFlag::S_IFCHR, char_mode, libc::makedev(1, 8))
            .map_err(|e| CrushError::StorageError(format!("Failed to mknod /dev/random: {}", e)))?;

        mknod(&urandom_node, SFlag::S_IFCHR, char_mode, libc::makedev(1, 9))
            .map_err(|e| CrushError::StorageError(format!("Failed to mknod /dev/urandom: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn populate_minimal_dev(&self) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn mount_devpts(&self) -> Result<()> {
        let pts_path = self.dev_path.join("pts");
        
        mount(
            Some("devpts"),
            &pts_path,
            Some("devpts"),
            MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
            Some("newinstance,ptmxmode=0666,mode=0620"),
        ).map_err(|e| CrushError::StorageError(format!("Failed to mount devpts filesystem: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn mount_devpts(&self) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }
}
