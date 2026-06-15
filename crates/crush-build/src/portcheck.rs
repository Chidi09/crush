//! Port ownership probe for the EADDRINUSE takeover feature (R1.1).
//!
//! Given a TCP port, returns the PID and process name of the holder, cross-platform:
//!   - Linux:   parse `/proc/net/tcp` + `/proc/net/tcp6` (inode → walk `/proc/*/fd`).
//!   - Windows: `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_ALL)` via `windows-sys`.
//!   - macOS:   `lsof -nP -iTCP:<port> -sTCP:LISTEN -t` fallback.

use std::net::TcpListener;

/// Describes the process currently holding a port.
#[derive(Debug, Clone)]
pub struct PortHolder {
    pub pid: u32,
    /// Best-effort process name; empty string if resolution failed.
    pub process: String,
}

/// Check whether `port` is bound. Returns `Some(PortHolder)` if occupied,
/// `None` if the port is free.
pub fn probe_port(port: u16) -> Option<PortHolder> {
    // Quick free-check first (avoids OS-specific parsing when port is open).
    if is_port_free(port) {
        return None;
    }
    find_port_holder(port)
}

/// Returns `true` if we can bind the port ourselves (i.e. nothing is holding it).
fn is_port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

#[cfg(target_os = "linux")]
fn find_port_holder(port: u16) -> Option<PortHolder> {
    linux::find_holder(port)
}

#[cfg(target_os = "windows")]
fn find_port_holder(port: u16) -> Option<PortHolder> {
    windows::find_holder(port)
}

#[cfg(target_os = "macos")]
fn find_port_holder(port: u16) -> Option<PortHolder> {
    macos::find_holder(port)
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn find_port_holder(_port: u16) -> Option<PortHolder> {
    None
}

// ─── Linux ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::PortHolder;
    use std::fs;

    pub fn find_holder(port: u16) -> Option<PortHolder> {
        let inode = port_inode(port)?;
        let pid = pid_for_inode(inode)?;
        let process = process_name(pid).unwrap_or_default();
        Some(PortHolder { pid, process })
    }

    /// Parse `/proc/net/tcp` and `/proc/net/tcp6` to find the socket inode
    /// bound to `port` in LISTEN state (state field == 0A).
    fn port_inode(port: u16) -> Option<u64> {
        for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
            if let Ok(content) = fs::read_to_string(path) {
                if let Some(inode) = parse_proc_net_tcp(&content, port) {
                    return Some(inode);
                }
            }
        }
        None
    }

    /// Parse one /proc/net/tcp[6] table and return the inode of a LISTEN entry
    /// whose local port matches. The format is:
    ///   sl  local_address  rem_address  st  tx_queue:rx_queue  tr:tm->when  retrnsmt  uid  timeout  inode
    pub fn parse_proc_net_tcp(content: &str, port: u16) -> Option<u64> {
        let port_hex = format!("{:04X}", port);
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 10 { continue; }
            let local = fields[1]; // "XXXXXXXX:PPPP"
            let state = fields[3]; // "0A" = LISTEN
            if state != "0A" { continue; }
            // local = hex-encoded address (LE on Linux) : hex port
            if let Some(lport_hex) = local.split(':').nth(1) {
                if lport_hex.eq_ignore_ascii_case(&port_hex) {
                    if let Ok(inode) = fields[9].parse::<u64>() {
                        return Some(inode);
                    }
                }
            }
        }
        None
    }

    /// Walk /proc/*/fd looking for a symlink to `socket:[inode]`.
    fn pid_for_inode(inode: u64) -> Option<u32> {
        let target = format!("socket:[{}]", inode);
        let Ok(proc) = fs::read_dir("/proc") else { return None };
        for entry in proc.flatten() {
            let pid_str = entry.file_name();
            let pid_str = pid_str.to_string_lossy();
            if !pid_str.chars().all(|c| c.is_ascii_digit()) { continue; }
            let Ok(pid) = pid_str.parse::<u32>() else { continue };
            let fd_dir = entry.path().join("fd");
            let Ok(fds) = fs::read_dir(&fd_dir) else { continue };
            for fd in fds.flatten() {
                if let Ok(link) = fs::read_link(fd.path()) {
                    if link.to_string_lossy() == target {
                        return Some(pid);
                    }
                }
            }
        }
        None
    }

    fn process_name(pid: u32) -> Option<String> {
        let comm = fs::read_to_string(format!("/proc/{}/comm", pid)).ok()?;
        Some(comm.trim().to_string())
    }
}

