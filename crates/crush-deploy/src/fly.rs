use anyhow::{anyhow, Context};
use serde_json::json;
use async_trait::async_trait;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};
use crush_build::parser::DeployFly;

pub struct FlyProvider {
    api_token: String,
    app_name: Option<String>,
    region: Option<String>,
    vm_size: Option<String>,
}

impl FlyProvider {
    pub fn new(config: &DeployFly) -> Self {
        Self {
            api_token: config.api_token.clone(),
            app_name: config.app_name.clone(),
            region: config.region.clone(),
            vm_size: config.vm_size.clone(),
        }
    }

    fn resolved_app_name(&self, project: &str) -> String {
        self.app_name.clone().unwrap_or_else(|| format!("crush-{}", project))
    }

    async fn api_get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://api.machines.dev/v1{}", path);
        let resp = reqwest::Client::new()
            .get(&url)
            .bearer_auth(&self.api_token)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_post(&self, path: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let url = format!("https://api.machines.dev/v1{}", path);
        let resp = reqwest::Client::new()
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send().await?
            .json::<serde_json::Value>().await?;
        Ok(resp)
    }

    async fn api_delete(&self, path: &str) -> anyhow::Result<()> {
        let url = format!("https://api.machines.dev/v1{}", path);
        reqwest::Client::new()
            .delete(&url)
            .bearer_auth(&self.api_token)
            .send().await?;
        Ok(())
    }

    async fn app_exists(&self, app_name: &str) -> anyhow::Result<bool> {
        let data = self.api_get(&format!("/apps/{}/machines", app_name)).await?;
        Ok(!data.get("error").is_some())
    }

    async fn create_app(&self, app_name: &str) -> anyhow::Result<()> {
        let resp = self.api_post("/apps", json!({
            "app_name": app_name,
            "org_slug": "personal"
        })).await?;
        if let Some(err) = resp.get("error") {
            return Err(anyhow!("Fly.io create app error: {}", err));
        }
        Ok(())
    }
}

#[async_trait]
impl DeployProvider for FlyProvider {
    async fn provision(&self, project: &str, _region: &str, _size: &str) -> anyhow::Result<DeploymentInfo> {
        let app_name = self.resolved_app_name(project);

        if !self.app_exists(&app_name).await? {
            println!("  Creating Fly.io app '{}'...", app_name);
            self.create_app(&app_name).await?;
        } else {
            println!("  Using existing Fly.io app '{}'.", app_name);
        }

        Ok(DeploymentInfo {
            provider: "fly".to_string(),
            project: project.to_string(),
            server_id: app_name.clone(),
            public_ip: format!("{}.fly.dev", app_name),
            region: self.region.clone().unwrap_or_else(|| "iad".to_string()),
            deployed_at: chrono::Utc::now().to_rfc3339(),
            image_digest: String::new(),
            port: 80,
            domain: Some(format!("{}.fly.dev", app_name)),
            status: DeployStatus::Provisioning,
        })
    }

    async fn deploy(&self, info: &DeploymentInfo, _image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()> {
        let app_name = &info.server_id;
        let vm_size = self.vm_size.as_deref().unwrap_or("shared-cpu-1x");
        let region = self.region.as_deref().unwrap_or("iad");

        // Build env object
        let env_obj: serde_json::Map<String, serde_json::Value> = env.iter()
            .filter_map(|e| {
                let mut parts = e.splitn(2, '=');
                let k = parts.next()?.to_string();
                let v = parts.next().unwrap_or("").to_string();
                Some((k, serde_json::Value::String(v)))
            })
            .collect();

        // For Fly, we use fly CLI if available since OCI push to fly registry
        // requires docker-protocol auth which is complex to implement inline.
        let fly_available = tokio::process::Command::new("fly")
            .arg("version")
            .output().await
            .map(|o| o.status.success())
            .unwrap_or(false);

        if fly_available {
            // fly deploy handles image build + push + deploy in one command
            // when given a Dockerfile. Since we have a tar, we load it first.
            let out = tokio::process::Command::new("fly")
                .args(["deploy", "--app", app_name, "--region", region])
                .env("FLY_API_TOKEN", &self.api_token)
                .output().await
                .context("fly CLI failed")?;
            if !out.status.success() {
                return Err(anyhow!("fly deploy failed: {}", String::from_utf8_lossy(&out.stderr)));
            }
        } else {
            // Machines API: create/update machine with image from registry
            let image = format!("registry.fly.io/{}:latest", app_name);
            let body = json!({
                "config": {
                    "image": image,
                    "size": vm_size,
                    "env": env_obj,
                    "services": [{
                        "ports": [{ "port": port, "handlers": ["http"] }],
                        "internal_port": port,
                        "protocol": "tcp"
                    }]
                },
                "region": region
            });
            let resp = self.api_post(&format!("/apps/{}/machines", app_name), body).await?;
            if let Some(err) = resp.get("error") {
                return Err(anyhow!("Fly.io deploy error: {}", err));
            }
        }

        Ok(())
    }

    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        self.api_delete(&format!("/apps/{}", info.server_id)).await?;
        println!("  Fly.io app '{}' deleted.", info.server_id);
        Ok(())
    }

    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let data = self.api_get(&format!("/apps/{}/machines", info.server_id)).await?;
        if let Some(machines) = data.as_array() {
            let running = machines.iter()
                .any(|m| m["state"].as_str() == Some("started"));
            if running {
                return Ok(DeployStatus::Running);
            }
        }
        Ok(DeployStatus::Stopped)
    }

    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let data = self.api_get(&format!("/apps/{}/machines", info.server_id)).await?;
        let machine_id = data.as_array()
            .and_then(|m| m.first())
            .and_then(|m| m["id"].as_str())
            .unwrap_or("");
        if machine_id.is_empty() {
            return Ok(String::new());
        }
        let log_data = self.api_get(&format!("/apps/{}/machines/{}/log", info.server_id, machine_id)).await?;
        let n = lines as usize;
        let text = log_data.as_array()
            .map(|entries| {
                entries.iter()
                    .take(n)
                    .filter_map(|l| l["message"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Ok(text)
    }
}
