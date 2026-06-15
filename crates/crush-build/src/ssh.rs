//! SSH config discovery — shared by the CLI (`crush ssh`) and the GUI Servers
//! panel. Parses `~/.ssh/config` into connectable host entries. Pure string
//! work; the actual connection is the system `ssh` client.

use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// One concrete `Host` block from the SSH config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
}

impl SshHost {
    /// `user@hostname:port` summary (falls back to the alias).
    pub fn target(&self) -> String {
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

fn home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

pub fn ssh_config_path() -> PathBuf {
    home().join(".ssh").join("config")
}

/// Parse `Host` blocks from SSH config text. Concrete aliases only — wildcard
/// patterns (`Host *`) are skipped. Keys are case-insensitive per the spec.
pub fn parse_ssh_config(text: &str) -> Vec<SshHost> {
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current: Option<SshHost> = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, rest) = match line.split_once([' ', '\t', '=']) {
            Some((k, v)) => (k.trim(), v.trim().trim_start_matches('=').trim()),
            None => (line, ""),
        };
        let key_lc = key.to_ascii_lowercase();

        if key_lc == "host" {
            if let Some(h) = current.take() {
                hosts.push(h);
            }
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

/// Configured hosts from `~/.ssh/config` (empty if absent).
pub fn load_hosts() -> Vec<SshHost> {
    std::fs::read_to_string(ssh_config_path())
        .map(|t| parse_ssh_config(&t))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_targets() {
        let cfg = "Host prod\n  HostName 203.0.113.10\n  User deploy\n  Port 2222\nHost db\n  HostName db.internal\n";
        let h = parse_ssh_config(cfg);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].target(), "deploy@203.0.113.10:2222");
        assert_eq!(h[1].target(), "db.internal");
    }

    #[test]
    fn skips_wildcards_and_reads_equals() {
        let cfg = "Host *\n  ForwardAgent yes\nhost=server1\n  hostname=1.1.1.1\n";
        let h = parse_ssh_config(cfg);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].alias, "server1");
        assert_eq!(h[0].hostname.as_deref(), Some("1.1.1.1"));
    }
}
