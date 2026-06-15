//! Servers panel — SSH hosts from ~/.ssh/config, surfaced in the GUI so you can
//! see, manage, and connect to your servers without dropping to a terminal.
//!
//! Management (health, containers, logs, restart) runs over the system `ssh`
//! client in BatchMode so it never hangs on a password prompt — key-auth hosts
//! "just work", others fail fast with a message the UI shows.

use serde::Serialize;
use crush_build::ssh::{load_hosts, SshHost};

/// Run a command on `host` via ssh, returning stdout (or an error string).
fn ssh_exec(host: &str, command: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new("ssh");
    cmd.args([
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=8",
        "-o", "StrictHostKeyChecking=accept-new",
        host, command,
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().map_err(|e| format!("ssh failed to launch: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { "ssh command failed".into() } else { err })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerHealth {
    pub reachable: bool,
    pub os: String,
    pub uptime: String,
    pub cpus: u32,
    pub mem_total_mb: u64,
    pub mem_used_mb: u64,
    pub disk_size: String,
    pub disk_used: String,
    pub disk_pct: String,
    pub has_docker: bool,
    pub error: Option<String>,
}

impl ServerHealth {
    fn unreachable(err: String) -> Self {
        ServerHealth {
            reachable: false, os: String::new(), uptime: String::new(), cpus: 0,
            mem_total_mb: 0, mem_used_mb: 0, disk_size: String::new(),
            disk_used: String::new(), disk_pct: String::new(), has_docker: false,
            error: Some(err),
        }
    }
}

/// CPU/memory/disk/uptime/OS + docker presence, in one ssh round-trip. Output is
/// labeled (`KEY:value`) so parsing is order-independent and tolerant of a
/// missing tool on the remote.
#[tauri::command]
pub async fn server_health(host: String) -> Result<ServerHealth, String> {
    let cmd = "echo CPU:$(nproc 2>/dev/null); \
echo MEM:$(free -m 2>/dev/null | awk '/Mem:/{print $2\",\"$3}'); \
echo DISK:$(df -BG / 2>/dev/null | tail -1 | awk '{print $2\",\"$3\",\"$5}'); \
echo UP:$(uptime -p 2>/dev/null); \
echo OSN:$(. /etc/os-release 2>/dev/null; echo $PRETTY_NAME); \
echo DOCKER:$(command -v docker >/dev/null 2>&1 && echo yes || echo no)";

    let out = match ssh_exec(&host, cmd) {
        Ok(o) => o,
        Err(e) => return Ok(ServerHealth::unreachable(e)),
    };

    let mut h = ServerHealth {
        reachable: true, os: String::new(), uptime: String::new(), cpus: 0,
        mem_total_mb: 0, mem_used_mb: 0, disk_size: String::new(),
        disk_used: String::new(), disk_pct: String::new(), has_docker: false, error: None,
    };
    for line in out.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let v = v.trim();
        match k {
            "CPU" => h.cpus = v.parse().unwrap_or(0),
            "MEM" => {
                let mut it = v.split(',');
                h.mem_total_mb = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
                h.mem_used_mb = it.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
            }
            "DISK" => {
                let mut it = v.split(',');
                h.disk_size = it.next().unwrap_or("").trim().to_string();
                h.disk_used = it.next().unwrap_or("").trim().to_string();
                h.disk_pct = it.next().unwrap_or("").trim().to_string();
            }
            "UP" => h.uptime = v.to_string(),
            "OSN" => h.os = v.trim_matches('"').to_string(),
            "DOCKER" => h.has_docker = v == "yes",
            _ => {}
        }
    }
    Ok(h)
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerContainer {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub ports: String,
}

/// Running containers on the host (`docker ps`). Empty if docker isn't installed.
#[tauri::command]
pub async fn server_containers(host: String) -> Result<Vec<ServerContainer>, String> {
    let fmt = "docker ps --all --format '{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}|{{.Ports}}'";
    let out = match ssh_exec(&host, fmt) {
        Ok(o) => o,
        Err(_) => return Ok(vec![]), // docker missing / not permitted → no containers
    };
    let mut list = Vec::new();
    for line in out.lines() {
        let f: Vec<&str> = line.splitn(5, '|').collect();
        if f.len() >= 4 && !f[0].trim().is_empty() {
            list.push(ServerContainer {
                id: f[0].trim().to_string(),
                name: f.get(1).unwrap_or(&"").trim().to_string(),
                image: f.get(2).unwrap_or(&"").trim().to_string(),
                status: f.get(3).unwrap_or(&"").trim().to_string(),
                ports: f.get(4).unwrap_or(&"").trim().to_string(),
            });
        }
    }
    Ok(list)
}

/// Tail a container's logs.
#[tauri::command]
pub async fn server_container_logs(host: String, id: String, tail: u32) -> Result<String, String> {
    let safe = sanitize_id(&id)?;
    ssh_exec(&host, &format!("docker logs --tail {tail} {safe} 2>&1"))
}

/// Restart a container.
#[tauri::command]
pub async fn server_container_restart(host: String, id: String) -> Result<(), String> {
    let safe = sanitize_id(&id)?;
    ssh_exec(&host, &format!("docker restart {safe}")).map(|_| ())
}

/// Stop a container.
#[tauri::command]
pub async fn server_container_stop(host: String, id: String) -> Result<(), String> {
    let safe = sanitize_id(&id)?;
    ssh_exec(&host, &format!("docker stop {safe}")).map(|_| ())
}

/// Only allow container ids/names (alnum, dash, underscore, dot) through to the
/// remote shell — never arbitrary strings.
fn sanitize_id(id: &str) -> Result<String, String> {
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        Ok(id.to_string())
    } else {
        Err("invalid container id".into())
    }
}

/// Configured SSH hosts (from `~/.ssh/config`).
#[tauri::command]
pub async fn ssh_hosts() -> Result<Vec<SshHost>, String> {
    Ok(load_hosts())
}

/// Open an interactive SSH session to `host` in a new terminal window. The
/// system `ssh` client honours the user's config (keys, ProxyJump, ports).
#[tauri::command]
pub async fn ssh_connect(host: String) -> Result<(), String> {
    if host.trim().is_empty() {
        return Err("no host given".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        std::process::Command::new("cmd")
            .args(["/k", &format!("ssh {host}")])
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|e| format!("failed to open terminal: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        // Tell Terminal.app to run ssh.
        std::process::Command::new("osascript")
            .args(["-e", &format!("tell app \"Terminal\" to do script \"ssh {host}\"")])
            .spawn()
            .map_err(|e| format!("failed to open Terminal: {e}"))?;
        Ok(())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        // Try common Linux terminal emulators; fall back to an error the UI shows.
        for (bin, args) in [
            ("x-terminal-emulator", vec!["-e".to_string(), format!("ssh {host}")]),
            ("gnome-terminal", vec!["--".to_string(), "ssh".to_string(), host.clone()]),
            ("konsole", vec!["-e".to_string(), format!("ssh {host}")]),
            ("xterm", vec!["-e".to_string(), format!("ssh {host}")]),
        ] {
            if std::process::Command::new(bin).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }
        Err(format!("couldn't open a terminal — run `ssh {host}` manually"))
    }
}
