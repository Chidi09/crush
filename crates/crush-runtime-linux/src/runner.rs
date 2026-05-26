use std::path::Path;
use crush_types::{Result, CrushError};

/// Fork a child into a new mount+UTS namespace, chroot into `rootfs`, then exec `command`.
/// Mounts /proc and bind-mounts /dev inside the rootfs before exec.
/// Blocks until the container process exits, returning its exit code.
pub fn run_container(rootfs: &Path, command: &[String], env_vars: &[String]) -> Result<i32> {
    if command.is_empty() {
        return Err(CrushError::NamespaceError("No entrypoint or cmd defined for image".to_string()));
    }

    // Clean any old debug log
    let _ = std::fs::remove_file("/tmp/crush_pre_exec_debug.log");

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
            let log = |msg: &str| {
                let path = b"/tmp/crush_pre_exec_debug.log\0";
                let fd = libc::open(path.as_ptr() as *const libc::c_char, libc::O_CREAT | libc::O_WRONLY | libc::O_APPEND, 0o644);
                if fd >= 0 {
                    let _ = libc::write(fd, msg.as_ptr() as *const libc::c_void, msg.len());
                    let _ = libc::write(fd, b"\n".as_ptr() as *const libc::c_void, 1);
                    let _ = libc::close(fd);
                }
            };

            let log_err = |msg: &str, err: i32| {
                let path = b"/tmp/crush_pre_exec_debug.log\0";
                let fd = libc::open(path.as_ptr() as *const libc::c_char, libc::O_CREAT | libc::O_WRONLY | libc::O_APPEND, 0o644);
                if fd >= 0 {
                    let _ = libc::write(fd, msg.as_ptr() as *const libc::c_void, msg.len());
                    let err_str = err.to_string();
                    let _ = libc::write(fd, b" error code: ".as_ptr() as *const libc::c_void, 13);
                    let _ = libc::write(fd, err_str.as_ptr() as *const libc::c_void, err_str.len());
                    let _ = libc::write(fd, b"\n".as_ptr() as *const libc::c_void, 1);
                    let _ = libc::close(fd);
                }
            };

            log("1. Starting pre_exec");

            // New mount + UTS + IPC + CGROUP + TIME namespaces
            let base_flags = nix::sched::CloneFlags::CLONE_NEWNS 
                | nix::sched::CloneFlags::CLONE_NEWUTS
                | nix::sched::CloneFlags::CLONE_NEWIPC
                | nix::sched::CloneFlags::CLONE_NEWCGROUP;
            
            // Try to unshare with TIME namespace (0x00000080)
            let all_flags = base_flags.bits() | 0x00000080;
            log("2. Attempting to unshare all namespaces (including TIME)");
            if libc::unshare(all_flags) != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("3. Unshare with TIME namespace failed", err_val);
                log("4. Attempting fallback unshare (base_flags)");
                if let Err(e) = nix::sched::unshare(base_flags) {
                    let fb_err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                    log_err("5. Fallback unshare failed", fb_err);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to unshare namespaces (base_flags={:?}): {}", base_flags, e)
                    ));
                }
            }
            log("6. Namespaces unshared successfully");

            // Make all mounts private so pivot_root doesn't fail with EINVAL due to shared mounts
            log("6.5 Making root mount private");
            let root_path = b"/\0";
            if libc::mount(
                std::ptr::null(),
                root_path.as_ptr() as *const libc::c_char,
                std::ptr::null(),
                libc::MS_PRIVATE | libc::MS_REC,
                std::ptr::null(),
            ) != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("6.6 Making root private failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to make root mount private: {}", std::io::Error::last_os_error())
                ));
            }

            // Mount procfs inside the container rootfs
            log("7. Mounting /proc");
            let proc_dir = rootfs_path.join("proc");
            let _ = std::fs::create_dir_all(&proc_dir);
            let src_proc = std::ffi::CString::new("proc").unwrap();
            let tgt_proc = std::ffi::CString::new(proc_dir.to_string_lossy().as_bytes()).unwrap();
            let mount_proc_ret = libc::mount(
                src_proc.as_ptr(),
                tgt_proc.as_ptr(),
                src_proc.as_ptr(),
                libc::MS_NOSUID | libc::MS_NOEXEC | libc::MS_NODEV,
                std::ptr::null(),
            );
            if mount_proc_ret != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("8. Mounting /proc failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to mount /proc inside rootfs: {}", std::io::Error::last_os_error())
                ));
            }

            // Bind-mount /dev into the container rootfs
            log("9. Bind-mounting /dev");
            let dev_dir = rootfs_path.join("dev");
            let _ = std::fs::create_dir_all(&dev_dir);
            let src_dev = std::ffi::CString::new("/dev").unwrap();
            let tgt_dev = std::ffi::CString::new(dev_dir.to_string_lossy().as_bytes()).unwrap();
            let mount_dev_ret = libc::mount(
                src_dev.as_ptr(),
                tgt_dev.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND | libc::MS_REC,
                std::ptr::null(),
            );
            if mount_dev_ret != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("10. Bind-mounting /dev failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to bind-mount /dev inside rootfs: {}", std::io::Error::last_os_error())
                ));
            }

            // pivot_root requires that the new root is a mount point, so we bind mount it onto itself
            log("11. Bind-mounting rootfs onto itself");
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
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("12. Bind-mounting rootfs onto itself failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to bind-mount rootfs onto itself ({}): {}", rootfs_str, std::io::Error::last_os_error())
                ));
            }

            // Create temporary dir for the old root
            log("13. Creating old_root dir");
            let old_root_dir = rootfs_path.join(".old_root");
            let _ = std::fs::create_dir_all(&old_root_dir);

            let old_root_str = old_root_dir.to_string_lossy().into_owned();
            let old_root_c = std::ffi::CString::new(old_root_str.as_bytes()).unwrap();

            // Perform pivot_root
            log("14. Performing pivot_root");
            let ret = libc::syscall(libc::SYS_pivot_root, tgt_root_c.as_ptr(), old_root_c.as_ptr());
            if ret != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("15. pivot_root failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed pivot_root: {}", std::io::Error::last_os_error())
                ));
            }

            // Switch execution directory to new root
            log("16. Performing chdir to new root");
            if libc::chdir(std::ffi::CString::new("/").unwrap().as_ptr()) != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("17. chdir to new root failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed chdir to new root: {}", std::io::Error::last_os_error())
                ));
            }

            // Detach and unmount the old root filesystem
            log("18. Umounting old_root");
            let old_root_relative = std::ffi::CString::new("/.old_root").unwrap();
            if libc::umount2(old_root_relative.as_ptr(), libc::MNT_DETACH) != 0 {
                let err_val = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                log_err("19. Umounting old_root failed", err_val);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to umount old_root: {}", std::io::Error::last_os_error())
                ));
            }

            // Remove /.old_root directory
            log("20. Removing /.old_root dir");
            let _ = std::fs::remove_dir("/.old_root");

            log("21. Completed pre_exec successfully!");
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
