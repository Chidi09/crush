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

    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "linux")]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(move || {
            // New mount + UTS + IPC + CGROUP + TIME namespaces
            let base_flags = nix::sched::CloneFlags::CLONE_NEWNS 
                | nix::sched::CloneFlags::CLONE_NEWUTS
                | nix::sched::CloneFlags::CLONE_NEWIPC
                | nix::sched::CloneFlags::CLONE_NEWCGROUP;
            
            let all_flags = base_flags | nix::sched::CloneFlags::CLONE_NEWTIME;
            if nix::sched::unshare(all_flags).is_err() {
                nix::sched::unshare(base_flags)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            }

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

            // pivot_root requires that the new root is a mount point, so we bind mount it onto itself
            let rootfs_str = rootfs_path.to_string_lossy().into_owned();
            let tgt_root_c = std::ffi::CString::new(rootfs_str.as_bytes()).unwrap();
            let mount_ret = libc::mount(
                tgt_root_c.as_ptr(),
                tgt_root_c.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND | libc::MS_REC,
                std::ptr::null(),
            );
            if mount_ret != 0 {
                return Err(std::io::Error::last_os_error());
            }

            // Create temporary dir for the old root
            let old_root_dir = rootfs_path.join(".old_root");
            let _ = std::fs::create_dir_all(&old_root_dir);

            let old_root_str = old_root_dir.to_string_lossy().into_owned();
            let old_root_c = std::ffi::CString::new(old_root_str.as_bytes()).unwrap();

            // Perform pivot_root
            let ret = libc::syscall(libc::SYS_pivot_root, tgt_root_c.as_ptr(), old_root_c.as_ptr());
            if ret != 0 {
                return Err(std::io::Error::last_os_error());
            }

            // Switch execution directory to new root
            if libc::chdir(std::ffi::CString::new("/").unwrap().as_ptr()) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            // Detach and unmount the old root filesystem
            let old_root_relative = std::ffi::CString::new("/.old_root").unwrap();
            if libc::umount2(old_root_relative.as_ptr(), libc::MNT_DETACH) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            // Remove /.old_root directory
            let _ = std::fs::remove_dir("/.old_root");

            Ok(())
        });
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| CrushError::NamespaceError(format!("Failed to spawn container process: {}", e)))?;

    // Tee the output streams to permanent log files on disk and live stdout/stderr
    let mut stdout = child.stdout.take().ok_or_else(|| CrushError::NamespaceError("Failed to capture stdout".to_string()))?;
    let mut stderr = child.stderr.take().ok_or_else(|| CrushError::NamespaceError("Failed to capture stderr".to_string()))?;

    let container_dir = rootfs.parent().unwrap_or(rootfs).to_path_buf();
    let _ = std::fs::create_dir_all(&container_dir);
    let stdout_path = container_dir.join("stdout.log");
    let stderr_path = container_dir.join("stderr.log");

    let stdout_thread = std::thread::spawn(move || {
        let mut file = std::fs::File::create(&stdout_path).ok();
        let mut buffer = [0u8; 4096];
        use std::io::{Read, Write};
        while let Ok(n) = stdout.read(&mut buffer) {
            if n == 0 { break; }
            if let Some(ref mut f) = file {
                let _ = f.write_all(&buffer[..n]);
            }
            let _ = std::io::stdout().write_all(&buffer[..n]);
        }
    });

    let stderr_thread = std::thread::spawn(move || {
        let mut file = std::fs::File::create(&stderr_path).ok();
        let mut buffer = [0u8; 4096];
        use std::io::{Read, Write};
        while let Ok(n) = stderr.read(&mut buffer) {
            if n == 0 { break; }
            if let Some(ref mut f) = file {
                let _ = f.write_all(&buffer[..n]);
            }
            let _ = std::io::stderr().write_all(&buffer[..n]);
        }
    });

    let status = child
        .wait()
        .map_err(|e| CrushError::NamespaceError(format!("Failed to wait for container: {}", e)))?;

    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    Ok(status.code().unwrap_or(-1))
}
