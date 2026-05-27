use anyhow::{anyhow, Context};
use serde_json::json;
use async_trait::async_trait;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};
use super::ssh::SshProvider;

pub struct HetznerProvider {
    api_token: String,
    ssh_key_name: Option<String>,
}

impl HetznerProvider {
    pub fn new(api_token: &str, ssh_key_name: Option<&str>) -> Self {
        Self {
            api_token: api_token.to_string(),
            ssh_key_name: ssh_key_name.map(|s| s.to_string()),
        }
    }

    async fn api_get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://api.hetzner.cloud/v1{}", path);
        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(&self.api_token)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_post(&self, path: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://api.hetzner.cloud/v1{}", path);
        let resp = reqwest::Client::new()
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_delete(&self, path: &str) -> anyhow::Result<()> {
        let url = format!("https://api.hetzner.cloud/v1{}", path);
        reqwest::Client::new()
            .delete(&url)
            .bearer_auth(&self.api_token)
            .send().await?;
        Ok(())
    }

    async fn find_server(&self, project: &str) -> anyhow::Result<Option<(String, String)>> {
        let data = self.api_get(&format!("/servers?label_selector=crush-project%3D{}", project)).await?;
        if let Some(servers) = data["servers"].as_array() {
            if let Some(server) = servers.first() {
                let id = server["id"].as_i64().unwrap_or(0).to_string();
                let ip = server["public_net"]["ipv4"]["ip"]
                    .as_str().unwrap_or("").to_string();
                if !ip.is_empty() {
                    return Ok(Some((id, ip)));
                }
            }
        }
        Ok(None)
    }

    async fn create_server(
        &self,
        project: &str,
        server_type: &str,
        datacenter: &str,
        image: &str,
    ) -> anyhow::Result<(String, String)> {
        let mut body = json!({
            "name": format!("crush-{}", project),
            "server_type": server_type,
            "image": image,
            "datacenter": datacenter,
            "labels": { "crush-project": project },
            "user_data": "#!/bin/bash\napt-get update -y && apt-get install -y curl\ncurl -fsSL https://github.com/Chidi09/crush/releases/latest/download/install.sh | bash\n"
        });
        if let Some(ref key) = self.ssh_key_name {
            body["ssh_keys"] = json!([key]);
        }
        let resp = self.api_post("/servers", body).await?;
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("Hetzner API error: {}", err));
        }
        let server = &resp["server"];
        let id = server["id"].as_i64().context("no server id")?.to_string();
        let ip = server["public_net"]["ipv4"]["ip"]
            .as_str().context("no server ip")?.to_string();
        Ok((id, ip))
    }
}

#[async_trait]
impl DeployProvider for HetznerProvider {
    async fn provision(&self, project: &str, region: &str, size: &str) -> anyhow::Result<DeploymentInfo> {
        let (server_id, public_ip) = if let Some((id, ip)) = self.find_server(project).await? {
            println!("  Reusing existing Hetzner server {} ({})", id, ip);
            (id, ip)
        } else {
            println!("  Creating Hetzner server for '{}'...", project);
            let server_type = if size.is_empty() { "cx21" } else { size };
            let datacenter = if region.is_empty() { "nbg1-dc3" } else { region };
            let (id, ip) = self.create_server(project, server_type, datacenter, "ubuntu-22.04").await?;
            println!("  Server {} created at {}", id, ip);
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            (id, ip)
        };

        Ok(DeploymentInfo {
            provider: "hetzner".to_string(),
            project: project.to_string(),
            server_id,
            public_ip,
            region: region.to_string(),
            deployed_at: chrono::Utc::now().to_rfc3339(),
            image_digest: String::new(),
            port: 80,
            domain: None,
            status: DeployStatus::Provisioning,
        })
    }

    async fn deploy(&self, info: &DeploymentInfo, image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()> {
        let ssh = SshProvider::new(&info.public_ip, 22, "root", None);
        ssh.deploy(info, image_tar, port, env).await
    }

    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        self.api_delete(&format!("/servers/{}", info.server_id)).await?;
        println!("  Hetzner server {} deleted.", info.server_id);
        Ok(())
    }

    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let data = self.api_get(&format!("/servers/{}", info.server_id)).await?;
        let s = data["server"]["status"].as_str().unwrap_or("unknown");
        Ok(match s {
            "running" => DeployStatus::Running,
            "off" | "stopped" => DeployStatus::Stopped,
            _ => DeployStatus::Provisioning,
        })
    }

    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let ssh = SshProvider::new(&info.public_ip, 22, "root", None);
        ssh.logs(info, lines).await
    }
}
