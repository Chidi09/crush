use std::path::Path;
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct AppArmorProfile;

impl AppArmorProfile {
    pub fn is_enabled() -> bool {
        let path = Path::new("/sys/module/apparmor/parameters/enabled");
        path.exists() && std::fs::read_to_string(path).map(|s| s.trim() == "Y").unwrap_or(false)
    }

    pub fn default_profile() -> String {
        r#"
#include <tunables/global>

profile crush-default flags=(attach_disconnected,mediate_deleted) {
  #include <abstractions/base>
  #include <abstractions/lxc/container-base>

  # Network
  network inet tcp,
  network inet udp,
  network inet6 tcp,
  network inet6 udp,
  network netlink raw,

  # Required for most containers
  / r,
  /** rw,
  /proc/ r,
  /proc/** rw,
  /sys/ r,
  /sys/** rw,
  /dev/** rw,
  /tmp/** rw,

  # Deny sensitive operations
  deny /sys/kernel/security/** rw,
  deny /sys/kernel/debug/** rw,
  deny cap_sys_admin,
  deny cap_net_admin,
  deny ptrace,
}
"#.to_string()
    }

    pub fn load_profile(name: &str, profile_content: &str) -> Result<()> {
        if !Self::is_enabled() {
            return Err(CrushError::NamespaceError("AppArmor not enabled on host".to_string()));
        }

        let mut child = Command::new("apparmor_parser")
            .args(["-r", "-W", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| CrushError::NamespaceError(format!("apparmor_parser error: {}", e)))?;

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            stdin.write_all(profile_content.as_bytes())
                .map_err(|e| CrushError::NamespaceError(format!("stdin write: {}", e)))?;
        }

        let status = child.wait()
            .map_err(|e| CrushError::NamespaceError(format!("apparmor_parser wait: {}", e)))?;

        if !status.success() {
            return Err(CrushError::NamespaceError(format!("Profile '{}' failed to load", name)));
        }

        Ok(())
    }

    pub fn unload_profile(name: &str) -> Result<()> {
        if !Self::is_enabled() { return Ok(()); }
        Command::new("apparmor_parser")
            .args(["-R", name])
            .output()
            .map_err(|e| CrushError::NamespaceError(format!("Failed to unload profile: {}", e)))?;
        Ok(())
    }
}
