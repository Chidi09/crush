use anyhow::{anyhow, Context};
use std::path::Path;
use ssh2::Session;
use std::net::TcpStream;
use async_trait::async_trait;
use super::provider::{DeployProvider, DeploymentInfo, DeployStatus};

pub struct SshProvider {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: Option<std::path::PathBuf>,
}

impl SshProvider {
    pub fn new(host: &str, port: u16, user: &str, key_path: Option<&str>) -> Self {
        Self {
            host: host.to_string(),
            port,
            user: user.to_string(),
            key_path: key_path.map(std::path::PathBuf::from),
        }
    }

    fn connect(&self) -> anyhow::Result<Session> {
        let tcp = TcpStream::connect(format!("{}:{}", self.host, self.port))
            .context("TCP connect failed")?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;

        if let Some(ref key) = self.key_path {
            sess.userauth_pubkey_file(&self.user, None, key, None)?;
        } else {
            sess.userauth_agent(&self.user)?;
        }

        if !sess.authenticated() {
            return Err(anyhow!("SSH authentication failed for {}@{}", self.user, self.host));
        }
        Ok(sess)
    }

    fn exec(&self, sess: &Session, cmd: &str) -> anyhow::Result<(String, String, i32)> {
        let mut channel = sess.channel_session()?;
        channel.exec(cmd)?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        use std::io::Read;
        channel.read_to_string(&mut stdout)?;
        channel.stderr().read_to_string(&mut stderr)?;
        channel.wait_close()?;
        let exit = channel.exit_status()?;
        Ok((stdout, stderr, exit))
    }

    fn upload_file(&self, sess: &Session, local: &Path, remote: &str) -> anyhow::Result<()> {
        let data = std::fs::read(local)?;
        let mut channel = sess.scp_send(
            std::path::Path::new(remote),
            0o644,
            data.len() as u64,
            None,
        )?;
        use std::io::Write;
        channel.write_all(&data)?;
        channel.send_eof()?;
        channel.wait_eof()?;
        channel.close()?;
        channel.wait_close()?;
        Ok(())
    }

    pub fn deploy_image(
        &self,
        project: &str,
        image_tar: &Path,
        port: u16,
        env: &[String],
    ) -> anyhow::Result<()> {
        let sess = self.connect()?;

        let (out, _, _) = self.exec(&sess, "which crush || echo MISSING")?;
        if out.trim() == "MISSING" {
            let install_cmd = concat!(
                "curl -fsSL https://github.com/Chidi09/crush/releases/latest/download/install.sh",
                " | bash"
            );
            let (_, err, code) = self.exec(&sess, install_cmd)?;
            if code != 0 {
                return Err(anyhow!("Failed to install crush on remote: {}", err));
            }
        }

        let remote_tar = format!("/tmp/{}.tar", project);
        self.upload_file(&sess, image_tar, &remote_tar)?;

        let env_flags = env.iter()
            .map(|e| format!("-e {}", e))
            .collect::<Vec<_>>()
            .join(" ");

        let deploy_cmd = format!(
            "crush load {tar} && crush stop {proj} 2>/dev/null; crush run \
             --name {proj} -p {port}:{port} {env} $(crush images --filter name={proj} -q | head -1)",
            tar = remote_tar,
            proj = project,
            port = port,
            env = env_flags,
        );

        let (_, err, code) = self.exec(&sess, &deploy_cmd)?;
        if code != 0 {
            return Err(anyhow!("Deploy command failed on remote: {}", err));
        }

        Ok(())
    }
}

#[async_trait]
impl DeployProvider for SshProvider {
    async fn provision(&self, project: &str, _region: &str, _size: &str) -> anyhow::Result<DeploymentInfo> {
        Ok(DeploymentInfo {
            provider: "ssh".to_string(),
            project: project.to_string(),
            server_id: self.host.clone(),
            public_ip: self.host.clone(),
            region: "custom".to_string(),
            deployed_at: chrono::Utc::now().to_rfc3339(),
            image_digest: String::new(),
            port: 80,
            domain: None,
            status: DeployStatus::Running,
        })
    }

    async fn deploy(&self, info: &DeploymentInfo, image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()> {
        let project = info.project.clone();
        let tar = image_tar.to_path_buf();
        let env = env.to_vec();
        let host = self.host.clone();
        let user = self.user.clone();
        let key = self.key_path.clone();
        tokio::task::spawn_blocking(move || {
            let p = SshProvider::new(&host, 22, &user, key.as_deref().and_then(|k| k.to_str()));
            p.deploy_image(&project, &tar, port, &env)
        }).await??;
        Ok(())
    }

    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        let project = info.project.clone();
        let host = self.host.clone();
        let user = self.user.clone();
        let key = self.key_path.clone();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let p = SshProvider::new(&host, 22, &user, key.as_deref().and_then(|k| k.to_str()));
            let sess = p.connect()?;
            p.exec(&sess, &format!("crush stop {} && crush rm {}", project, project))?;
            Ok(())
        }).await??;
        Ok(())
    }

    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus> {
        let project = info.project.clone();
        let host = self.host.clone();
        let user = self.user.clone();
        let key = self.key_path.clone();
        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<DeployStatus> {
            let p = SshProvider::new(&host, 22, &user, key.as_deref().and_then(|k| k.to_str()));
            let sess = p.connect()?;
            let (out, _, _) = p.exec(&sess, &format!("crush ps --filter name={}", project))?;
            if out.contains("running") {
                Ok(DeployStatus::Running)
            } else {
                Ok(DeployStatus::Stopped)
            }
        }).await??;
        Ok(result)
    }

    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String> {
        let project = info.project.clone();
        let host = self.host.clone();
        let user = self.user.clone();
        let key = self.key_path.clone();
        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
            let p = SshProvider::new(&host, 22, &user, key.as_deref().and_then(|k| k.to_str()));
            let sess = p.connect()?;
            let (out, _, _) = p.exec(&sess, &format!("crush logs {} --tail {}", project, lines))?;
            Ok(out)
        }).await??;
        Ok(result)
    }
}
