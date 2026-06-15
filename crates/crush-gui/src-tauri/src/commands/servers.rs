//! Servers panel — SSH hosts from ~/.ssh/config, surfaced in the GUI so you can
//! see and connect to your servers without dropping to a terminal first.

use crush_build::ssh::{load_hosts, SshHost};

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
