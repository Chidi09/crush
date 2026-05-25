use std::net::Ipv4Addr;
use std::process::Command;
use crush_types::{Result, CrushError, PortMapping};

pub struct PortForwardManager;

impl PortForwardManager {
    pub fn new() -> Self { Self }

    /// Validates that an IP address string matches a valid IPv4 address.
    /// ⚠ CRITICAL: Prevents shell injection via pfctl rule construction.
    fn validate_ip(ip: &str) -> bool {
        let re = regex::Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap();
        if !re.is_match(ip) { return false; }
        let parts: Vec<u8> = ip.split('.').filter_map(|o| o.parse().ok()).collect();
        parts.len() == 4 && parts.iter().all(|&o| o <= 254)
    }

    pub fn add_rule(&self, vm_ip: Ipv4Addr, port: &PortMapping) -> Result<()> {
        #[cfg(target_os = "macos")]
        { self.add_pf_rule(vm_ip, port)?; }
        Ok(())
    }

    pub fn remove_rule(&self, port: &PortMapping) -> Result<()> {
        #[cfg(target_os = "macos")]
        { self.remove_pf_rule(port)?; }
        Ok(())
    }

    pub fn assign_vm_ip(vm_id: &str) -> Result<Ipv4Addr> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        vm_id.hash(&mut hasher);
        let hash = hasher.finish();
        let third = ((hash >> 16) & 0xff) as u8;
        let fourth = ((hash) & 0xff) as u8;
        Ok(Ipv4Addr::new(192, 168, third, fourth))
    }

    #[cfg(target_os = "macos")]
    fn add_pf_rule(&self, vm_ip: Ipv4Addr, port: &PortMapping) -> Result<()> {
        let proto = match port.protocol {
            crush_types::Protocol::Tcp => "tcp",
            crush_types::Protocol::Udp => "udp",
        };

        // ⚠ CRITICAL: No shell injection. Use Command::args() with validated values.
        // Build a minimal pf.conf fragment and pipe it to pfctl -f -
        let pf_conf = format!(
            "rdr pass on lo0 inet proto {} from any to any port {} -> {} port {}\n",
            proto, port.host_port, vm_ip, port.container_port
        );

        let mut child = Command::new("pfctl")
            .args(["-a", "crush", "-f", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| CrushError::NetworkError(format!("pfctl spawn failed: {}", e)))?;

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            stdin.write_all(pf_conf.as_bytes())
                .map_err(|e| CrushError::NetworkError(format!("pfctl write failed: {}", e)))?;
        }

        child.wait()
            .map_err(|e| CrushError::NetworkError(format!("pfctl wait failed: {}", e)))?;

        let _ = Command::new("pfctl").args(["-e"]).output();
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn remove_pf_rule(&self, _port: &PortMapping) -> Result<()> {
        let _ = Command::new("pfctl").args(["-a", "crush", "-F", "nat"]).output();
        Ok(())
    }
}
