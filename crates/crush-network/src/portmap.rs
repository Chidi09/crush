use std::collections::HashSet;
use std::sync::Mutex;
use std::net::TcpListener;
use crush_types::{Result, CrushError, PortMapping, Protocol};

const DYNAMIC_PORT_START: u16 = 49152;
const DYNAMIC_PORT_END: u16 = 65535;

pub struct PortMapper {
    allocated_ports: Mutex<HashSet<u16>>,
}

impl PortMapper {
    pub fn new() -> Self {
        Self {
            allocated_ports: Mutex::new(HashSet::new()),
        }
    }

    pub fn allocate_dynamic_port(&self) -> Result<u16> {
        let mut ports = self.allocated_ports.lock()
            .map_err(|e| CrushError::NetworkError(format!("Lock error: {}", e)))?;

        for port in DYNAMIC_PORT_START..=DYNAMIC_PORT_END {
            if ports.contains(&port) {
                continue;
            }
            if !Self::port_in_use(port) {
                ports.insert(port);
                return Ok(port);
            }
        }

        Err(CrushError::NetworkError("No available ports in dynamic range".to_string()))
    }

    pub fn release_port(&self, port: u16) {
        if let Ok(mut ports) = self.allocated_ports.lock() {
            ports.remove(&port);
        }
    }

    pub fn add_port_mapping(port: &PortMapping, container_ip: &str) -> Result<()> {
        let proto = match port.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        };

        let nft_rule = format!(
            "add rule ip crush_nat prerouting {} dport {} dnat to {}:{}",
            proto, port.host_port, container_ip, port.container_port
        );

        let result = std::process::Command::new("nft")
            .args(&nft_rule.split_whitespace().collect::<Vec<_>>())
            .output();

        if let Ok(ref out) = result {
            if !out.status.success() {
                let _ = std::process::Command::new("iptables")
                    .args(["-t", "nat", "-A", "PREROUTING", "-p", proto,
                        "--dport", &port.host_port.to_string(),
                        "-j", "DNAT", "--to-destination", &format!("{}:{}", container_ip, port.container_port)])
                    .output();
            }
        }

        Ok(())
    }

    pub fn remove_port_mapping(port: &PortMapping, container_ip: &str) -> Result<()> {
        let proto = match port.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        };

        let _ = std::process::Command::new("nft")
            .args(["delete", "rule", "ip", "crush_nat", "prerouting",
                &format!("{} dport {} dnat to {}:{}", proto, port.host_port, container_ip, port.container_port)])
            .output();

        let _ = std::process::Command::new("iptables")
            .args(["-t", "nat", "-D", "PREROUTING", "-p", proto,
                "--dport", &port.host_port.to_string(),
                "-j", "DNAT", "--to-destination", &format!("{}:{}", container_ip, port.container_port)])
            .output();

        Ok(())
    }

    pub fn detect_conflict(host_port: u16) -> bool {
        Self::port_in_use(host_port)
    }

    fn port_in_use(port: u16) -> bool {
        TcpListener::bind(format!("0.0.0.0:{}", port)).is_err()
    }
}
