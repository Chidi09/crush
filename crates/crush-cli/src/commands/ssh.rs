//! `crush ssh` — discover hosts from `~/.ssh/config` and connect.
//!
//! The config parser lives in `crush_build::ssh` (shared with the GUI Servers
//! panel). `crush ssh` lists hosts; `crush ssh <host>` execs the system `ssh`
//! client, which already honours your config (keys, ProxyJump, ports).

use anyhow::{anyhow, Result};
use owo_colors::OwoColorize;
use crush_build::ssh::{load_hosts, ssh_config_path};

/// `crush ssh` entry point. With no host, list configured servers; with a host,
/// open an interactive session via the system `ssh` client.
pub fn exec(host: Option<String>, list: bool) -> Result<()> {
    let hosts = load_hosts();

    if host.is_none() || list {
        if hosts.is_empty() {
            println!("No hosts found in {}.", ssh_config_path().display());
            println!("   {} add one, e.g.:\n     Host myserver\n       HostName 1.2.3.4\n       User root", "↳".cyan());
            return Ok(());
        }
        println!("{} from {}", "SSH hosts".bold(), ssh_config_path().display().to_string().dimmed());
        for h in &hosts {
            println!("  {:<24} {}", h.alias.bold(), h.target().dimmed());
        }
        println!("\n   {} connect with: {}", "↳".cyan(), "crush ssh <host>".bold());
        return Ok(());
    }

    let host = host.unwrap();
    if !hosts.iter().any(|h| h.alias == host) && !host.contains('@') && !host.contains('.') {
        eprintln!("{} '{}' isn't in your SSH config.", "⚠".yellow(), host.bold());
        if !hosts.is_empty() {
            eprintln!("   known hosts: {}", hosts.iter().map(|h| h.alias.as_str()).collect::<Vec<_>>().join(", "));
        }
    }

    println!("{} connecting to {}\u{2026}", "→".cyan(), host.bold());
    let status = std::process::Command::new("ssh")
        .arg(&host)
        .status()
        .map_err(|e| anyhow!("failed to launch ssh (is the OpenSSH client installed?): {e}"))?;
    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        }
    }
    Ok(())
}
