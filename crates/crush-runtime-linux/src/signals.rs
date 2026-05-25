use crush_types::{Result, CrushError};

#[cfg(target_os = "linux")]
use nix::sys::signal::{kill, Signal};
#[cfg(target_os = "linux")]
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
#[cfg(target_os = "linux")]
use nix::unistd::Pid;

pub struct SignalHandler;

impl SignalHandler {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "linux")]
    pub fn reap_zombies(&self) {
        loop {
            match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, code)) => {
                    println!("Signals: Reaped exited child process PID {} (exit status: {})", pid, code);
                }
                Ok(WaitStatus::Signaled(pid, sig, _core)) => {
                    println!("Signals: Reaped signaled child process PID {} (signal code: {:?})", pid, sig);
                }
                Ok(WaitStatus::StillAlive) | Err(_) => {
                    break;
                }
                _ => {}
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn reap_zombies(&self) {
        // Cross-compilation mock target for non-Linux hosts
    }

    #[cfg(target_os = "linux")]
    pub fn forward_signal(&self, target_pid: u32, sig: i32) -> Result<()> {
        let raw_signal = match sig {
            15 => Signal::SIGTERM,
            9 => Signal::SIGKILL,
            2 => Signal::SIGINT,
            _ => Signal::SIGTERM,
        };

        kill(Pid::from_raw(target_pid as i32), raw_signal)
            .map_err(|e| CrushError::NamespaceError(format!("Failed to signal container process: {}", e)))?;
            
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn forward_signal(&self, _target_pid: u32, _sig: i32) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub async fn shutdown_container_gracefully(&self, pid: u32, timeout_seconds: u32) -> Result<()> {
        if let Err(e) = self.forward_signal(pid, 15) {
            println!("Signals: Handled dead process exception: {}", e);
            return Ok(());
        }

        let check_interval = std::time::Duration::from_millis(200);
        let max_ticks = (timeout_seconds * 1000) / 200;

        for _ in 0..max_ticks {
            tokio::time::sleep(check_interval).await;
            
            match waitpid(Pid::from_raw(pid as i32), Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::StillAlive) => {
                    // Process is still running, keep waiting
                }
                _ => {
                    // Process exited successfully or exited by signal
                    return Ok(());
                }
            }
        }

        // Force SIGKILL if grace period runs out
        let _ = self.forward_signal(pid, 9);
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn shutdown_container_gracefully(&self, _pid: u32, _timeout_seconds: u32) -> Result<()> {
        // Cross-compilation mock target for non-Linux hosts
        Ok(())
    }
}
