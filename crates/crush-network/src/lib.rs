pub mod netns;
pub mod veth;
pub mod bridge;
pub mod nat;
pub mod ebpf;
pub mod portmap;
pub mod dns;
pub mod networks;
pub mod ipv6;
pub mod cni;
pub mod modes;

use std::path::PathBuf;
use crush_types::{Result, CrushError, PortMapping, Protocol, ContainerStatus};
use netns::NetworkNamespace;
use veth::VethPair;
use bridge::BridgeManager;
use nat::NatManager;
use portmap::PortMapper;
use dns::DnsResolver;
pub use networks::NetworkManager;
use modes::NetworkMode;

const DEFAULT_BRIDGE: &str = "crush0";
const DEFAULT_SUBNET: &str = "172.17.0.1/16";

pub struct ContainerNetwork {
    pub container_id: String,
    pub mode: NetworkMode,
    pub bridge_name: String,
    pub subnet: String,
    pub container_ip: Option<String>,
    pub container_ipv6: Option<String>,
    pub veth_host: Option<String>,
    pub netns: Option<String>,
}

pub struct NetworkOrchestrator {
    bridge_name: String,
    subnet: String,
    port_mapper: PortMapper,
    dns: DnsResolver,
    networks: NetworkManager,
    data_dir: PathBuf,
}

impl NetworkOrchestrator {
    pub fn new(data_dir: PathBuf) -> Self {
        let networks = NetworkManager::new(data_dir.join("networks"));

        Self {
            bridge_name: DEFAULT_BRIDGE.to_string(),
            subnet: DEFAULT_SUBNET.to_string(),
            port_mapper: PortMapper::new(),
            dns: DnsResolver::new(),
            networks,
            data_dir,
        }
    }

    pub async fn setup_container_network(
        &self,
        container_id: &str,
        container_name: &str,
        mode: NetworkMode,
        ports: &[PortMapping],
    ) -> Result<ContainerNetwork> {
        mode.validate()?;

        match mode {
            NetworkMode::Bridge => {
                self.setup_bridge_network(container_id, container_name, ports).await
            }
            NetworkMode::Host => {
                Ok(ContainerNetwork {
                    container_id: container_id.to_string(),
                    mode: NetworkMode::Host,
                    bridge_name: String::new(),
                    subnet: String::new(),
                    container_ip: None,
                    container_ipv6: None,
                    veth_host: None,
                    netns: None,
                })
            }
            NetworkMode::None => {
                Ok(ContainerNetwork {
                    container_id: container_id.to_string(),
                    mode: NetworkMode::None,
                    bridge_name: String::new(),
                    subnet: String::new(),
                    container_ip: None,
                    container_ipv6: None,
                    veth_host: None,
                    netns: None,
                })
            }
            NetworkMode::Container(ref id) => {
                Ok(ContainerNetwork {
                    container_id: container_id.to_string(),
                    mode: NetworkMode::Container(id.clone()),
                    bridge_name: String::new(),
                    subnet: String::new(),
                    container_ip: None,
                    container_ipv6: None,
                    veth_host: None,
                    netns: None,
                })
            }
        }
    }

    async fn setup_bridge_network(
        &self,
        container_id: &str,
        container_name: &str,
        ports: &[PortMapping],
    ) -> Result<ContainerNetwork> {
        BridgeManager::ensure_bridge(&self.bridge_name, &self.subnet)?;

        let netns = NetworkNamespace::create(container_id)?;
        netns.bind_mount()?;

        let veth = VethPair::create(
            container_id,
            &self.bridge_name,
            netns.path(),
        )?;

        let container_ip = BridgeManager::allocate_ip(&self.subnet, container_id)?;
        let ip_addr = container_ip.split('/').next().unwrap_or("172.17.0.2");

        BridgeManager::set_container_ip(
            container_id,
            &veth.container_iface,
            &container_ip,
        )?;

        NatManager::setup_nftables(container_id, &self.bridge_name, ip_addr)?;

        if !ports.is_empty() {
            let port_tuples: Vec<(u16, u16, Protocol)> = ports.iter()
                .map(|p| (p.host_port, p.container_port, p.protocol))
                .collect();

            NatManager::setup_iptables_fallback(container_id, ip_addr, &port_tuples)?;

            for port in ports {
                PortMapper::add_port_mapping(port, ip_addr)?;
            }
        }

        ipv6::Ipv6Manager::enable_dual_stack(&self.bridge_name)?;
        let ipv6_addr = ipv6::Ipv6Manager::assign_container_ipv6(
            container_id, container_id, &veth.container_iface,
        ).ok();

        let dns_servers = vec![
            "127.0.0.11".parse().unwrap(),
            "8.8.8.8".parse().unwrap(),
        ];
        let container_root = self.data_dir.join("containers").join(container_id);
        DnsResolver::write_resolv_conf(&container_root, &dns_servers).ok();

        self.dns.register_container(container_name, ip_addr.parse().unwrap());

        Ok(ContainerNetwork {
            container_id: container_id.to_string(),
            mode: NetworkMode::Bridge,
            bridge_name: self.bridge_name.clone(),
            subnet: self.subnet.clone(),
            container_ip: Some(container_ip),
            container_ipv6: ipv6_addr,
            veth_host: Some(veth.host_iface),
            netns: Some(container_id.to_string()),
        })
    }

    pub async fn teardown_container_network(&self, net: &ContainerNetwork) -> Result<()> {
        if let Some(ref veth_host) = net.veth_host {
            let _ = VethPair::delete(veth_host);
        }
        if let Some(ref container_ip) = net.container_ip {
            let ip = container_ip.split('/').next().unwrap_or(container_ip);
            let _ = NatManager::cleanup_nftables(&net.container_id);
            let _ = NatManager::cleanup_iptables(ip);
        }
        if let Some(ref ns_id) = net.netns {
            let _ = NetworkNamespace::delete(ns_id);
        }
        Ok(())
    }

    pub fn update_status(&self, _status: ContainerStatus) {
    }

    pub fn networks(&self) -> &NetworkManager {
        &self.networks
    }

    pub fn dns(&self) -> &DnsResolver {
        &self.dns
    }
}
