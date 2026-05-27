use anyhow::{anyhow, Context};
use async_trait::async_trait;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};
use super::ssh::SshProvider;
use crush_build::parser::DeployGcp;

pub struct GcpProvider {
    project_id: String,
    zone: String,
    machine_type: String,
    service_account_key: Option<String>,
}

impl GcpProvider {
    pub fn new(config: &DeployGcp) -> Self {
        Self {
            project_id: config.project_id.clone(),
            zone: config.zone.clone(),
            machine_type: config.machine_type.clone().unwrap_or_else(|| "e2-micro".to_string()),
            service_account_key: config.service_account_key.clone(),
        }
    }

    async fn get_token(&self) -> anyhow::Result<String> {
        // Use gcloud CLI for auth — simplest approach that works with both
        // service account JSON and ADC (Application Default Credentials)
        if let Some(ref key_path) = self.service_account_key {
            let out = tokio::process::Command::new("gcloud")
                .args(["auth", "activate-service-account", "--key-file", key_path])
                .output().await
                .context("gcloud CLI not found — install Google Cloud SDK")?;
            if !out.status.success() {
                return Err(anyhow!("gcloud auth failed: {}", String::from_utf8_lossy(&out.stderr)));
            }
        }
        let out = tokio::process::Command::new("gcloud")
            .args(["auth", "print-access-token"])
            .output().await
            .context("gcloud CLI not found")?;
        if !out.status.success() {
            return Err(anyhow!("gcloud print-access-token failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    async fn api_get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let token = self.get_token().await?;
        let url = format!("https://compute.googleapis.com/compute/v1{}", path);
        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(token)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_post(&self, path: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let token = self.get_token().await?;
        let url = format!("https://compute.googleapis.com/compute/v1{}", path);
        let resp = reqwest::Client::new()
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_delete(&self, path: &str) -> anyhow::Result<()> {
        let token = self.get_token().await?;
        let url = format!("https://compute.googleapis.com/compute/v1{}", path);
        reqwest::Client::new()
            .delete(&url)
            .bearer_auth(token)
            .send().await?;
        Ok(())
    }

    fn instance_name(&self, project: &str) -> String {
        format!("crush-{}", project)
    }

    async fn find_instance(&self, project: &str) -> anyhow::Result<Option<(String, String)>> {
        let path = format!(
            "/projects/{}/zones/{}/instances/{}",
            self.project_id, self.zone, self.instance_name(project)
        );
        let data = self.api_get(&path).await?;
        if data.get("error").is_some() {
            return Ok(None);
        }
        let status = data["status"].as_str().unwrap_or("");
        let ip = data["networkInterfaces"][0]["accessConfigs"][0]["natIP"]
            .as_str().unwrap_or("").to_string();
        if !ip.is_empty() && status == "RUNNING" {
            let name = data["name"].as_str().unwrap_or("").to_string();
            return Ok(Some((name, ip)));
        }
        Ok(None)
    }

    async fn create_instance(&self, project: &str) -> anyhow::Result<(String, String)> {
        use serde_json::json;
        let name = self.instance_name(project);
        let zone_url = format!("projects/{}/zones/{}", self.project_id, self.zone);
        let startup = "#!/bin/bash\napt-get update -y && apt-get install -y curl\ncurl -fsSL https://github.com/Chidi09/crush/releases/latest/download/install.sh | bash\n";

        let body = json!({
            "name": name,
            "machineType": format!("zones/{}/machineTypes/{}", self.zone, self.machine_type),
            "disks": [{
                "boot": true,
                "autoDelete": true,
                "initializeParams": {
                    "sourceImage": "projects/ubuntu-os-cloud/global/images/family/ubuntu-2204-lts"
                }
            }],
            "networkInterfaces": [{
                "accessConfigs": [{ "type": "ONE_TO_ONE_NAT", "name": "External NAT" }]
            }],
            "metadata": {
                "items": [{ "key": "startup-script", "value": startup }]
            },
            "labels": { "crush-project": project }
        });

        let path = format!("/projects/{}/zones/{}/instances", self.project_id, self.zone);
        let resp = self.api_post(&path, body).await?;
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("GCP API error: {}", err));
        }

        // Poll until RUNNING
        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            let inst_path = format!(
                "/projects/{}/zones/{}/instances/{}",
                self.project_id, self.zone, name
            );
            let data = self.api_get(&inst_path).await?;
            if data["status"].as_str() == Some("RUNNING") {
                let ip = data["networkInterfaces"][0]["accessConfigs"][0]["natIP"]
                    .as_str().unwrap_or("").to_string();
                if !ip.is_empty() {
                    return Ok((name, ip));
                }
            }
        }
        Err(anyhow!("Timed out waiting for GCP instance to reach RUNNING state"))
    }
}

#[async_trait]
impl DeployProvider for GcpProvider {
    async fn provision(&self, project: &str, _region: &str, _size: &str) -> anyhow::Result<DeploymentInfo> {
        let (server_id, public_ip) = if let Some((id, ip)) = self.find_instance(project).await? {
            println!("  Reusing existing GCP instance {} ({})", id, ip);
            (id, ip)
        } else {
            println!("  Creating GCP instance ({}) in {}...", self.machine_type, self.zone);
            let (id, ip) = self.create_instance(project).await?;
            println!("  Instance {} running at {}", id, ip);
            (id, ip)
        };

        Ok(DeploymentInfo {
            provider: "gcp".to_string(),
            project: project.to_string(),
            server_id,
            public_ip,
            region: self.zone.clone(),
            deployed_at: chrono::Utc::now().to_rfc3339(),
            image_digest: String::new(),
            port: 80,
            domain: None,
            status: DeployStatus::Provisioning,
        })
    }

    async fn deploy(&self, info: &DeploymentInfo, image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()> {
        let ssh = SshProvider::new(&info.public_ip, 22, "ubuntu", None);
        ssh.deploy(info, image_tar, port, env).await
    }

    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        let path = format!(
            "/projects/{}/zones/{}/instances/{}",
            self.project_id, self.zone, info.server_id
        );
        self.api_delete(&path).await?;
        println!("  GCP instance {} deleted.", info.server_id);
        Ok(())
    }

    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let path = format!(
            "/projects/{}/zones/{}/instances/{}",
            self.project_id, self.zone, info.server_id
        );
        let data = self.api_get(&path).await?;
        Ok(match data["status"].as_str().unwrap_or("UNKNOWN") {
            "RUNNING" => DeployStatus::Running,
            "TERMINATED" | "STOPPED" => DeployStatus::Stopped,
            _ => DeployStatus::Provisioning,
        })
    }

    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let ssh = SshProvider::new(&info.public_ip, 22, "ubuntu", None);
        ssh.logs(info, lines).await
    }
}
