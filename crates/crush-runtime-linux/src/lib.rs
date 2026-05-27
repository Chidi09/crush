pub mod namespace;
pub mod user_ns;
pub mod seccomp;
pub mod capabilities;
pub mod overlay;
pub mod devices;
pub mod cgroup;
pub mod signals;
pub mod lifecycle;
pub mod runner;

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use nix::sched::CloneFlags;
use tokio::sync::Mutex;
use crush_types::{RuntimeBackend, Container, Image, Result, CrushError, ContainerStatus};

use namespace::NamespaceManager;
use user_ns::UserNamespaceManager;
use seccomp::SeccompFilterCompiler;
use capabilities::CapabilitiesManager;
use overlay::OverlayManager;
use devices::DeviceNodeManager;
use cgroup::CgroupManager;
use signals::SignalHandler;
use lifecycle::ContainerLifecycleManager;

pub struct LinuxRuntime {
    ns: NamespaceManager,
    user_ns: UserNamespaceManager,
    seccomp: SeccompFilterCompiler,
    caps: CapabilitiesManager,
    signals: SignalHandler,
    lifecycle: ContainerLifecycleManager,
    child_pids: Arc<Mutex<std::collections::HashMap<String, u32>>>,
}

impl LinuxRuntime {
    pub fn new() -> Self {
        let rt = Self {
            ns: NamespaceManager::new(),
            user_ns: UserNamespaceManager::new(),
            seccomp: SeccompFilterCompiler::new(),
            caps: CapabilitiesManager::new(),
            signals: SignalHandler::new(),
            lifecycle: ContainerLifecycleManager::new(),
            child_pids: Arc::new(Mutex::new(std::collections::HashMap::new())),
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let child_pids = rt.child_pids.clone();
            handle.spawn(async move {
                let containers_dir = dirs_or_default().join("containers");
                if let Ok(entries) = std::fs::read_dir(&containers_dir) {
                    for entry in entries.flatten() {
                        let pid_file = entry.path().join("pid");
                        if let Ok(text) = std::fs::read_to_string(&pid_file) {
                            if let Ok(pid) = text.trim().parse::<u32>() {
                                #[cfg(unix)]
                                {
                                    if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
                                        let id = entry.file_name().to_string_lossy().to_string();
                                        child_pids.lock().await.insert(id, pid);
                                    } else {
                                        let _ = std::fs::remove_file(&pid_file);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        rt
    }
}

#[async_trait]
impl RuntimeBackend for LinuxRuntime {
    async fn create(&self, container: &Container, spec_path: &PathBuf) -> Result<()> {
        self.lifecycle.validate_state_transition(container.status, ContainerStatus::Created)?;

        self.ns.unshare_namespaces()?;

        // Trigger createRuntime hooks
        let create_runtime_hooks = load_oci_hooks(spec_path, "createRuntime");
        self.lifecycle.trigger_hooks_list("createRuntime", &create_runtime_hooks)?;

        let rootfs = spec_path.join("rootfs");
        let overlay = OverlayManager::new(
            container.mounts.iter().map(|m| m.host_path.clone()).collect(),
            spec_path.join("upper"),
            spec_path.join("work"),
            rootfs.clone()
        );
        overlay.mount_overlay()?;
        overlay.execute_pivot_root(&spec_path.join("old_root"))?;
        overlay.mount_filtered_proc()?;

        let dev_manager = DeviceNodeManager::new(&rootfs);
        dev_manager.populate_minimal_dev()?;
        dev_manager.mount_devpts()?;

        let cgroup = CgroupManager::new(&container.id);
        cgroup.initialize_cgroup()?;

        if let Some(mem_bytes) = container.memory_limit_bytes {
            cgroup.enforce_memory_limit(mem_bytes)?;
        }

        if let Some(cpu_shares) = container.cpu_shares {
            // Convert Docker cpu_shares (2-262144, default 1024) to cgroup v2
            // cpu.weight (1-10000, default 100). Linear scale: 1024 -> 100.
            let weight = ((cpu_shares as u64) * 100 / 1024).clamp(1, 10000);
            cgroup.enforce_cpu_limit(weight)?;
        }

        // Seccomp is NOT applied here — it must run in the container child process
        // after fork but before execve (via pre_exec in spawn_container_process).
        // Applying it here would restrict the daemon's own syscalls instead.

        self.caps.drop_unnecessary_capabilities()?;
        self.caps.apply_ambient_capabilities()?;

        // Trigger createContainer hooks
        let create_container_hooks = load_oci_hooks(spec_path, "createContainer");
        self.lifecycle.trigger_hooks_list("createContainer", &create_container_hooks)?;

        Ok(())
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        let spec_path = get_spec_path(container_id);

        // Trigger startContainer hooks
        let start_container_hooks = load_oci_hooks(&spec_path, "startContainer");
        self.lifecycle.trigger_hooks_list("startContainer", &start_container_hooks)?;

        let child_pid = self.spawn_container_process(container_id).await?;

        let cgroup = CgroupManager::new(container_id);
        cgroup.add_process_to_cgroup(child_pid)?;

        {
            let mut pids = self.child_pids.lock().await;
            pids.insert(container_id.to_string(), child_pid);
        }

        // Trigger poststart hooks
        let poststart_hooks = load_oci_hooks(&spec_path, "poststart");
        self.lifecycle.trigger_hooks_list("poststart", &poststart_hooks)?;

        Ok(())
    }

    async fn stop(&self, container_id: &str, timeout_seconds: u32) -> Result<()> {
        let pid = self.get_pid(container_id).await?.ok_or_else(|| {
            CrushError::ContainerNotFound(container_id.to_string())
        })?;

        self.signals.shutdown_container_gracefully(pid, timeout_seconds).await?;
        self.signals.reap_zombies();

        let cgroup = CgroupManager::new(container_id);
        let _ = cgroup.remove_cgroup();

        {
            let mut pids = self.child_pids.lock().await;
            pids.remove(container_id);
        }
        let _ = std::fs::remove_file(dirs_or_default().join("containers").join(container_id).join("pid"));

        let spec_path = get_spec_path(container_id);

        // Trigger poststop hooks
        let poststop_hooks = load_oci_hooks(&spec_path, "poststop");
        self.lifecycle.trigger_hooks_list("poststop", &poststop_hooks)?;

        Ok(())
    }

    async fn pause(&self, container_id: &str) -> Result<()> {
        let cgroup = CgroupManager::new(container_id);
        cgroup.set_freeze_state(true)
    }

    async fn resume(&self, container_id: &str) -> Result<()> {
        let cgroup = CgroupManager::new(container_id);
        cgroup.set_freeze_state(false)
    }

    async fn delete(&self, container_id: &str) -> Result<()> {
        let cgroup = CgroupManager::new(container_id);
        let _ = cgroup.remove_cgroup();

        {
            let mut pids = self.child_pids.lock().await;
            pids.remove(container_id);
        }

        Ok(())
    }

    async fn exec(&self, container_id: &str, command: &[String], tty: bool) -> Result<i32> {
        let pid = self.get_pid(container_id).await?.ok_or_else(|| {
            CrushError::ContainerNotFound(container_id.to_string())
        })?;

        let ns_path = format!("/proc/{}/ns", pid);

        let net_ns = format!("{}/net", ns_path);
        self.ns.join_namespace(&net_ns, CloneFlags::CLONE_NEWNET)?;

        let mnt_ns = format!("{}/mnt", ns_path);
        self.ns.join_namespace(&mnt_ns, CloneFlags::CLONE_NEWNS)?;

        let pid_ns = format!("{}/pid", ns_path);
        self.ns.join_namespace(&pid_ns, CloneFlags::CLONE_NEWPID)?;

        let mut child = tokio::process::Command::new(&command[0])
            .args(&command[1..])
            .stdin(if tty { std::process::Stdio::inherit() } else { std::process::Stdio::null() })
            .stdout(if tty { std::process::Stdio::inherit() } else { std::process::Stdio::piped() })
            .stderr(if tty { std::process::Stdio::inherit() } else { std::process::Stdio::piped() })
            .spawn()
            .map_err(|e| CrushError::NamespaceError(format!("Failed to exec command: {}", e)))?;

        let exit_status = child.wait().await
            .map_err(|e| CrushError::NamespaceError(format!("Failed to wait for exec: {}", e)))?;

        Ok(exit_status.code().unwrap_or(-1))
    }

    async fn get_pid(&self, container_id: &str) -> Result<Option<u32>> {
        let pids = self.child_pids.lock().await;
        Ok(pids.get(container_id).copied())
    }
}

impl LinuxRuntime {
    pub async fn restore_pids(&self) {
        let containers_dir = dirs_or_default().join("containers");
        if let Ok(entries) = std::fs::read_dir(&containers_dir) {
            for entry in entries.flatten() {
                let pid_file = entry.path().join("pid");
                if let Ok(text) = std::fs::read_to_string(&pid_file) {
                    if let Ok(pid) = text.trim().parse::<u32>() {
                        // Verify the process is still alive
                        #[cfg(unix)]
                        {
                            if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
                                let id = entry.file_name().to_string_lossy().to_string();
                                self.child_pids.lock().await.insert(id, pid);
                            } else {
                                // Process is dead — clean up stale pid file
                                let _ = std::fs::remove_file(&pid_file);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn spawn_container_process(&self, container_id: &str) -> Result<u32> {
        let container_dir = dirs_or_default().join("containers").join(container_id);
        let container_json_path = container_dir.join("container.json");
        if !container_json_path.exists() {
            return Err(CrushError::ContainerNotFound(container_id.to_string()));
        }

        let container_content = std::fs::read_to_string(&container_json_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read container.json: {}", e)))?;
        let container: Container = serde_json::from_str(&container_content)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse container.json: {}", e)))?;

        // Load image config
        let image_json_path = dirs_or_default().join("images").join(&container.image).join("image.json");
        let image: Image = if image_json_path.exists() {
            let image_content = std::fs::read_to_string(&image_json_path)
                .map_err(|e| CrushError::StorageError(format!("Failed to read image.json: {}", e)))?;
            serde_json::from_str(&image_content)
                .map_err(|e| CrushError::StorageError(format!("Failed to parse image.json: {}", e)))?
        } else {
            return Err(CrushError::ContainerNotFound(format!("Image config not found for image '{}'", container.image)));
        };

        let mut command = image.entrypoint.clone();
        command.extend(image.cmd.clone());
        if command.is_empty() {
            command.push("/bin/sh".to_string());
        }

        let rootfs = container_dir.join("rootfs");
        let env = image.env.clone();

        // Run container in a blocking task
        let rootfs_clone = rootfs.clone();
        let command_clone = command.clone();
        let env_clone = env.clone();
        let container_clone = container.clone();
        let container_id_clone = container_id.to_string();

        tokio::task::spawn_blocking(move || {
            let res = runner::run_container(&rootfs_clone, &command_clone, &env_clone, &container_clone);
            // On exit, clean up the pid file
            let pid_file = dirs_or_default().join("containers").join(&container_id_clone).join("pid");
            let _ = std::fs::remove_file(pid_file);
            res
        });

        // Poll for the PID file to be written (up to 2 seconds)
        let pid_path = container_dir.join("pid");
        let mut attempts = 0;
        let pid = loop {
            if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
                if let Ok(p) = pid_str.trim().parse::<u32>() {
                    break p;
                }
            }
            if attempts > 100 {
                return Err(CrushError::NamespaceError("Failed to retrieve spawned container PID".to_string()));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            attempts += 1;
        };

        Ok(pid)
    }

    fn default_syscall_allowlist() -> Vec<String> {
        vec![
            "read","write","open","close","stat","fstat","lstat","poll","lseek","mmap",
            "mprotect","munmap","brk","rt_sigaction","ioctl","pread64","pwrite64","readv",
            "writev","access","pipe","select","sched_yield","nanosleep","exit","exit_group",
            "futex","gettid","tgkill","tkill","getpid","getppid","socket","connect","accept",
            "sendto","recvfrom","sendmsg","recvmsg","bind","listen","epoll_create","epoll_ctl",
            "epoll_wait","clone","execve","wait4","kill","fcntl","flock","getcwd","chdir",
            "fchdir","rename","mkdir","rmdir","creat","link","unlink","symlink","readlink",
            "chmod","fchmod","chown","fchown","umask","gettimeofday","getuid","getgid",
            "setuid","setgid","geteuid","getegid","dup","dup2","dup3","pipe2","eventfd",
            "eventfd2","signalfd","signalfd4","timerfd_create","timerfd_settime",
            "timerfd_gettime","getrandom","capget","capset","prctl","arch_prctl",
            "set_robust_list","get_robust_list","set_tid_address","clock_gettime",
            "clock_getres","clock_nanosleep","sched_getaffinity","sched_setaffinity",
            "sched_getparam","sched_setparam","sched_getscheduler","sched_setscheduler",
            "mlock","munlock","madvise","restart_syscall",
        ].into_iter().map(String::from).collect()
    }
}

fn get_spec_path(container_id: &str) -> PathBuf {
    dirs_or_default().join("containers").join(container_id)
}

fn dirs_or_default() -> PathBuf {
    let base = if cfg!(target_os = "linux") {
        PathBuf::from("/var/lib/crush")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData\\Crush".to_string()))
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".crush")
    };
    std::fs::create_dir_all(&base).ok();
    base
}

fn load_oci_hooks(spec_path: &std::path::Path, stage: &str) -> Vec<lifecycle::OciHook> {
    let config_path = spec_path.join("config.json");
    if !config_path.exists() {
        return Vec::new();
    }
    
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    
    let hook_list = match value.get("hooks").and_then(|h| h.get(stage)).and_then(|s| s.as_array()) {
        Some(arr) => arr,
        None => return Vec::new(),
    };
    
    let mut result = Vec::new();
    for item in hook_list {
        if let Some(path) = item.get("path").and_then(|p| p.as_str()) {
            let args = item.get("args")
                .and_then(|a| a.as_array())
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            let env = item.get("env")
                .and_then(|e| e.as_array())
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            let timeout = item.get("timeout").and_then(|t| t.as_u64().map(|v| v as u32));
            
            result.push(lifecycle::OciHook {
                path: path.to_string(),
                args,
                env,
                timeout,
            });
        }
    }
    
    result
}
