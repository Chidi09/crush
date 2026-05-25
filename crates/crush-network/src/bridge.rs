use std::process::Command;
use std::net::Ipv4Addr;
use crush_types::{Result, CrushError};

pub struct BridgeManager;

impl BridgeManager {
    pub fn ensure_bridge(name: &str, subnet: &str) -> Result<()> {
        let add = Command::new("ip")
            .args(["link", "add", name, "type", "bridge"])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to add bridge: {}", e)))?;
        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            if !stderr.contains("File exists") {
                return Err(CrushError::NetworkError(format!("Bridge add failed: {}", stderr)));
            }
        }

        let up = Command::new("ip")
            .args(["link", "set", name, "up"])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to bring bridge up: {}", e)))?;
        if !up.status.success() {
            return Err(CrushError::NetworkError(format!(
                "Bridge up failed: {}", String::from_utf8_lossy(&up.stderr)
            )));
        }

        let addr = Command::new("ip")
            .args(["addr", "show", "dev", name])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to check bridge addr: {}", e)))?;
        let addr_str = String::from_utf8_lossy(&addr.stdout);

        let gateway = subnet.split('/').next().unwrap_or("172.17.0.1");
        if !addr_str.contains(gateway) {
            let add_addr = Command::new("ip")
                .args(["addr", "add", subnet, "dev", name])
                .output()
                .map_err(|e| CrushError::NetworkError(format!("Failed to add bridge addr: {}", e)))?;
            if !add_addr.status.success() {
                return Err(CrushError::NetworkError(format!(
                    "Bridge addr add failed: {}", String::from_utf8_lossy(&add_addr.stderr)
                )));
            }
        }

        let arp_proxy = Command::new("sysctl")
            .args(["-w", &format!("net.ipv4.conf.{}.proxy_arp=1", name)])
            .output();
        if let Ok(out) = arp_proxy {
            if !out.status.success() {
                eprintln!("Warning: ARP proxy enable failed: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let ip_forward = Command::new("sysctl")
            .args(["-w", "net.ipv4.ip_forward=1"])
            .output();
        if let Ok(out) = ip_forward {
            if !out.status.success() {
                eprintln!("Warning: ip_forward enable failed: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        Ok(())
    }

    pub fn delete_bridge(name: &str) -> Result<()> {
        let set_down = Command::new("ip")
            .args(["link", "set", name, "down"])
            .output();
        let del = Command::new("ip")
            .args(["link", "delete", name])
            .output();
        if let Ok(out) = del {
            if !out.status.success() {
                if let Ok(ref down) = set_down {
                    let _ = down.status;
                }
                return Err(CrushError::NetworkError(format!(
                    "Bridge delete failed: {}", String::from_utf8_lossy(&out.stderr)
                )));
            }
        }
        Ok(())
    }

    pub fn allocate_ip(subnet: &str, container_id: &str) -> Result<String> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(container_id.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let base: Ipv4Addr = subnet.split('/').next().unwrap_or("172.17.0.0")
            .parse()
            .map_err(|_| CrushError::NetworkError("Invalid subnet".to_string()))?;
        let octets = base.octets();

        let third = (u8::from_str_radix(&hash[0..2], 16).unwrap_or(0))
            .max(1u8);
        let fourth = u8::from_str_radix(&hash[2..4], 16).unwrap_or(2)
            .max(2u8);

        Ok(format!("{}.{}.{}.{}/16", octets[0], octets[1], third, fourth))
    }

    pub fn set_container_ip(netns: &str, iface: &str, ip: &str) -> Result<()> {
        let add = Command::new("ip")
            .args(["-n", netns, "addr", "add", ip, "dev", iface])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to set container IP: {}", e)))?;
        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            if !stderr.contains("File exists") {
                return Err(CrushError::NetworkError(format!("IP add failed: {}", stderr)));
            }
        }

        let route = Command::new("ip")
            .args(["-n", netns, "route", "add", "default", "via", "172.17.0.1"])
            .output();
        if let Ok(out) = route {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: default route add failed: {}", stderr);
                }
            }
        }

        Ok(())
    }
}
