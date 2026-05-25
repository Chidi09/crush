use std::path::Path;
use std::process::Command;
use crush_types::{Result, CrushError};

const VETH_MTU: u32 = 1500;
const OUI_CRUSH: &str = "06:00";

pub struct VethPair {
    pub host_iface: String,
    pub container_iface: String,
    pub container_mac: String,
}

impl VethPair {
    pub fn create(container_id: &str, bridge_name: &str, netns_path: &Path) -> Result<Self> {
        let host_iface = format!("veth_{}_h", &container_id[..8.min(container_id.len())]);
        let container_iface = "eth0".to_string();
        let mac = Self::generate_mac(container_id);

        let add = Command::new("ip")
            .args(["link", "add", &host_iface, "type", "veth", "peer", "name", &container_iface])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to create veth pair: {}", e)))?;

        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            if !stderr.contains("File exists") {
                return Err(CrushError::NetworkError(format!("veth creation failed: {}", stderr)));
            }
        }

        let set_mac = Command::new("ip")
            .args(["link", "set", "dev", &container_iface, "address", &mac])
            .output();
        if let Ok(out) = set_mac {
            if !out.status.success() {
                eprintln!("Warning: could not set MAC: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let set_mtu = Command::new("ip")
            .args(["link", "set", "dev", &host_iface, "mtu", &VETH_MTU.to_string()])
            .output();
        if let Ok(out) = set_mtu {
            if !out.status.success() {
                eprintln!("Warning: could not set MTU: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let set_container_mtu = Command::new("ip")
            .args(["link", "set", "dev", &container_iface, "mtu", &VETH_MTU.to_string()])
            .output();
        if let Ok(out) = set_container_mtu {
            if !out.status.success() {
                eprintln!("Warning: could not set container MTU: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let attach = Command::new("ip")
            .args(["link", "set", "dev", &host_iface, "master", bridge_name])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to attach veth to bridge: {}", e)))?;

        if !attach.status.success() {
            return Err(CrushError::NetworkError(format!(
                "Failed to attach veth to bridge: {}",
                String::from_utf8_lossy(&attach.stderr)
            )));
        }

        let host_up = Command::new("ip")
            .args(["link", "set", "dev", &host_iface, "up"])
            .output();
        if let Ok(out) = host_up {
            if !out.status.success() {
                eprintln!("Warning: could not bring host veth up: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let peer_in_ns = Command::new("ip")
            .args(["link", "set", "dev", &container_iface, "netns", &netns_path.to_string_lossy()])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("Failed to move peer into netns: {}", e)))?;

        if !peer_in_ns.status.success() {
            return Err(CrushError::NetworkError(format!(
                "Failed to move veth into netns: {}",
                String::from_utf8_lossy(&peer_in_ns.stderr)
            )));
        }

        let ns_up = Command::new("ip")
            .args(["-n", &netns_path.to_string_lossy(), "link", "set", "dev", &container_iface, "up"])
            .output();
        if let Ok(out) = ns_up {
            if !out.status.success() {
                eprintln!("Warning: could not bring container veth up: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        Ok(Self {
            host_iface: host_iface.clone(),
            container_iface: container_iface.to_string(),
            container_mac: mac,
        })
    }

    pub fn delete(host_iface: &str) -> Result<()> {
        let _ = Command::new("ip")
            .args(["link", "delete", host_iface])
            .output();
        Ok(())
    }

    fn generate_mac(container_id: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(container_id.as_bytes());
        let hash = hex::encode(hasher.finalize());
        format!(
            "{}.{}.{}.{}.{}.{}",
            OUI_CRUSH, &hash[0..2], &hash[2..4], &hash[4..6], &hash[6..8], &hash[8..10]
        ).replace('.', ":")
    }
}
