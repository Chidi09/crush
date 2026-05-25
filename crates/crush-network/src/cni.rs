use std::path::Path;
use std::process::Command;
use crush_types::{Result, CrushError};

const CNI_DIR: &str = "/opt/cni/bin";
const CNI_CONF_DIR: &str = "/etc/cni/net.d";

pub struct CniPlugin;

impl CniPlugin {
    pub fn invoke_add(plugin: &str, container_id: &str, netns: &str, ifname: &str) -> Result<CniResult> {
        let plugin_path = Path::new(CNI_DIR).join(plugin);
        if !plugin_path.exists() {
            return Err(CrushError::NetworkError(format!(
                "CNI plugin not found: {:?}. Install CNI plugins to {}", plugin_path, CNI_DIR
            )));
        }

        let stdin = serde_json::json!({
            "cniVersion": "1.0.0",
            "name": "crush",
            "type": plugin,
            "containerId": container_id,
            "netns": format!("/run/crush/netns/{}", netns),
            "ifName": ifname,
            "args": {
                "IgnoreUnknown": "1"
            }
        });

        let output = Command::new(&plugin_path)
            .env("CNI_COMMAND", "ADD")
            .env("CNI_CONTAINERID", container_id)
            .env("CNI_NETNS", format!("/run/crush/netns/{}", netns))
            .env("CNI_IFNAME", ifname)
            .env("CNI_PATH", CNI_DIR)
            .arg(&stdin.to_string())
            .output()
            .map_err(|e| CrushError::NetworkError(format!("CNI plugin execution failed: {}", e)))?;

        if !output.status.success() {
            return Err(CrushError::NetworkError(format!(
                "CNI plugin '{}' ADD failed: {}", plugin, String::from_utf8_lossy(&output.stderr)
            )));
        }

        let result: CniResult = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| CniResult {
                ips: vec![],
                dns: CniDns { nameservers: vec![] },
            });

        Ok(result)
    }

    pub fn invoke_del(plugin: &str, container_id: &str, netns: &str, ifname: &str) -> Result<()> {
        let plugin_path = Path::new(CNI_DIR).join(plugin);
        if !plugin_path.exists() {
            return Ok(());
        }

        let _ = Command::new(&plugin_path)
            .env("CNI_COMMAND", "DEL")
            .env("CNI_CONTAINERID", container_id)
            .env("CNI_NETNS", format!("/run/crush/netns/{}", netns))
            .env("CNI_IFNAME", ifname)
            .env("CNI_PATH", CNI_DIR)
            .output();

        Ok(())
    }

    pub fn list_plugins() -> Result<Vec<String>> {
        let mut plugins = Vec::new();
        if let Ok(entries) = std::fs::read_dir(CNI_DIR) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                plugins.push(name);
            }
        }
        Ok(plugins)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CniResult {
    pub ips: Vec<CniIp>,
    pub dns: CniDns,
}

#[derive(Debug, serde::Deserialize)]
pub struct CniIp {
    pub address: String,
    pub gateway: Option<String>,
    pub interface: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CniDns {
    pub nameservers: Vec<String>,
}