// ─── Windows ──────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod windows {
    use super::PortHolder;

    pub fn find_holder(port: u16) -> Option<PortHolder> {
        use windows_sys::Win32::NetworkManagement::IpHelper::{
            GetExtendedTcpTable, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
        };
        use windows_sys::Win32::Networking::WinSock::AF_INET;

        let mut buf_size: u32 = 4096;
        let mut buf: Vec<u8> = vec![0u8; buf_size as usize];

        loop {
            let ret = unsafe {
                GetExtendedTcpTable(
                    buf.as_mut_ptr() as *mut _,
                    &mut buf_size,
                    0, // bOrder = false
                    AF_INET as u32,
                    TCP_TABLE_OWNER_PID_LISTENER,
                    0,
                )
            };
            if ret == 0 {
                break;
            }
            if ret == 122 {
                // ERROR_INSUFFICIENT_BUFFER
                buf.resize(buf_size as usize, 0);
            } else {
                return None;
            }
        }

        let table = unsafe { &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID) };
        let count = table.dwNumEntries as usize;
        let rows = unsafe {
            std::slice::from_raw_parts(table.table.as_ptr(), count)
        };

        let port_be = port.to_be() as u32; // dwLocalPort is in network byte order
        for row in rows {
            if row.dwLocalPort == port_be {
                let pid = row.dwOwningPid;
                let process = process_name(pid).unwrap_or_default();
                return Some(PortHolder { pid, process });
            }
        }
        None
    }

    fn process_name(pid: u32) -> Option<String> {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::QueryFullProcessImageNameW;

        let handle: HANDLE = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle == 0 {
            return None;
        }

        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
        unsafe { CloseHandle(handle) };

        if ok == 0 {
            return None;
        }

        let path = String::from_utf16_lossy(&buf[..size as usize]);
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(path);
        Some(name)
    }
}

// ─── macOS ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos {
    use super::PortHolder;
    use std::process::Command;

    pub fn find_holder(port: u16) -> Option<PortHolder> {
        // `lsof -nP -iTCP:<port> -sTCP:LISTEN -t` prints the PIDs, one per line.
        let out = Command::new("lsof")
            .args([
                "-nP",
                &format!("-iTCP:{}", port),
                "-sTCP:LISTEN",
                "-t",
            ])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&out.stdout);
        let pid: u32 = stdout.lines().next()?.trim().parse().ok()?;
        let process = process_name(pid).unwrap_or_default();
        Some(PortHolder { pid, process })
    }

    fn process_name(pid: u32) -> Option<String> {
        let out = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "comm="])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let name = s.trim();
        if name.is_empty() { None } else { Some(name.to_string()) }
    }
}

/// Find a free port starting at `preferred`, returning the first unbound one.
pub fn next_free_port(preferred: u16) -> u16 {
    for candidate in preferred..=65534 {
        if is_port_free(candidate) {
            return candidate;
        }
    }
    0 // unlikely
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_free_when_unbound() {
        // ephemeral port unlikely to be in use
        assert!(is_port_free(19999));
    }

    #[test]
    fn port_busy_when_bound() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(!is_port_free(port));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_proc_net_tcp_fixture() {
        // Real /proc/net/tcp line for a LISTEN socket on port 8080 (0x1F90).
        let fixture = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
                             0: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 42424 1 0000000000000000 100 0 0 10 0\n";
        assert_eq!(linux::parse_proc_net_tcp(fixture, 8080), Some(42424));
        assert_eq!(linux::parse_proc_net_tcp(fixture, 9090), None);
    }

    #[test]
    fn next_free_returns_a_free_port() {
        let p = next_free_port(20000);
        assert!(is_port_free(p));
    }
}
