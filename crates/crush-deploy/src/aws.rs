use anyhow::{anyhow, Context};
use async_trait::async_trait;
use base64::Engine;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};
use super::ssh::SshProvider;
use crush_build::parser::DeployAws;

pub struct AwsProvider {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    instance_type: String,
    key_pair: Option<String>,
    security_group: Option<String>,
}

impl AwsProvider {
    pub fn new(config: &DeployAws) -> Self {
        Self {
            access_key_id: config.access_key_id.clone(),
            secret_access_key: config.secret_access_key.clone(),
            region: config.region.clone(),
            instance_type: config.instance_type.clone().unwrap_or_else(|| "t3.micro".to_string()),
            key_pair: config.key_pair.clone(),
            security_group: config.security_group.clone(),
        }
    }

    fn ec2_client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    // Minimal AWS SigV4 signing — uses aws_sdk_ec2 would be ideal but we keep
    // the dependency light by calling the EC2 query API directly via reqwest.
    // For now, set env vars and shell out to aws-cli if available, otherwise
    // error with a clear message asking the user to install aws-cli.
    async fn aws_cli(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = tokio::process::Command::new("aws");
        cmd.env("AWS_ACCESS_KEY_ID", &self.access_key_id)
            .env("AWS_SECRET_ACCESS_KEY", &self.secret_access_key)
            .env("AWS_DEFAULT_REGION", &self.region)
            .args(args)
            .arg("--output").arg("json");
        let out = cmd.output().await
            .context("aws CLI not found — install it from https://aws.amazon.com/cli/")?;
        if !out.status.success() {
            return Err(anyhow!("aws CLI error: {}", String::from_utf8_lossy(&out.stderr)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    async fn find_instance(&self, project: &str) -> anyhow::Result<Option<(String, String)>> {
        let out = self.aws_cli(&[
            "ec2", "describe-instances",
            "--filters",
            &format!("Name=tag:crush-project,Values={}", project),
            "Name=instance-state-name,Values=running,stopped",
        ]).await?;
        let val: serde_json::Value = serde_json::from_str(&out)?;
        let instance = val["Reservations"]
            .as_array()
            .and_then(|r| r.first())
            .and_then(|r| r["Instances"].as_array())
            .and_then(|i| i.first());
        if let Some(inst) = instance {
            let id = inst["InstanceId"].as_str().unwrap_or("").to_string();
            let ip = inst["PublicIpAddress"].as_str().unwrap_or("").to_string();
            if !id.is_empty() {
                return Ok(Some((id, ip)));
            }
        }
        Ok(None)
    }

    async fn latest_ubuntu_ami(&self) -> anyhow::Result<String> {
        let out = self.aws_cli(&[
            "ec2", "describe-images",
            "--owners", "099720109477",
            "--filters",
            "Name=name,Values=ubuntu/images/*/ubuntu-jammy-22.04-amd64-server-*",
            "Name=state,Values=available",
            "--query", "sort_by(Images, &CreationDate)[-1].ImageId",
        ]).await?;
        let ami = out.trim().trim_matches('"').to_string();
        if ami.is_empty() || ami == "null" {
            Err(anyhow!("Could not find latest Ubuntu 22.04 AMI in {}", self.region))
        } else {
            Ok(ami)
        }
    }

    async fn run_instance(&self, project: &str, ami: &str) -> anyhow::Result<(String, String)> {
        let userdata = base64::engine::general_purpose::STANDARD.encode(
            "#!/bin/bash\napt-get update -y && apt-get install -y curl\ncurl -fsSL https://github.com/Chidi09/crush/releases/latest/download/install.sh | bash\n"
        );
        let tag_spec = format!("ResourceType=instance,Tags=[{{Key=crush-project,Value={}}}]", project);
        let key_arg = self.key_pair.as_ref().map(|k| format!("--key-name {}", k));
        let sg_arg = self.security_group.as_ref().map(|sg| format!("--security-group-ids {}", sg));

        let mut cli_args: Vec<&str> = vec![
            "ec2", "run-instances",
            "--image-id", ami,
            "--instance-type", &self.instance_type,
            "--user-data", &userdata,
            "--tag-specifications", &tag_spec,
            "--count", "1",
        ];
        if let Some(ref k) = key_arg { cli_args.push(k.as_str()); }
        if let Some(ref sg) = sg_arg { cli_args.push(sg.as_str()); }

        let out = self.aws_cli(&cli_args).await?;
        let val: serde_json::Value = serde_json::from_str(&out)?;
        let id = val["Instances"][0]["InstanceId"]
            .as_str().context("no InstanceId in response")?.to_string();

        // Wait for running state and public IP
        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            let desc = self.aws_cli(&["ec2", "describe-instances", "--instance-ids", &id]).await?;
            let dv: serde_json::Value = serde_json::from_str(&desc)?;
            let inst = &dv["Reservations"][0]["Instances"][0];
            let state = inst["State"]["Name"].as_str().unwrap_or("");
            if state == "running" {
                let ip = inst["PublicIpAddress"].as_str().unwrap_or("").to_string();
                if !ip.is_empty() {
                    return Ok((id, ip));
                }
            }
        }
        Err(anyhow!("Timed out waiting for EC2 instance to reach running state"))
    }
}

#[async_trait]
impl DeployProvider for AwsProvider {
    async fn provision(&self, project: &str, _region: &str, _size: &str) -> anyhow::Result<DeploymentInfo> {
        let (server_id, public_ip) = if let Some((id, ip)) = self.find_instance(project).await? {
            println!("  Reusing existing EC2 instance {} ({})", id, ip);
            (id, ip)
        } else {
            println!("  Looking up latest Ubuntu 22.04 AMI in {}...", self.region);
            let ami = self.latest_ubuntu_ami().await?;
            println!("  Creating EC2 instance ({})...", self.instance_type);
            let (id, ip) = self.run_instance(project, &ami).await?;
            println!("  Instance {} running at {}", id, ip);
            (id, ip)
        };

        Ok(DeploymentInfo {
            provider: "aws".to_string(),
            project: project.to_string(),
            server_id,
            public_ip,
            region: self.region.clone(),
            deployed_at: chrono::Utc::now().to_rfc3339(),
            image_digest: String::new(),
            port: 80,
            domain: None,
            status: DeployStatus::Provisioning,
        })
    }

    async fn deploy(&self, info: &DeploymentInfo, image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()> {
        let key = self.key_pair.as_deref().map(|k| format!("~/.ssh/{}.pem", k));
        let ssh = SshProvider::new(&info.public_ip, 22, "ubuntu", key.as_deref());
        ssh.deploy(info, image_tar, port, env).await
    }

    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        self.aws_cli(&["ec2", "terminate-instances", "--instance-ids", &info.server_id]).await?;
        println!("  EC2 instance {} terminated.", info.server_id);
        Ok(())
    }

    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let out = self.aws_cli(&["ec2", "describe-instances", "--instance-ids", &info.server_id]).await?;
        let val: serde_json::Value = serde_json::from_str(&out)?;
        let state = val["Reservations"][0]["Instances"][0]["State"]["Name"]
            .as_str().unwrap_or("unknown");
        Ok(match state {
            "running" => DeployStatus::Running,
            "stopped" | "terminated" => DeployStatus::Stopped,
            _ => DeployStatus::Provisioning,
        })
    }

    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let key = self.key_pair.as_deref().map(|k| format!("~/.ssh/{}.pem", k));
        let ssh = SshProvider::new(&info.public_ip, 22, "ubuntu", key.as_deref());
        ssh.logs(info, lines).await
    }
}
