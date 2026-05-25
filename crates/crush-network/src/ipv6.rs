use std::process::Command;
use crush_types::{Result, CrushError};

pub struct Ipv6Manager;

impl Ipv6Manager {
    pub fn enable_dual_stack(bridge_name: &str) -> Result<()> {
        let enable_ipv6 = Command::new("sysctl")
            .args(["-w", &format!("net.ipv6.conf.{}.disable_ipv6=0", bridge_name)])
            .output();
        if let Ok(ref out) = enable_ipv6 {
            if !out.status.success() {
                eprintln!("Warning: could not enable IPv6 on bridge: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let ula_addr = format!("fd00:dead:beef::1/64");
        let add_ula = Command::new("ip")
            .args(["-6", "addr", "add", &ula_addr, "dev", bridge_name])
            .output();
        if let Ok(ref out) = add_ula {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: could not add ULA to bridge: {}", stderr);
                }
            }
        }

        let forward_ipv6 = Command::new("sysctl")
            .args(["-w", "net.ipv6.conf.all.forwarding=1"])
            .output();
        if let Ok(ref out) = forward_ipv6 {
            if !out.status.success() {
                eprintln!("Warning: could not enable IPv6 forwarding: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        Ok(())
    }

    pub fn assign_container_ipv6(container_id: &str, netns: &str, veth: &str) -> Result<String> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(container_id.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let suffix = format!("{}:{}:{}:{}", &hash[0..4], &hash[4..8], &hash[8..12], &hash[12..16]);
        let addr = format!("fd00:dead:beef:{}:{}:{}:{}:{}",
            &hash[16..20], &hash[20..24], &hash[24..28], &hash[28..32]);

        let add = Command::new("ip")
            .args(["-n", netns, "-6", "addr", "add", &format!("{}/64", &addr), "dev", veth])
            .output();
        if let Ok(ref out) = add {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: IPv6 addr add: {}", stderr);
                }
            }
        }

        Ok(addr)
    }
}
