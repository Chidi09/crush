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
use crush_types::{RuntimeBackend, Container, Result, CrushError, ContainerStatus};

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
        Self {
            ns: NamespaceManager::new(),
            user_ns: UserNamespaceManager::new(),
            seccomp: SeccompFilterCompiler::new(),
            caps: CapabilitiesManager::new(),
            signals: SignalHandler::new(),
            lifecycle: ContainerLifecycleManager::new(),
            child_pids: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
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
            cgroup.enforce_cpu_limit(cpu_shares)?;
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
    async fn spawn_container_process(&self, container_id: &str) -> Result<u32> {
        // Compile the seccomp BPF filter here in the parent.
        // It will be applied in the child after fork via pre_exec, before execve.
        let whitelist = Self::default_syscall_allowlist();
        let mut _blocked = 0usize;
        let bpf_bytes = self.seccomp.compile_bpf_filter(&whitelist, &mut _blocked)?;

        let mut cmd = std::process::Command::new("/sbin/init");
        cmd.arg(container_id)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Apply seccomp in the child process (after fork, before execve).
        // This is the correct and only safe place — pre_exec runs exclusively in the child.
        #[cfg(target_os = "linux")]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(move || {
                let filter_count = bpf_bytes.len() / 8; // SockFilter = 8 bytes
                let prog = libc::sock_fprog {
                    len: filter_count as u16,
                    filter: bpf_bytes.as_ptr() as *mut libc::sock_filter,
                };
                // SECCOMP_MODE_FILTER = 2
                let ret = libc::prctl(libc::PR_SET_SECCOMP, 2u64, &prog as *const _ as u64);
                if ret != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let child = tokio::process::Command::from(cmd).spawn();

        match child {
            Ok(mut c) => {
                let pid = c.id().ok_or_else(|| {
                    CrushError::NamespaceError("Failed to get child PID".to_string())
                })?;
                tokio::spawn(async move { let _ = c.wait().await; });
                Ok(pid)
            }
            Err(e) => Err(CrushError::NamespaceError(format!(
                "Failed to spawn container process: {}. Is /sbin/init available?", e
            ))),
        }
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
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("crush")
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
