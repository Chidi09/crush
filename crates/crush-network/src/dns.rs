use std::collections::HashMap;
use std::sync::Arc;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError};

pub struct DnsResolver {
    upstream: Vec<IpAddr>,
    records: Arc<Mutex<HashMap<String, Vec<IpAddr>>>>,
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            upstream: Self::read_host_resolv(),
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_container(&self, name: &str, ip: Ipv4Addr) {
        let mut records = self.records.blocking_lock();
        records.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(IpAddr::V4(ip));
    }

    pub fn unregister_container(&self, name: &str) {
        let mut records = self.records.blocking_lock();
        records.remove(name);
    }

    pub fn register_alias(&self, alias: &str, container_name: &str) {
        // ⚠ FIX: acquire lock ONCE to prevent self-deadlock
        let mut records = self.records.blocking_lock();
        let ips = records.get(container_name).cloned().unwrap_or_default();
        if !ips.is_empty() {
            records.insert(alias.to_string(), ips);
        }
    }

    pub async fn resolve(&self, name: &str) -> Option<Vec<IpAddr>> {
        let records = self.records.lock().await;
        records.get(name).cloned()
    }

    pub fn write_resolv_conf(container_root: &Path, nameservers: &[IpAddr]) -> Result<()> {
        let resolv_path = container_root.join("etc").join("resolv.conf");
        std::fs::create_dir_all(resolv_path.parent().unwrap())
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        let mut content = "nameserver 127.0.0.11\noptions ndots:0\n".to_string();
        for ns in nameservers { content.push_str(&format!("nameserver {}\n", ns)); }
        std::fs::write(&resolv_path, content)
            .map_err(|e| CrushError::StorageError(e.to_string()))
    }

    fn read_host_resolv() -> Vec<IpAddr> {
        std::fs::read_to_string("/etc/resolv.conf")
            .map(|c| c.lines()
                .filter_map(|l| l.strip_prefix("nameserver "))
                .filter_map(|ns| ns.trim().parse().ok())
                .collect())
            .unwrap_or_default()
    }
}
