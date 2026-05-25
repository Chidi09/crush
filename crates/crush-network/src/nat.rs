use std::process::Command;
use crush_types::{Result, CrushError, Protocol};

pub struct NatManager;

impl NatManager {
    pub fn setup_nftables(container_id: &str, bridge_name: &str, ip: &str) -> Result<()> {
        let table = "crush_nat";
        let chain = format!("crush_{}", container_id);

        let create_table = Command::new("nft")
            .args(["add", "table", "ip", table])
            .output();
        if let Ok(ref out) = create_table {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: nftables table create: {}", stderr);
                }
            }
        }

        let add_chain = Command::new("nft")
            .args(["add", "chain", "ip", table, &chain, "{ type nat hook postrouting priority srcnat ; }"])
            .output();
        if let Ok(ref out) = add_chain {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: nftables chain add: {}", stderr);
                }
            }
        }

        let masq_rule = Command::new("nft")
            .args(["add", "rule", "ip", table, &chain, "masquerade"])
            .output();
        if let Ok(ref out) = masq_rule {
            if !out.status.success() {
                eprintln!("Warning: nftables masquerade: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        let forward_chain = format!("crush_forward_{}", container_id);
        let add_fwd = Command::new("nft")
            .args(["add", "chain", "ip", "filter", &forward_chain, "{ type filter hook forward priority filter ; }"])
            .output();
        if let Ok(ref out) = add_fwd {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: nftables forward chain: {}", stderr);
                }
            }
        }

        let accept_rule = Command::new("nft")
            .args(["add", "rule", "ip", "filter", &forward_chain, &format!("ip daddr {} accept", ip)])
            .output();
        if let Ok(ref out) = accept_rule {
            if !out.status.success() {
                eprintln!("Warning: nftables accept rule: {}", String::from_utf8_lossy(&out.stderr));
            }
        }

        Ok(())
    }

    pub fn setup_iptables_fallback(container_id: &str, ip: &str, ports: &[(u16, u16, Protocol)]) -> Result<()> {
        for (host_port, container_port, proto) in ports {
            let proto_str = match proto {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
            };

            let _ = Command::new("iptables")
                .args(["-t", "nat", "-A", "PREROUTING", "-p", proto_str,
                    "--dport", &host_port.to_string(),
                    "-j", "DNAT", "--to-destination", &format!("{}:{}", ip, container_port)])
                .output();

            let _ = Command::new("iptables")
                .args(["-t", "nat", "-A", "POSTROUTING", "-p", proto_str,
                    "--dport", &container_port.to_string(),
                    "-j", "MASQUERADE"])
                .output();

            let _ = Command::new("iptables")
                .args(["-A", "FORWARD", "-p", proto_str,
                    "-d", ip, "--dport", &container_port.to_string(),
                    "-j", "ACCEPT"])
                .output();
        }

        let _ = Command::new("iptables")
            .args(["-A", "FORWARD", "-i", "crush0", "-j", "ACCEPT"])
            .output();

        let _ = Command::new("iptables")
            .args(["-A", "FORWARD", "-o", "crush0", "-j", "ACCEPT"])
            .output();

        Ok(())
    }

    pub fn cleanup_nftables(container_id: &str) -> Result<()> {
        let _ = Command::new("nft")
            .args(["delete", "chain", "ip", "crush_nat", &format!("crush_{}", container_id)])
            .output();
        let _ = Command::new("nft")
            .args(["delete", "chain", "ip", "filter", &format!("crush_forward_{}", container_id)])
            .output();
        Ok(())
    }

    pub fn cleanup_iptables(container_ip: &str) -> Result<()> {
        let _ = Command::new("iptables-save")
            .output()
            .map(|o| {
                let rules = String::from_utf8_lossy(&o.stdout);
                for line in rules.lines() {
                    if line.contains(container_ip) {
                        let args: Vec<&str> = line.split_whitespace().collect();
                        let _ = Command::new("iptables")
                            .args(args.iter().map(|a| *a))
                            .output();
                    }
                }
            });
        Ok(())
    }
}
