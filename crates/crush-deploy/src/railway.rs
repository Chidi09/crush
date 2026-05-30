use anyhow::{anyhow, Context};
use async_trait::async_trait;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};
use crush_build::parser::DeployRailway;

/// Railway is a PaaS (like Fly) — there are no VMs to SSH into. Deployment goes
/// through the Railway CLI (`railway up`), which builds from the project's
/// Dockerfile (use `crush eject` first) and ships it. Auth is via a Railway
/// **project token** passed as `RAILWAY_TOKEN`.
///
/// NOTE: this provider drives the official `@railway/cli`. It must be installed
/// (`npm i -g @railway/cli`). Commands run in the process working directory,
/// which `crush deploy` sets to the project root.
pub struct RailwayProvider {
    api_token: String,
    project: Option<String>,
    service: Option<String>,
    environment: Option<String>,
    region: Option<String>,
}

impl RailwayProvider {
    pub fn new(config: &DeployRailway) -> Self {
        Self {
            api_token: config.api_token.clone(),
            project: config.project.clone(),
            service: config.service.clone(),
            environment: config.environment.clone(),
            region: config.region.clone(),
        }
    }

    async fn cli_available() -> bool {
        tokio::process::Command::new("railway")
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn cli(&self) -> tokio::process::Command {
        let mut c = tokio::process::Command::new("railway");
        c.env("RAILWAY_TOKEN", &self.api_token);
        c
    }
}

#[async_trait]
impl DeployProvider for RailwayProvider {
    async fn provision(&self, project: &str, _region: &str, _size: &str) -> anyhow::Result<DeploymentInfo> {
        if !Self::cli_available().await {
            return Err(anyhow!(
                "Railway CLI not found. Install it with `npm i -g @railway/cli`, \
                 then re-run deploy. (Railway deploys via `railway up`.)"
            ));
        }

        let name = self.project.clone().unwrap_or_else(|| format!("crush-{}", project));
        let service = self.service.clone().unwrap_or_else(|| project.to_string());
        // Railway assigns the public domain after the first deploy; use the
        // conventional subdomain as a best-effort placeholder until status fills it.
        let domain = format!("{}.up.railway.app", service);

        Ok(DeploymentInfo {
            provider: "railway".to_string(),
            project: project.to_string(),
            server_id: name,
            public_ip: domain.clone(),
            region: self.region.clone().unwrap_or_else(|| "us-west1".to_string()),
            deployed_at: chrono::Utc::now().to_rfc3339(),
            image_digest: String::new(),
            port: 80,
            domain: Some(domain),
            status: DeployStatus::Provisioning,
        })
    }

    async fn deploy(&self, info: &DeploymentInfo, _image_tar: &std::path::Path, _port: u16, _env: &[String]) -> anyhow::Result<()> {
        // Verified against a real CI pipeline (OLPDF): a project-scoped
        // RAILWAY_TOKEN + `railway up --service <svc> --environment <env> --detach`.
        // Railway builds from the repo's Dockerfile (driven by railway.json's
        // `builder: DOCKERFILE`); env vars live in railway.json / the dashboard,
        // not set via the CLI here.
        let service = self.service.clone().unwrap_or_else(|| info.project.clone());
        let environment = self.environment.clone().unwrap_or_else(|| "production".to_string());

        let mut cmd = self.cli();
        cmd.args(["up", "--service", &service, "--environment", &environment, "--detach"]);
        let out = cmd.output().await.context("`railway up` failed to run")?;
        if !out.status.success() {
            return Err(anyhow!(
                "railway up failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }

    async fn destroy(&self, _info: &DeploymentInfo) -> anyhow::Result<()> {
        let out = self.cli().args(["down", "--yes"]).output().await
            .context("`railway down` failed to run")?;
        if !out.status.success() {
            return Err(anyhow!("railway down failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
        Ok(())
    }

    async fn status(&self, _info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let out = self.cli().args(["status"]).output().await
            .context("`railway status` failed to run")?;
        if !out.status.success() {
            return Ok(DeployStatus::Failed(
                String::from_utf8_lossy(&out.stderr).trim().to_string(),
            ));
        }
        // The CLI doesn't expose a clean machine-readable state here; a successful
        // status call means the project is linked and reachable.
        Ok(DeployStatus::Running)
    }

    async fn logs(&self, _info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let out = self.cli()
            .args(["logs", "--lines", &lines.to_string()])
            .output().await
            .context("`railway logs` failed to run")?;
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }
}
