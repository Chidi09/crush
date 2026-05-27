use anyhow::{anyhow, Context};
use serde_json::json;
use async_trait::async_trait;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};
use super::ssh::SshProvider;
use crush_build::parser::DeployDigitalOcean;

pub struct DigitalOceanProvider {
    api_token: String,
    ssh_key_name: Option<String>,
}

impl DigitalOceanProvider {
    pub fn new(config: &DeployDigitalOcean) -> Self {
        Self {
            api_token: config.api_token.clone(),
            ssh_key_name: config.ssh_key_name.clone(),
        }
    }

    async fn api_get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://api.digitalocean.com/v2{}", path);
        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(&self.api_token)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_post(&self, path: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://api.digitalocean.com/v2{}", path);
        let resp = reqwest::Client::new()
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_delete(&self, path: &str) -> anyhow::Result<()> {
        let url = format!("https://api.digitalocean.com/v2{}", path);
        reqwest::Client::new()
            .delete(&url)
            .bearer_auth(&self.api_token)
            .send().await?;
        Ok(())
    }

    async fn find_droplet(&self, project: &str) -> anyhow::Result<Option<(String, String)>> {
        let tag = format!("crush-{}", project);
        let data = self.api_get(&format!("/droplets?tag_name={}", tag)).await?;
        if let Some(droplets) = data["droplets"].as_array() {
            if let Some(droplet) = droplets.first() {
                let id = droplet["id"].as_i64().unwrap_or(0).to_string();
                let ip = droplet["networks"]["v4"]
                    .as_array()
                    .and_then(|nets| nets.iter().find(|n| n["type"] == "public"))
                    .and_then(|n| n["ip_address"].as_str())
                    .unwrap_or("")
                    .to_string();
                if !ip.is_empty() {
                    return Ok(Some((id, ip)));
                }
            }
        }
        Ok(None)
    }

    async fn create_droplet(
        &self,
        project: &str,
        region: &str,
        size: &str,
    ) -> anyhow::Result<(String, String)> {
        let tag = format!("crush-{}", project);
        let mut body = json!({
            "name": format!("crush-{}", project),
            "region": region,
            "size": size,
            "image": "ubuntu-22-04-x64",
            "tags": [tag],
            "user_data": "#!/bin/bash\napt-get update -y && apt-get install -y curl\ncurl -fsSL https://github.com/Chidi09/crush/releases/latest/download/install.sh | bash\n"
        });
        if let Some(ref key) = self.ssh_key_name {
            body["ssh_keys"] = json!([key]);
        }
        let resp = self.api_post("/droplets", body).await?;
        if let Some(err) = resp.get("message") {
            return Err(anyhow!("DigitalOcean API error: {}", err));
        }
        let droplet = &resp["droplet"];
        let id = droplet["id"].as_i64().context("no droplet id")?.to_string();

        // Poll for IP — DO doesn't return IP in create response
        for _ in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let data = self.api_get(&format!("/droplets/{}", id)).await?;
            let ip = data["droplet"]["networks"]["v4"]
                .as_array()
                .and_then(|nets| nets.iter().find(|n| n["type"] == "public"))
                .and_then(|n| n["ip_address"].as_str())
                .unwrap_or("")
                .to_string();
            if !ip.is_empty() {
                return Ok((id, ip));
            }
        }
        Err(anyhow!("Timed out waiting for DigitalOcean droplet IP"))
    }
}

#[async_trait]
impl DeployProvider for DigitalOceanProvider {
    async fn provision(&self, project: &str, region: &str, size: &str) -> anyhow::Result<DeploymentInfo> {
        let (server_id, public_ip) = if let Some((id, ip)) = self.find_droplet(project).await? {
            println!("  Reusing existing DigitalOcean droplet {} ({})", id, ip);
            (id, ip)
        } else {
            println!("  Creating DigitalOcean droplet for '{}'...", project);
            let r = if region.is_empty() { "nyc3" } else { region };
            let s = if size.is_empty() { "s-1vcpu-1gb" } else { size };
            let (id, ip) = self.create_droplet(project, r, s).await?;
            println!("  Droplet {} created at {}", id, ip);
            (id, ip)
        };

        Ok(DeploymentInfo {
            provider: "digitalocean".to_string(),
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
        self.api_delete(&format!("/droplets/{}", info.server_id)).await?;
        println!("  DigitalOcean droplet {} deleted.", info.server_id);
        Ok(())
    }

    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let data = self.api_get(&format!("/droplets/{}", info.server_id)).await?;
        let s = data["droplet"]["status"].as_str().unwrap_or("unknown");
        Ok(match s {
            "active" => DeployStatus::Running,
            "off" => DeployStatus::Stopped,
            _ => DeployStatus::Provisioning,
        })
    }

    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let ssh = SshProvider::new(&info.public_ip, 22, "root", None);
        ssh.logs(info, lines).await
    }
}
