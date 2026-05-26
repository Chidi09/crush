use std::path::Path;
use crush_types::{Result, CrushError};

/// Fork a child into a new mount+UTS namespace, chroot into `rootfs`, then exec `command`.
/// Mounts /proc and bind-mounts /dev inside the rootfs before exec.
/// Blocks until the container process exits, returning its exit code.
pub fn run_container(rootfs: &Path, command: &[String], env_vars: &[String]) -> Result<i32> {
    if command.is_empty() {
        return Err(CrushError::NamespaceError("No entrypoint or cmd defined for image".to_string()));
    }

    // Ensure standard dirs exist in the rootfs before forking
    for dir in &["proc", "dev", "sys", "tmp"] {
        std::fs::create_dir_all(rootfs.join(dir)).ok();
    }

    let rootfs_path = rootfs.to_path_buf();

    let mut cmd = std::process::Command::new(&command[0]);
    if command.len() > 1 {
        cmd.args(&command[1..]);
    }

    cmd.env_clear();
    for var in env_vars {
        if let Some(pos) = var.find('=') {
            cmd.env(&var[..pos], &var[pos + 1..]);
        }
    }

    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    #[cfg(target_os = "linux")]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(move || {
            // New mount + UTS namespace so mounts and hostname don't affect the host
            nix::sched::unshare(
                nix::sched::CloneFlags::CLONE_NEWNS | nix::sched::CloneFlags::CLONE_NEWUTS,
            )
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

            // Mount procfs inside the container rootfs
            let proc_dir = rootfs_path.join("proc");
            let _ = std::fs::create_dir_all(&proc_dir);
            let src_proc = std::ffi::CString::new("proc").unwrap();
            let tgt_proc = std::ffi::CString::new(proc_dir.to_string_lossy().as_bytes()).unwrap();
            libc::mount(
                src_proc.as_ptr(),
                tgt_proc.as_ptr(),
                src_proc.as_ptr(),
                libc::MS_NOSUID | libc::MS_NOEXEC | libc::MS_NODEV,
                std::ptr::null(),
            );

            // Bind-mount /dev into the container rootfs
            let dev_dir = rootfs_path.join("dev");
            let _ = std::fs::create_dir_all(&dev_dir);
            let src_dev = std::ffi::CString::new("/dev").unwrap();
            let tgt_dev = std::ffi::CString::new(dev_dir.to_string_lossy().as_bytes()).unwrap();
            libc::mount(
                src_dev.as_ptr(),
                tgt_dev.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND | libc::MS_REC,
                std::ptr::null(),
            );

            // chroot + move to new root
            nix::unistd::chroot(&rootfs_path)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("chroot: {}", e)))?;
            std::env::set_current_dir("/")?;

            Ok(())
        });
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| CrushError::NamespaceError(format!("Failed to spawn container process: {}", e)))?;

    let status = child
        .wait()
        .map_err(|e| CrushError::NamespaceError(format!("Failed to wait for container: {}", e)))?;

    Ok(status.code().unwrap_or(-1))
}
