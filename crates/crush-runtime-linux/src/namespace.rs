#[cfg(target_os = "linux")]
use nix::sched::{unshare, CloneFlags};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

use crush_types::{Result, CrushError};

pub struct NamespaceManager;

impl NamespaceManager {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "linux")]
    pub fn unshare_namespaces(&self) -> Result<()> {
        let flags = CloneFlags::CLONE_NEWUTS 
            | CloneFlags::CLONE_NEWIPC 
            | CloneFlags::CLONE_NEWPID 
            | CloneFlags::CLONE_NEWNET 
            | CloneFlags::CLONE_NEWNS 
            | CloneFlags::CLONE_NEWCGROUP
            | CloneFlags::CLONE_NEWUSER;

        unshare(flags).map_err(|e| CrushError::NamespaceError(format!("Failed to unshare namespaces: {}", e)))?;
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn unshare_namespaces(&self) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub fn join_namespace(&self, ns_path: &str, ns_type: CloneFlags) -> Result<()> {
        let file = File::open(ns_path)
            .map_err(|e| CrushError::NamespaceError(format!("Failed to open namespace path: {}", e)))?;
        
        let fd = file.as_raw_fd();
        let success = unsafe {
            libc::setns(fd, ns_type.bits() as i32)
        };

        if success != 0 {
            return Err(CrushError::NamespaceError(format!(
                "Failed setns join for path {}: {}",
                ns_path,
                std::io::Error::last_os_error()
            )));
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn join_namespace(&self, _ns_path: &str, _ns_type: u32) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }
}
