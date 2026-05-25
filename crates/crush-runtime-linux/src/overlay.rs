use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError};

#[cfg(target_os = "linux")]
use nix::mount::{mount, MntFlags, MsFlags};
#[cfg(target_os = "linux")]
use nix::unistd::chdir;

pub struct OverlayManager {
    lower_layers: Vec<PathBuf>,
    upper_dir: PathBuf,
    work_dir: PathBuf,
    merged_dir: PathBuf,
}

impl OverlayManager {
    pub fn new(
        lower_layers: Vec<PathBuf>,
        upper_dir: PathBuf,
        work_dir: PathBuf,
        merged_dir: PathBuf,
    ) -> Self {
        Self {
            lower_layers,
            upper_dir,
            work_dir,
            merged_dir,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn mount_overlay(&self) -> Result<()> {
        let lower_paths: Vec<String> = self.lower_layers.iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
            
        let options = format!(
            "lowerdir={},upperdir={},workdir={}",
            lower_paths.join(":"),
            self.upper_dir.to_string_lossy(),
            self.work_dir.to_string_lossy()
        );

        mount(
            Some("overlay"),
            &self.merged_dir,
            Some("overlay"),
            MsFlags::MS_NODEV | MsFlags::MS_NOSUID,
            Some(options.as_str()),
        ).map_err(|e| CrushError::StorageError(format!("Failed to mount OverlayFS: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn mount_overlay(&self) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn execute_pivot_root(&self, old_root_dir: &Path) -> Result<()> {
        // pivot_root requires that the new root is a mount point, so we bind mount it onto itself
        mount(
            Some(&self.merged_dir),
            &self.merged_dir,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        ).map_err(|e| CrushError::StorageError(format!("Failed bind mount for pivot_root: {}", e)))?;

        // Perform the pivot_root swap
        nix::unistd::pivot_root(&self.merged_dir, old_root_dir)
            .map_err(|e| CrushError::StorageError(format!("pivot_root system call failed: {}", e)))?;

        // Switch execution directory to new root
        chdir("/")
            .map_err(|e| CrushError::StorageError(format!("Failed to chdir to new root /: {}", e)))?;

        // Detach and unmount the old root filesystem
        nix::mount::umount2(old_root_dir, MntFlags::MNT_DETACH)
            .map_err(|e| CrushError::StorageError(format!("Failed to unmount old root directory: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn execute_pivot_root(&self, _old_root_dir: &Path) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn mount_filtered_proc(&self) -> Result<()> {
        let proc_path = Path::new("/proc");
        
        // Mount procfs securely with hidepid=2 to mask host process IDs
        mount(
            Some("proc"),
            proc_path,
            Some("proc"),
            MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            Some("hidepid=2"),
        ).map_err(|e| CrushError::StorageError(format!("Failed to mount secure procfs: {}", e)))?;

        // Restrict sysctl configurations inside container
        mount(
            Some("/proc/sys"),
            Path::new("/proc/sys"),
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_REMOUNT,
            None::<&str>,
        ).map_err(|e| CrushError::StorageError(format!("Failed to lock /proc/sys read-only: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn mount_filtered_proc(&self) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }
}
