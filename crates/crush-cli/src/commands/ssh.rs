//! `crush ssh` — discover hosts from the user's `~/.ssh/config` and connect.
//!
//! Crush already knows how to deploy over SSH; this surfaces the same servers
//! you've configured for interactive use. `crush ssh` lists the hosts in your
//! SSH config; `crush ssh <host>` opens a session by execing the system `ssh`
//! client, which already honours your config (keys, ProxyJump, ports), so we
//! don't reimplement any of it.

use std::path::PathBuf;
use anyhow::{anyhow, Result};
use owo_colors::OwoColorize;

/// One `Host` block from the SSH config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
}

impl SshHost {
    /// `user@hostname:port` summary for display (falls back to the alias).
    fn target(&self) -> String {
        let host = self.hostname.clone().unwrap_or_else(|| self.alias.clone());
        let with_user = match &self.user {
            Some(u) => format!("{u}@{host}"),
            None => host,
        };
        match self.port {
            Some(p) if p != 22 => format!("{with_user}:{p}"),
            _ => with_user,
        }
    }
}

fn ssh_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".ssh").join("config")
}

/// Parse `Host` blocks from SSH config text. Concrete aliases only — wildcard
/// patterns (`Host *`, `Host *.example.com`) are skipped since they aren't
/// connectable targets. Keys are case-insensitive per the SSH spec.
pub fn parse_ssh_config(text: &str) -> Vec<SshHost> {
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current: Option<SshHost> = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Tokens are whitespace- (or `=`-) separated.
        let (key, rest) = match line.split_once([' ', '\t', '=']) {
            Some((k, v)) => (k.trim(), v.trim().trim_start_matches('=').trim()),
            None => (line, ""),
        };
        let key_lc = key.to_ascii_lowercase();

        if key_lc == "host" {
            if let Some(h) = current.take() {
                hosts.push(h);
            }
            // A Host line can list several aliases; take the first concrete one.
            let alias = rest
                .split_whitespace()
                .find(|a| !a.contains('*') && !a.contains('?'));
            current = alias.map(|a| SshHost {
                alias: a.to_string(),
                hostname: None,
                user: None,
                port: None,
            });
        } else if let Some(h) = current.as_mut() {
            match key_lc.as_str() {
                "hostname" => h.hostname = Some(rest.to_string()),
                "user" => h.user = Some(rest.to_string()),
                "port" => h.port = rest.parse().ok(),
                _ => {}
            }
        }
    }
    if let Some(h) = current.take() {
        hosts.push(h);
    }
    hosts
}

/// Load configured hosts from `~/.ssh/config` (empty if absent).
pub fn load_hosts() -> Vec<SshHost> {
    std::fs::read_to_string(ssh_config_path())
        .map(|t| parse_ssh_config(&t))
        .unwrap_or_default()
}

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
    // Validate against the config so typos get a helpful list instead of a hang.
    if !hosts.iter().any(|h| h.alias == host) {
        // Allow ad-hoc targets too (user@ip), but warn if it's neither.
        if !host.contains('@') && !host.contains('.') {
            eprintln!("{} '{}' isn't in your SSH config.", "⚠".yellow(), host.bold());
            if !hosts.is_empty() {
                eprintln!("   known hosts: {}", hosts.iter().map(|h| h.alias.as_str()).collect::<Vec<_>>().join(", "));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hosts_with_fields() {
        let cfg = "\
Host prod
    HostName 203.0.113.10
    User deploy
    Port 2222

Host db
    HostName db.internal
";
        let hosts = parse_ssh_config(cfg);
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].alias, "prod");
        assert_eq!(hosts[0].hostname.as_deref(), Some("203.0.113.10"));
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert_eq!(hosts[0].port, Some(2222));
        assert_eq!(hosts[0].target(), "deploy@203.0.113.10:2222");
        assert_eq!(hosts[1].alias, "db");
        assert_eq!(hosts[1].target(), "db.internal");
    }

    #[test]
    fn skips_wildcard_blocks() {
        let cfg = "Host *\n  ForwardAgent yes\nHost real\n  HostName x.com\n";
        let hosts = parse_ssh_config(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "real");
    }

    #[test]
    fn handles_equals_and_case_insensitive_keys() {
        let cfg = "host=server1\n  HOSTNAME=1.1.1.1\n  user=root\n";
        let hosts = parse_ssh_config(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "server1");
        assert_eq!(hosts[0].hostname.as_deref(), Some("1.1.1.1"));
        assert_eq!(hosts[0].user.as_deref(), Some("root"));
    }

    #[test]
    fn ignores_comments_and_blanks() {
        let cfg = "# a comment\n\nHost h1\n  # nested comment\n  HostName a.b\n";
        let hosts = parse_ssh_config(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("a.b"));
    }

    #[test]
    fn multi_alias_takes_first_concrete() {
        let cfg = "Host web web-prod *.web\n  HostName w.com\n";
        let hosts = parse_ssh_config(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "web");
    }
}
