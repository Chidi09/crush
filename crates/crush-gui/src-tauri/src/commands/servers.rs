//! Servers panel — SSH hosts from ~/.ssh/config, surfaced in the GUI so you can
//! see, manage, and connect to your servers without dropping to a terminal.
//!
//! Management (health, containers, logs, restart) runs over the system `ssh`
//! client in BatchMode so it never hangs on a password prompt — key-auth hosts
//! "just work", others fail fast with a message the UI shows.

use serde::Serialize;
use crush_build::ssh::{load_hosts, SshHost};
use tauri::{State, Window};
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;
use crate::state::{AppState, LogTailer};

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
            let name = f.get(1).unwrap_or(&"").trim().to_string();
            // The name often has a leading slash in docker, strip it
            let name = name.strip_prefix('/').unwrap_or(&name).to_string();
            list.push(ServerContainer {
                id: f[0].trim().to_string(),
                name,
                image: f.get(2).unwrap_or(&"").trim().to_string(),
                status: f.get(3).unwrap_or(&"").trim().to_string(),
                ports: f.get(4).unwrap_or(&"").trim().to_string(),
            });
        }
    }
    Ok(list)
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerContainerStat {
    pub name: String,
    pub cpu: String,
    pub mem: String,
}

#[tauri::command]
pub async fn server_container_stats(host: String) -> Result<Vec<ServerContainerStat>, String> {
    let fmt = "docker stats --no-stream --format '{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}'";
    let out = match ssh_exec(&host, fmt) {
        Ok(o) => o,
        Err(_) => return Ok(vec![]),
    };
    let mut list = Vec::new();
    for line in out.lines() {
        let f: Vec<&str> = line.splitn(3, '|').collect();
        if f.len() >= 3 && !f[0].trim().is_empty() {
            list.push(ServerContainerStat {
                name: f[0].trim().to_string(),
                cpu: f[1].trim().to_string(),
                mem: f[2].trim().split(" / ").next().unwrap_or(f[2].trim()).to_string(), // Keep just the used part or the whole thing
            });
        }
    }
    Ok(list)
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeServerService {
    pub name: String,
    pub status: String,
    pub kind: String,
}

/// Running systemd services (or PM2, etc).
#[tauri::command]
pub async fn server_services(host: String) -> Result<Vec<NativeServerService>, String> {
    let fmt = "systemctl list-units --type=service --state=running --no-pager --plain --no-legend 2>/dev/null || echo ''";
    let out = match ssh_exec(&host, fmt) {
        Ok(o) => o,
        Err(_) => return Ok(vec![]),
    };
    let mut list = Vec::new();
    for line in out.lines() {
        if line.trim().is_empty() { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let name = parts[0].strip_suffix(".service").unwrap_or(parts[0]).to_string();
            // Skip common system services to reduce noise, unless you want all of them.
            if name.starts_with("systemd-") { continue; }
            list.push(NativeServerService {
                name,
                status: parts[3].to_string(), // usually "running"
                kind: "systemd".to_string(),
            });
        }
    }
    // Try PM2
    let pm2_cmd = "command -v pm2 >/dev/null 2>&1 && pm2 jlist 2>/dev/null || echo '[]'";
    if let Ok(pm2_out) = ssh_exec(&host, pm2_cmd) {
        if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&pm2_out) {
            if let Some(arr) = arr.as_array() {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                        let status = item.get("pm2_env").and_then(|e| e.get("status")).and_then(|s| s.as_str()).unwrap_or("unknown");
                        list.push(NativeServerService {
                            name: name.to_string(),
                            status: status.to_string(),
                            kind: "pm2".to_string(),
                        });
                    }
                }
            }
        }
    }
    Ok(list)
}

#[tauri::command]
pub async fn server_service_restart(host: String, name: String, kind: String) -> Result<(), String> {
    let safe_name = sanitize_id(&name)?;
    let cmd = match kind.as_str() {
        "systemd" => format!("sudo systemctl restart {safe_name}"),
        "pm2" => format!("pm2 restart {safe_name}"),
        _ => return Err("unknown service kind".into()),
    };
    ssh_exec(&host, &cmd).map(|_| ())
}

/// Tail a container's logs.
#[tauri::command]
pub async fn server_container_logs(host: String, id: String, tail: u32) -> Result<String, String> {
    let safe = sanitize_id(&id)?;
    ssh_exec(&host, &format!("docker logs --tail {tail} {safe} 2>&1"))
}

#[tauri::command]
pub async fn server_container_logs_follow(
    host: String,
    id: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_id(&id)?;
    let key = format!("{}:{}", host, safe_id);

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut tailers = state.log_tailers.write().await;
        tailers.insert(key.clone(), LogTailer { shutdown: shutdown_tx });
    }

    tokio::spawn(async move {
        let mut cmd = tokio::process::Command::new("ssh");
        cmd.args([
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=8",
            "-o", "StrictHostKeyChecking=accept-new",
            &host,
            &format!("docker logs -f --tail 200 {}", safe_id),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        if let Ok(mut child) = cmd.spawn() {
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            let key_emit = key.clone();
            
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        let _ = child.kill().await;
                        break;
                    }
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                crate::events::emit_log_line(&window, &key_emit, "", "stdout", &l);
                            }
                            Ok(None) | Err(_) => break,
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                crate::events::emit_log_line(&window, &key_emit, "", "stderr", &l);
                            }
                            Ok(None) | Err(_) => break,
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn server_container_logs_unfollow(
    host: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let safe_id = sanitize_id(&id)?;
    let key = format!("{}:{}", host, safe_id);
    let mut tailers = state.log_tailers.write().await;
    if let Some(tailer) = tailers.remove(&key) {
        let _ = tailer.shutdown.send(());
    }
    Ok(())
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

/// Open an interactive shell in a container via SSH.
#[tauri::command]
pub async fn server_container_exec(host: String, id: String) -> Result<(), String> {
    if host.trim().is_empty() {
        return Err("no host given".into());
    }
    let safe_id = sanitize_id(&id)?;
    // The command is `ssh -t host docker exec -it id sh`
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        std::process::Command::new("cmd")
            .args(["/k", &format!("ssh -t {host} docker exec -it {safe_id} sh")])
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|e| format!("failed to open terminal: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .args(["-e", &format!("tell app \"Terminal\" to do script \"ssh -t {host} docker exec -it {safe_id} sh\"")])
            .spawn()
            .map_err(|e| format!("failed to open Terminal: {e}"))?;
        Ok(())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        for (bin, args) in [
            ("x-terminal-emulator", vec!["-e".to_string(), format!("ssh -t {host} docker exec -it {safe_id} sh")]),
            ("gnome-terminal", vec!["--".to_string(), "ssh".to_string(), "-t".to_string(), host.clone(), "docker".to_string(), "exec".to_string(), "-it".to_string(), safe_id.clone(), "sh".to_string()]),
            ("konsole", vec!["-e".to_string(), format!("ssh -t {host} docker exec -it {safe_id} sh")]),
            ("xterm", vec!["-e".to_string(), format!("ssh -t {host} docker exec -it {safe_id} sh")]),
        ] {
            if std::process::Command::new(bin).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }
        Err(format!("couldn't open a terminal — run `ssh -t {host} docker exec -it {safe_id} sh` manually"))
    }
}
