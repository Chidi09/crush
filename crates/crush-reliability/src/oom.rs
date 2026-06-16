use std::path::Path;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OomPolicy {
    Restart,
    ReportOnly,
}

#[derive(Debug, Clone)]
pub enum OomEvent {
    None,
    OomKilled { container_id: String, peak_memory: u64, count: u64 },
    ResourceWarn { service: String, rss_bytes: u64, pct_ram: u8 },
}

enum MonitorTarget {
    Cgroup(String),
    HostProcess { pid: u32, service: String },
}

pub struct OomMonitor {
    target: MonitorTarget,
    memory_events_path: String,
    policy: OomPolicy,
    last_oom_count: u64,
    threshold_pct: u8,
    high_samples: u8,
    warning_active: bool,
    total_ram: u64,
}

impl OomMonitor {
    pub fn new(container_id: &str, policy: OomPolicy) -> Self {
        let cgroup = format!("/sys/fs/cgroup/crush/{}", container_id);
        Self {
            target: MonitorTarget::Cgroup(container_id.to_string()),
            memory_events_path: format!("{}/memory.events", cgroup),
            policy,
            last_oom_count: 0,
            threshold_pct: 0,
            high_samples: 0,
            warning_active: false,
            total_ram: 0,
        }
    }

    pub fn new_for_pid(pid: u32, service: &str, threshold_pct: u8) -> Self {
        let total_ram = get_total_ram().unwrap_or(8 * 1024 * 1024 * 1024);
        Self {
            target: MonitorTarget::HostProcess { pid, service: service.to_string() },
            memory_events_path: String::new(),
            policy: OomPolicy::ReportOnly,
            last_oom_count: 0,
            threshold_pct,
            high_samples: 0,
            warning_active: false,
            total_ram,
        }
    }

    pub async fn poll(&mut self) -> Result<OomEvent> {
        match &self.target {
            MonitorTarget::Cgroup(container_id) => {
                let path = Path::new(&self.memory_events_path);
                if !path.exists() {
                    return Ok(OomEvent::None);
                }

                let content = tokio::fs::read_to_string(path).await
                    .map_err(|e| CrushError::CgroupError(format!("Failed to read memory.events: {}", e)))?;

                for line in content.lines() {
                    if let Some(count_str) = line.strip_prefix("oom_kill ") {
                        if let Ok(count) = count_str.trim().parse::<u64>() {
                            if count > self.last_oom_count {
                                let delta = count - self.last_oom_count;
                                self.last_oom_count = count;
                                return Ok(OomEvent::OomKilled {
                                    container_id: container_id.clone(),
                                    peak_memory: self.read_peak_memory().await.unwrap_or(0),
                                    count: delta,
                                });
                            }
                        }
                    }
                }

                Ok(OomEvent::None)
            }
            MonitorTarget::HostProcess { pid, service } => {
                let rss = get_process_rss(*pid);
                if rss == 0 {
                    return Ok(OomEvent::None);
                }

                let pct = ((rss as f64 / self.total_ram as f64) * 100.0) as u8;

                if pct > self.threshold_pct {
                    self.high_samples += 1;
                    if self.high_samples >= 2 && !self.warning_active {
                        self.warning_active = true;
                        return Ok(OomEvent::ResourceWarn {
                            service: service.clone(),
                            rss_bytes: rss,
                            pct_ram: pct,
                        });
                    }
                } else if pct <= self.threshold_pct.saturating_sub(10) {
                    self.high_samples = 0;
                    self.warning_active = false;
                } else {
                    self.high_samples = 0;
                }

                Ok(OomEvent::None)
            }
        }
    }

    async fn read_peak_memory(&self) -> Result<u64> {
        if let MonitorTarget::Cgroup(id) = &self.target {
            let path = format!("/sys/fs/cgroup/crush/{}/memory.peak", id);
            let path = Path::new(&path);
            if path.exists() {
                let content = tokio::fs::read_to_string(path).await
                    .map_err(|e| CrushError::CgroupError(e.to_string()))?;
                content.trim().parse::<u64>()
                    .map_err(|e| CrushError::CgroupError(e.to_string()))
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }
}

fn get_total_ram() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Some(kb * 1024);
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return Some(bytes);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        unsafe {
            let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
            mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut mem_status) != 0 {
                return Some(mem_status.ullTotalPhys);
            }
        }
    }
    None
}

fn get_process_rss(pid: u32) -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/statm", pid)) {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pages) = parts[1].parse::<u64>() {
                    return pages * 4096;
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid.to_string()]).output() {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                let trimmed = line.trim();
                if let Ok(kb) = trimmed.parse::<u64>() {
                    return kb * 1024;
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
        use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
        use windows_sys::Win32::Foundation::CloseHandle;

        let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if handle != 0 {
            let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
            let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let ret = unsafe { GetProcessMemoryInfo(handle, &mut counters as *mut _ as *mut _, size) };
            unsafe { CloseHandle(handle) };
            if ret != 0 {
                return counters.WorkingSetSize as u64;
            }
        }
    }
    0
}
