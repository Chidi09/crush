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

/// Blue-green executor over SSH. Reuses [`SshProvider`]'s connect/exec/upload to
/// drive container + gateway operations on the remote host. Each call opens its
/// own session via `spawn_blocking` (ssh2 is sync), matching the rest of this
/// module.
pub struct SshBlueGreen {
    pub ssh: SshProvider,
    pub project: String,
    pub public_port: u16,
}

impl SshBlueGreen {
    pub fn new(ssh: SshProvider, project: &str, public_port: u16) -> Self {
        Self { ssh, project: project.to_string(), public_port }
    }

    fn clone_ssh(&self) -> SshProvider {
        SshProvider::new(
            &self.ssh.host,
            self.ssh.port,
            &self.ssh.user,
            self.ssh.key_path.as_deref().and_then(|k| k.to_str()),
        )
    }

    /// Run a remote command, returning (stdout, exit_code).
    async fn run(&self, cmd: String) -> anyhow::Result<(String, i32)> {
        let ssh = self.clone_ssh();
        tokio::task::spawn_blocking(move || -> anyhow::Result<(String, i32)> {
            let sess = ssh.connect()?;
            let (out, _err, code) = ssh.exec(&sess, &cmd)?;
            Ok((out, code))
        })
        .await?
    }
}

#[async_trait]
impl crate::bluegreen::BlueGreenOps for SshBlueGreen {
    async fn current_color(&self, _project: &str) -> anyhow::Result<Option<crate::bluegreen::Color>> {
        use crate::bluegreen::{internal_port, target_file_path, Color};
        // The live color is whatever the gateway currently points at.
        let file = target_file_path(&self.project);
        let (out, _) = self.run(format!("cat {file} 2>/dev/null || true")).await?;
        let port: Option<u16> = out.trim().rsplit(':').next().and_then(|p| p.trim().parse().ok());
        Ok(match port {
            Some(p) if p == internal_port(self.public_port, Color::Blue) => Some(Color::Blue),
            Some(p) if p == internal_port(self.public_port, Color::Green) => Some(Color::Green),
            _ => None,
        })
    }

    async fn load_image(&self, image_tar: &std::path::Path) -> anyhow::Result<String> {
        let project = self.project.clone();
        let remote_tar = format!("/tmp/{}-bg.tar", project);
        // Upload + load, then resolve the freshest image id for this project.
        let ssh = self.clone_ssh();
        let tar = image_tar.to_path_buf();
        let remote_tar2 = remote_tar.clone();
        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let sess = ssh.connect()?;
            ssh.upload_file(&sess, &tar, &remote_tar2)?;
            let (_o, err, code) = ssh.exec(&sess, &format!("crush load {remote_tar2}"))?;
            if code != 0 {
                return Err(anyhow!("crush load failed on remote: {err}"));
            }
            Ok(())
        })
        .await??;
        let (out, _) = self
            .run(format!("crush images --filter name={project} -q | head -1"))
            .await?;
        let image_ref = out.trim().to_string();
        if image_ref.is_empty() {
            return Err(anyhow!("could not resolve loaded image for {project}"));
        }
        Ok(image_ref)
    }

    async fn run_release(&self, container: &str, image_ref: &str, port: u16, env: &[String]) -> anyhow::Result<()> {
        let env_flags = env.iter().map(|e| format!("-e {e}")).collect::<Vec<_>>().join(" ");
        // Publish only to loopback — the gateway is the sole public entry point.
        let cmd = format!(
            "crush rm -f {c} 2>/dev/null; crush run -d --name {c} -p 127.0.0.1:{p}:{p} -e PORT={p} {env} {img}",
            c = container, p = port, env = env_flags, img = image_ref,
        );
        let (_out, code) = self.run(cmd).await?;
        if code != 0 {
            return Err(anyhow!("failed to start release container {container}"));
        }
        Ok(())
    }

    async fn health_check(&self, port: u16, health_path: &str) -> anyhow::Result<bool> {
        let path = if health_path.starts_with('/') { health_path.to_string() } else { format!("/{health_path}") };
        let cmd = format!(
            "curl -fsS -o /dev/null -w '%{{http_code}}' --max-time 3 http://127.0.0.1:{port}{path} 2>/dev/null || echo 000"
        );
        let (out, _) = self.run(cmd).await?;
        let code: u32 = out.trim().parse().unwrap_or(0);
        Ok((200..400).contains(&code))
    }

    async fn ensure_gateway(&self, public_port: u16, target_file: &str) -> anyhow::Result<()> {
        // Start the gateway daemon once; idempotent via a pgrep guard.
        let cmd = format!(
            "pgrep -f 'crush gateway --listen {p}' >/dev/null 2>&1 || \
             setsid crush gateway --listen {p} --target-file {f} >/var/log/crush-gateway.log 2>&1 < /dev/null &",
            p = public_port, f = target_file,
        );
        let (_out, _code) = self.run(cmd).await?;
        Ok(())
    }

    async fn switch_gateway(&self, target_file: &str, port: u16) -> anyhow::Result<()> {
        let (_out, code) = self
            .run(format!("crush gateway --target-file {target_file} --set {port}"))
            .await?;
        if code != 0 {
            return Err(anyhow!("failed to flip gateway to :{port}"));
        }
        Ok(())
    }

    async fn stop_release(&self, container: &str) -> anyhow::Result<()> {
        let _ = self.run(format!("crush stop {container} 2>/dev/null || true")).await?;
        Ok(())
    }

    async fn remove_release(&self, container: &str) -> anyhow::Result<()> {
        let _ = self.run(format!("crush rm -f {container} 2>/dev/null || true")).await?;
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
