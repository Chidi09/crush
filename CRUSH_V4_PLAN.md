# Crush v0.4.0 Implementation Plan

Three areas: **cloud deployment** (Crushfile → running production server),
**eBPF metrics** (per-container network + disk I/O via kernel programs), and
**detector overhaul** (signal scoring, full dep-tree parsing, no more wrong guesses).

**Rules:**
- Do NOT add `Co-Authored-By: Claude` or any AI trailer to git commits.
- Cross-compile target: `x86_64-pc-windows-gnu` on Linux VPS (safe-meet).
- New crate `crush-deploy` must be added to the workspace `members` list in root `Cargo.toml`.
- Bump version to `0.4.0` in root `Cargo.toml` as the final task.

---

## PART A — Cloud Deployment

### Overview

The deployment system works in three steps every time:
1. `crush build` (or reuse cached image) — already works.
2. **Provision** — ensure a server exists for this project (create one if not).
3. **Deploy** — push the image to the server and restart the container.

State is persisted to `~/.crush/deployments/<project-name>.json` so subsequent
`crush deploy` calls update in-place rather than creating new servers.

---

### Task A1 — Extend Crushfile format with `[deploy]` section

**File:** `crates/crush-build/src/parser.rs`

Add to the `Crushfile` struct:
```rust
pub deploy: Option<CrushfileDeploy>,
```

Add new structs:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushfileDeploy {
    /// "hetzner" | "ssh" | "aws" | "gcp" | "digitalocean" | "fly"
    pub provider: String,
    pub region: Option<String>,
    pub server_type: Option<String>,   // e.g. "cx21" for Hetzner, "t3.micro" for AWS
    pub image: Option<String>,         // base OS image for provisioning
    pub domain: Option<String>,        // optional custom domain
    pub env: Option<Vec<String>>,      // runtime env vars for the deployed container
    pub registry: Option<String>,      // image registry to push to before deploy

    // Provider-specific blocks
    pub hetzner: Option<DeployHetzner>,
    pub ssh: Option<DeploySsh>,
    pub aws: Option<DeployAws>,
    pub gcp: Option<DeployGcp>,
    pub digitalocean: Option<DeployDigitalOcean>,
    pub fly: Option<DeployFly>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployHetzner {
    pub api_token: String,   // supports ${ENV_VAR} interpolation
    pub server_name: Option<String>,
    pub ssh_key_name: Option<String>,
    pub datacenter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploySsh {
    pub host: String,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub key: Option<String>,        // path to private key
    pub password: Option<String>,   // fallback (not recommended)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployAws {
    pub access_key_id: String,         // supports ${ENV_VAR}
    pub secret_access_key: String,     // supports ${ENV_VAR}
    pub region: String,
    pub instance_type: Option<String>, // default: "t3.micro"
    pub ami: Option<String>,           // default: latest Ubuntu 22.04 in region
    pub key_pair: Option<String>,
    pub security_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployGcp {
    pub project_id: String,
    pub service_account_key: Option<String>,  // path to JSON key file
    pub zone: String,
    pub machine_type: Option<String>,          // default: "e2-micro"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployDigitalOcean {
    pub api_token: String,    // supports ${ENV_VAR}
    pub size: Option<String>, // default: "s-1vcpu-1gb"
    pub region: Option<String>,
    pub ssh_key_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployFly {
    pub api_token: String,     // supports ${ENV_VAR}
    pub app_name: Option<String>,
    pub region: Option<String>,
    pub vm_size: Option<String>,
}
```

Env var interpolation for credential fields is already implemented in
`CrushfileParser::interpolate_env` and will apply automatically.

---

### Task A2 — Create `crush-deploy` crate

**New directory:** `crates/crush-deploy/`

**File:** `crates/crush-deploy/Cargo.toml`
```toml
[package]
name = "crush-deploy"
version.workspace = true
edition.workspace = true

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
anyhow = { workspace = true }
crush-types = { workspace = true }
crush-build = { workspace = true }
dirs = "5.0"
chrono = { version = "0.4", features = ["serde"] }
base64 = "0.22"
```

**File:** `crates/crush-deploy/src/lib.rs`

```rust
pub mod provider;
pub mod hetzner;
pub mod ssh;
pub mod aws;
pub mod gcp;
pub mod digitalocean;
pub mod fly;
pub mod state;

pub use provider::{DeployProvider, DeploymentInfo, DeployStatus};
pub use state::DeploymentState;
```

**File:** `crates/crush-deploy/src/provider.rs`

Define the shared trait all providers implement:
```rust
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub provider: String,
    pub project: String,
    pub server_id: String,
    pub public_ip: String,
    pub region: String,
    pub deployed_at: String,   // RFC3339
    pub image_digest: String,
    pub port: u16,
    pub domain: Option<String>,
    pub status: DeployStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeployStatus {
    Provisioning,
    Running,
    Stopped,
    Failed(String),
}

#[async_trait]
pub trait DeployProvider: Send + Sync {
    /// Ensure a server exists for this project. Creates one if not found.
    /// Returns the server's public IP address.
    async fn provision(&self, project: &str, region: &str, size: &str) -> anyhow::Result<DeploymentInfo>;

    /// Upload the image tarball to the server and run it.
    /// `image_tar` is a path to an OCI tarball previously produced by `crush export`.
    async fn deploy(&self, info: &DeploymentInfo, image_tar: &std::path::Path, port: u16, env: &[String]) -> anyhow::Result<()>;

    /// Stop and remove the running container on the server.
    async fn destroy(&self, info: &DeploymentInfo) -> anyhow::Result<()>;

    /// Return the current status of the deployment.
    async fn status(&self, info: &DeploymentInfo) -> anyhow::Result<DeployStatus>;

    /// Stream the last N lines of container logs from the server.
    async fn logs(&self, info: &DeploymentInfo, lines: u32) -> anyhow::Result<String>;
}
```

---

### Task A3 — SSH provider (foundation for all others)

**File:** `crates/crush-deploy/src/ssh.rs`

The SSH provider is also the deployment engine all other providers use once
they've provisioned a server. It:
1. Connects via SSH using the `ssh2` crate.
2. Uploads the OCI tarball via SFTP.
3. Ensures `crush` is installed on the remote (downloads the Linux binary if not).
4. Runs `crush load <tarball> && crush run <image> -p <port> --name <project>`.

Add `ssh2 = "0.9"` to `crush-deploy/Cargo.toml`.

```rust
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
            key_path: key_path.map(|k| std::path::PathBuf::from(k)),
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

        // Ensure crush is installed on the remote
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

        // Upload the tarball
        let remote_tar = format!("/tmp/{}.tar", project);
        self.upload_file(&sess, image_tar, &remote_tar)?;

        // Load image, stop old container, run new one
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
            status: super::provider::DeployStatus::Running,
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
```

---

### Task A4 — Hetzner provider

**File:** `crates/crush-deploy/src/hetzner.rs`

Hetzner Cloud REST API base: `https://api.hetzner.cloud/v1`

```rust
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

    /// Find an existing server by label `crush-project=<name>` or return None.
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
        // Check if server already exists
        let (server_id, public_ip) = if let Some((id, ip)) = self.find_server(project).await? {
            println!("  Reusing existing Hetzner server {} ({})", id, ip);
            (id, ip)
        } else {
            println!("  Creating Hetzner server for '{}'...", project);
            let server_type = if size.is_empty() { "cx21" } else { size };
            let datacenter = if region.is_empty() { "nbg1-dc3" } else { region };
            let (id, ip) = self.create_server(project, server_type, datacenter, "ubuntu-22.04").await?;
            println!("  Server {} created at {}", id, ip);
            // Wait for the server to boot (poll status)
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
        // Delegate actual file upload + run to SSH provider
        let ssh = SshProvider::new(&info.public_ip, 22, "root", None);
        ssh.deploy(&info, image_tar, port, env).await
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
```

---

### Task A5 — DigitalOcean provider

**File:** `crates/crush-deploy/src/digitalocean.rs`

DigitalOcean API base: `https://api.digitalocean.com/v2`

Implement `DeployProvider` for `DigitalOceanProvider` following the same pattern
as Task A4 (Hetzner) but against the DO API:
- `GET /v2/droplets?tag_name=crush-<project>` to find existing droplets
- `POST /v2/droplets` to create — body fields: `name`, `region`, `size`, `image` (`"ubuntu-22-04-x64"`), `tags`, `user_data` (same install script as Hetzner), `ssh_keys`
- `DELETE /v2/droplets/<id>` to destroy
- `GET /v2/droplets/<id>` to get status — `status` field: `"active"` / `"off"`
- Auth header: `Authorization: Bearer <token>`

Delegate `deploy` and `logs` to `SshProvider` the same way Hetzner does.
Default size: `"s-1vcpu-1gb"`, default region: `"nyc3"`.

---

### Task A6 — AWS EC2 provider

**File:** `crates/crush-deploy/src/aws.rs`

Add `aws-sdk-ec2 = "1"` and `aws-config = "1"` to `crush-deploy/Cargo.toml`.

Implement `AwsProvider` using the AWS SDK:
1. `provision`: call `describe_instances` filtering by tag `crush-project=<name>`.
   If found and running, reuse it. Otherwise call `run_instances` with:
   - `image_id`: latest Ubuntu 22.04 AMI for the region (query via `describe_images`
     with owner `"099720109477"` and name filter `"ubuntu/images/*/ubuntu-jammy-22.04-amd64-server-*"`, sorted by creation date descending, take first)
   - `instance_type`: from config (default `"t3.micro"`)
   - `tag_specifications`: `[{ResourceType: "instance", Tags: [{Key: "crush-project", Value: project}]}]`
   - `user_data`: base64-encoded install script (same content as Hetzner)
   - `key_name`: from config if set
2. Wait for the instance to reach `running` state using `describe_instances` polling.
3. Retrieve the public IP from `instances[0].public_ip_address`.
4. `deploy` / `logs`: delegate to `SshProvider` using the public IP.
5. `destroy`: call `terminate_instances` with the instance ID.

Security group note: if no security group is configured, create one named
`crush-<project>-sg` with inbound rules for port 22 (SSH) and the container
port. Do this only once — tag it like the instance so it's reused.

---

### Task A7 — GCP Compute Engine provider

**File:** `crates/crush-deploy/src/gcp.rs`

GCP REST API base: `https://compute.googleapis.com/compute/v1`

For auth, read `GOOGLE_APPLICATION_CREDENTIALS` env var for service account JSON.
Use the JSON key to get a Bearer token via `https://oauth2.googleapis.com/token`
with `grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer` and a signed JWT.

Add `jsonwebtoken = "9"` to `crush-deploy/Cargo.toml` for JWT signing.

Implement `GcpProvider`:
1. `provision`: `GET /projects/<project_id>/zones/<zone>/instances?filter=labels.crush-project=<name>`.
   If not found: `POST /projects/<project_id>/zones/<zone>/instances` with:
   - `name`: `crush-<project>`
   - `machineType`: `zones/<zone>/machineTypes/<machine_type>`
   - `disks`: boot disk from `projects/ubuntu-os-cloud/global/images/family/ubuntu-2204-lts`
   - `networkInterfaces`: one interface with accessConfig for external IP
   - `metadata.items`: startup script (install crush)
   - `labels`: `{"crush-project": "<name>"}`
2. Poll instance status until `status == "RUNNING"`.
3. Get public IP from `networkInterfaces[0].accessConfigs[0].natIP`.
4. `deploy`/`logs`: delegate to `SshProvider`.
5. `destroy`: `DELETE /projects/<project_id>/zones/<zone>/instances/crush-<project>`.

---

### Task A8 — Fly.io provider

**File:** `crates/crush-deploy/src/fly.rs`

Fly.io has a GraphQL API and a Machines API (`https://api.machines.dev/v1`).
Use the Machines API — it's REST and simpler.

Implement `FlyProvider`:
1. `provision`: `GET /v1/apps/<app-name>/machines` to check if the app exists.
   If not, first `POST /v1/apps` to create it: `{"app_name": "crush-<project>", "org_slug": "personal"}`.
2. `deploy`:
   - Push the image to the Fly registry: `fly.io/<app-name>:<digest>`.
     Fly's registry accepts image pushes via token auth — use `docker push`
     equivalent via the OCI registry HTTP API with `Authorization: Bearer <token>`.
   - `POST /v1/apps/<app-name>/machines` to create or update the machine:
     `{"config": {"image": "registry.fly.io/<app-name>:latest", "services": [{"ports": [{"port": <port>}], "internal_port": <port>}]}}`.
3. `status`: `GET /v1/apps/<app-name>/machines` and check `state` field.
4. `destroy`: `DELETE /v1/apps/<app-name>`.
5. `logs`: `GET /v1/apps/<app-name>/machines/<machine-id>/log`.

Auth: `Authorization: Bearer <api_token>` on all requests.

---

### Task A9 — Deployment state store

**File:** `crates/crush-deploy/src/state.rs`

```rust
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use super::provider::DeploymentInfo;

pub struct DeploymentState {
    dir: PathBuf,
}

impl DeploymentState {
    pub fn new() -> Self {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".crush")
            .join("deployments");
        std::fs::create_dir_all(&dir).ok();
        Self { dir }
    }

    fn path(&self, project: &str) -> PathBuf {
        self.dir.join(format!("{}.json", project))
    }

    pub fn load(&self, project: &str) -> Option<DeploymentInfo> {
        let content = std::fs::read_to_string(self.path(project)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, info: &DeploymentInfo) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(info)?;
        std::fs::write(self.path(&info.project), json)?;
        Ok(())
    }

    pub fn remove(&self, project: &str) {
        let _ = std::fs::remove_file(self.path(project));
    }

    pub fn list(&self) -> Vec<DeploymentInfo> {
        let mut result = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(info) = serde_json::from_str::<DeploymentInfo>(&content) {
                        result.push(info);
                    }
                }
            }
        }
        result
    }
}
```

---

### Task A10 — Wire `crush deploy` into the CLI

**File:** `crates/crush-cli/Cargo.toml`

Add:
```toml
crush-deploy = { workspace = true }
```

**File:** `crates/crush-cli/src/main.rs`

Add to `Commands` enum:
```rust
/// Deploy a project to cloud infrastructure defined in the Crushfile
Deploy(DeployArgs),
```

Add args struct:
```rust
#[derive(Args, Debug)]
pub struct DeployArgs {
    /// Override the provider from Crushfile (hetzner, ssh, aws, gcp, digitalocean, fly)
    #[arg(long)]
    provider: Option<String>,
    /// Stream deployment logs after deploy completes
    #[arg(long)]
    logs: bool,
    /// Show current deployment status
    #[arg(long)]
    status: bool,
    /// Destroy the deployment and remove the server
    #[arg(long)]
    destroy: bool,
}
```

Add the `Commands::Deploy` handler:
```rust
Commands::Deploy(args) => {
    use crush_deploy::{DeploymentState, DeployProvider};

    let root = std::env::current_dir()?;
    let crushfile_path = root.join("Crushfile");
    let crushfile = crush_build::parser::CrushfileParser::parse(&crushfile_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse Crushfile: {}", e))?;

    let deploy_config = crushfile.deploy.as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "No [deploy] section in Crushfile.\n\
             Add one like:\n\
             \n  [deploy]\n  provider = \"hetzner\"\n\
             \n  [deploy.hetzner]\n  api_token = \"${{HETZNER_API_TOKEN}}\"\n"
        ))?;

    let project = crushfile.project.as_ref()
        .and_then(|p| p.name.clone())
        .unwrap_or_else(|| root.file_name().unwrap_or_default().to_string_lossy().to_string());

    let provider_name = args.provider.as_deref()
        .unwrap_or(&deploy_config.provider);

    let state = DeploymentState::new();

    if args.status {
        if let Some(info) = state.load(&project) {
            let provider = build_provider(provider_name, deploy_config)?;
            let status = provider.status(&info).await?;
            println!("Project:  {}", info.project);
            println!("Provider: {}", info.provider);
            println!("Server:   {} ({})", info.server_id, info.public_ip);
            println!("Deployed: {}", info.deployed_at);
            println!("Status:   {:?}", status);
            if let Some(domain) = &info.domain {
                println!("Domain:   {}", domain);
            } else {
                println!("URL:      http://{}:{}", info.public_ip, info.port);
            }
        } else {
            println!("No deployment found for '{}'", project);
        }
        return Ok(());
    }

    if args.destroy {
        if let Some(info) = state.load(&project) {
            let provider = build_provider(provider_name, deploy_config)?;
            println!("Destroying deployment for '{}'...", project);
            provider.destroy(&info).await?;
            state.remove(&project);
            println!("Deployment destroyed.");
        } else {
            println!("No deployment found for '{}'", project);
        }
        return Ok(());
    }

    // Build the project image
    println!("[1/4] Building image...");
    // Re-use the existing build logic from Commands::Default
    let detection = crush_build::StackDetector::new().detect(&root);
    let store = crush_image::ImageStore::new(&data_dir)?;
    let image = store.build_from_detection(&detection).await
        .map_err(|e| anyhow::anyhow!("Build failed: {}", e))?;

    // Export to a tarball
    println!("[2/4] Exporting OCI tarball...");
    let tar_path = std::env::temp_dir().join(format!("{}-deploy.tar", project));
    store.export_oci_tarball(&image.digest, &tar_path).await
        .map_err(|e| anyhow::anyhow!("Export failed: {}", e))?;

    // Provision infra
    println!("[3/4] Provisioning {}...", provider_name);
    let provider = build_provider(provider_name, deploy_config)?;
    let region = deploy_config.region.as_deref().unwrap_or("");
    let size = deploy_config.server_type.as_deref().unwrap_or("");
    let mut info = provider.provision(&project, region, size).await?;
    info.image_digest = image.digest.clone();
    info.port = detection.port;

    // Deploy
    println!("[4/4] Deploying to {}...", info.public_ip);
    let env = deploy_config.env.as_deref().unwrap_or(&[]).to_vec();
    provider.deploy(&info, &tar_path, detection.port, &env).await?;
    info.status = crush_deploy::DeployStatus::Running;

    state.save(&info)?;

    println!("\nDeployed successfully!");
    println!("  URL: http://{}:{}", info.public_ip, info.port);
    if let Some(domain) = &deploy_config.domain {
        println!("  Domain: {} (point DNS A record to {})", domain, info.public_ip);
        info.domain = Some(domain.clone());
        state.save(&info)?;
    }

    // Clean up tarball
    let _ = std::fs::remove_file(&tar_path);

    if args.logs {
        println!("\nContainer logs:");
        let logs = provider.logs(&info, 50).await?;
        print!("{}", logs);
    }
}
```

Add `build_provider` helper function:
```rust
fn build_provider(
    name: &str,
    config: &crush_build::parser::CrushfileDeploy,
) -> anyhow::Result<Box<dyn crush_deploy::DeployProvider>> {
    use crush_deploy::*;
    match name {
        "hetzner" => {
            let h = config.hetzner.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.hetzner] section"))?;
            Ok(Box::new(hetzner::HetznerProvider::new(
                &h.api_token,
                h.ssh_key_name.as_deref(),
            )))
        }
        "ssh" => {
            let s = config.ssh.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.ssh] section"))?;
            Ok(Box::new(ssh::SshProvider::new(
                &s.host,
                s.port.unwrap_or(22),
                s.user.as_deref().unwrap_or("root"),
                s.key.as_deref(),
            )))
        }
        "aws"           => { let a = config.aws.as_ref().ok_or_else(|| anyhow::anyhow!("Missing [deploy.aws]"))?; Ok(Box::new(aws::AwsProvider::new(a))) }
        "gcp"           => { let g = config.gcp.as_ref().ok_or_else(|| anyhow::anyhow!("Missing [deploy.gcp]"))?; Ok(Box::new(gcp::GcpProvider::new(g))) }
        "digitalocean"  => { let d = config.digitalocean.as_ref().ok_or_else(|| anyhow::anyhow!("Missing [deploy.digitalocean]"))?; Ok(Box::new(digitalocean::DigitalOceanProvider::new(d))) }
        "fly"           => { let f = config.fly.as_ref().ok_or_else(|| anyhow::anyhow!("Missing [deploy.fly]"))?; Ok(Box::new(fly::FlyProvider::new(f))) }
        other           => Err(anyhow::anyhow!("Unknown provider '{}'. Options: hetzner, ssh, aws, gcp, digitalocean, fly", other)),
    }
}
```

Also add `crush-deploy` to `Cargo.toml` workspace `members` and
`workspace.dependencies`.

---

## PART B — eBPF Metrics

### Overview

The existing eBPF programs (`xdp_router`, `tc_egress`) are for **routing** — they
live in `crush-ebpf-progs/src/main.rs`. For **metrics** we need two new programs:
- `cgroup_skb` — attached per container cgroup, counts bytes in/out
- `tracepoint/block/block_rq_complete` — counts block I/O per cgroup

These feed into BPF hash maps the userspace `crush stats` command reads.

---

### Task B1 — Add metrics eBPF programs to crush-ebpf-progs

**File:** `crates/crush-ebpf-progs/src/main.rs`

Add the following **after** the existing panic handler, still inside the `no_std` file:

```rust
// ---------------------------------------------------------------------------
// Metrics: per-cgroup network bytes
// ---------------------------------------------------------------------------
//
// Attached as cgroup_skb on INGRESS and EGRESS for each container cgroup.
// Key: cgroup_id (u64).  Value: 16-byte packed struct {rx_bytes: u64, tx_bytes: u64}.

#[map]
static NET_BYTES_MAP: aya_bpf::maps::HashMap<u64, u64> =
    aya_bpf::maps::HashMap::with_max_entries(4096, 0);

#[map]
static NET_TX_MAP: aya_bpf::maps::HashMap<u64, u64> =
    aya_bpf::maps::HashMap::with_max_entries(4096, 0);

#[aya_bpf::macros::cgroup_skb]
pub fn crush_net_ingress(ctx: aya_bpf::programs::SkBuffContext) -> i32 {
    let cgroup_id = unsafe { aya_bpf::helpers::bpf_get_current_cgroup_id() };
    let len = ctx.len() as u64;
    if let Some(val) = unsafe { NET_BYTES_MAP.get_ptr_mut(&cgroup_id) } {
        unsafe { *val += len; }
    } else {
        let _ = unsafe { NET_BYTES_MAP.insert(&cgroup_id, &len, 0) };
    }
    1 // BPF_OK
}

#[aya_bpf::macros::cgroup_skb]
pub fn crush_net_egress(ctx: aya_bpf::programs::SkBuffContext) -> i32 {
    let cgroup_id = unsafe { aya_bpf::helpers::bpf_get_current_cgroup_id() };
    let len = ctx.len() as u64;
    if let Some(val) = unsafe { NET_TX_MAP.get_ptr_mut(&cgroup_id) } {
        unsafe { *val += len; }
    } else {
        let _ = unsafe { NET_TX_MAP.insert(&cgroup_id, &len, 0) };
    }
    1
}

// ---------------------------------------------------------------------------
// Metrics: per-cgroup block I/O
// ---------------------------------------------------------------------------

#[map]
static BLOCK_READ_MAP: aya_bpf::maps::HashMap<u64, u64> =
    aya_bpf::maps::HashMap::with_max_entries(4096, 0);

#[map]
static BLOCK_WRITE_MAP: aya_bpf::maps::HashMap<u64, u64> =
    aya_bpf::maps::HashMap::with_max_entries(4096, 0);

// Tracepoint: block_rq_complete
// Fired when a block I/O request completes.
// We attribute it to the current cgroup.
#[aya_bpf::macros::tracepoint]
pub fn crush_block_rq_complete(ctx: aya_bpf::programs::TracePointContext) -> u32 {
    let cgroup_id = unsafe { aya_bpf::helpers::bpf_get_current_cgroup_id() };
    // Read the nr_sector field from the tracepoint args at offset 24 (u32).
    // struct trace_event_raw_block_rq_complete: bytes 0-7 common, 8 dev, 16 sector, 20 nr_sector, 24 errors, 28 rwbs
    // rwbs field is a string like "R", "W", "D", "F"
    // For simplicity read nr_sector at offset 20 and rwbs at offset 24
    let nr_sector: u32 = unsafe {
        ctx.read_at(20).unwrap_or(0)
    };
    let bytes = (nr_sector as u64) * 512;

    // rwbs is a 8-byte char array; first byte is 'R' (read) or 'W' (write)
    let rwbs_byte: u8 = unsafe { ctx.read_at(24).unwrap_or(b'?') };

    match rwbs_byte {
        b'R' => {
            if let Some(val) = unsafe { BLOCK_READ_MAP.get_ptr_mut(&cgroup_id) } {
                unsafe { *val += bytes; }
            } else {
                let _ = unsafe { BLOCK_READ_MAP.insert(&cgroup_id, &bytes, 0) };
            }
        }
        b'W' | b'F' => {
            if let Some(val) = unsafe { BLOCK_WRITE_MAP.get_ptr_mut(&cgroup_id) } {
                unsafe { *val += bytes; }
            } else {
                let _ = unsafe { BLOCK_WRITE_MAP.insert(&cgroup_id, &bytes, 0) };
            }
        }
        _ => {}
    }
    0
}
```

---

### Task B2 — Add metrics reader to crush-network EbpfManager

**File:** `crates/crush-network/src/ebpf.rs`

Add a `ContainerMetrics` struct and a `read_container_metrics` method to
`EbpfManager` (inside the `#[cfg(feature = "ebpf")]` block):

```rust
#[derive(Debug, Default, Clone)]
pub struct ContainerMetrics {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
}

// (inside the feature = "ebpf" impl block)
pub fn read_container_metrics(&self, cgroup_id: u64) -> ContainerMetrics {
    use aya::maps::HashMap;

    let mut m = ContainerMetrics::default();

    if let Ok(map) = HashMap::<_, u64, u64>::try_from(self.bpf.map("NET_BYTES_MAP").unwrap()) {
        m.rx_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
    }
    if let Ok(map) = HashMap::<_, u64, u64>::try_from(self.bpf.map("NET_TX_MAP").unwrap()) {
        m.tx_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
    }
    if let Ok(map) = HashMap::<_, u64, u64>::try_from(self.bpf.map("BLOCK_READ_MAP").unwrap()) {
        m.block_read_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
    }
    if let Ok(map) = HashMap::<_, u64, u64>::try_from(self.bpf.map("BLOCK_WRITE_MAP").unwrap()) {
        m.block_write_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
    }

    m
}
```

Also add a method to attach the new cgroup programs:
```rust
pub fn attach_metrics(&mut self, container_id: &str) -> Result<()> {
    use aya::programs::{CgroupSkb, CgroupSkbAttachType};

    let cgroup_path = format!("/sys/fs/cgroup/crush/{}", container_id);

    let ingress: &mut CgroupSkb = self.bpf
        .program_mut("crush_net_ingress")
        .ok_or_else(|| CrushError::NetworkError("crush_net_ingress not found".into()))?
        .try_into()
        .map_err(|e| CrushError::NetworkError(format!("ingress cast: {}", e)))?;
    ingress.load()
        .map_err(|e| CrushError::NetworkError(format!("ingress load: {}", e)))?;
    ingress.attach(std::path::Path::new(&cgroup_path), CgroupSkbAttachType::Ingress)
        .map_err(|e| CrushError::NetworkError(format!("ingress attach {}: {}", cgroup_path, e)))?;

    let egress: &mut CgroupSkb = self.bpf
        .program_mut("crush_net_egress")
        .ok_or_else(|| CrushError::NetworkError("crush_net_egress not found".into()))?
        .try_into()
        .map_err(|e| CrushError::NetworkError(format!("egress cast: {}", e)))?;
    egress.load()
        .map_err(|e| CrushError::NetworkError(format!("egress load: {}", e)))?;
    egress.attach(std::path::Path::new(&cgroup_path), CgroupSkbAttachType::Egress)
        .map_err(|e| CrushError::NetworkError(format!("egress attach: {}", e)))?;

    Ok(())
}
```

---

### Task B3 — Enhanced `crush stats` TUI with eBPF data + new columns

**File:** `crates/crush-tui/src/tui.rs`

The current stats table has: `CONTAINER | CPU% | MEM | MEM% | PIDS`.
Extend it to: `CONTAINER | CPU% | MEM | NET IN | NET OUT | DISK R | DISK W | PIDS`.

1. Add `ContainerMetrics` to the `ContainerStats` struct (or a parallel struct):
```rust
pub struct ExtendedStats {
    pub cpu_pct: f32,
    pub mem_mb: f32,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub disk_read_per_sec: u64,
    pub disk_write_per_sec: u64,
    pub pids: u32,
}
```

2. When eBPF is available (`cfg(feature = "ebpf")` and `EbpfManager::check_availability() == Available`),
   read from the eBPF maps. Otherwise fall back to parsing `/proc/<pid>/net/dev` for network
   and `/proc/<pid>/io` for disk I/O (these are per-process approximations, not per-cgroup,
   but they work without eBPF).

   `/proc/<pid>/net/dev` — the `eth0` or `veth` line contains cumulative rx/tx bytes.
   `/proc/<pid>/io` — lines `read_bytes:` and `write_bytes:` give cumulative disk I/O.
   Compute rates by sampling twice 500ms apart, same as CPU delta.

3. In `draw_containers_table`, add columns:
```rust
let header = Row::new(vec![
    Cell::from("CONTAINER").style(header_style),
    Cell::from("CPU %").style(header_style),
    Cell::from("MEM").style(header_style),
    Cell::from("NET IN/s").style(header_style),
    Cell::from("NET OUT/s").style(header_style),
    Cell::from("DISK R/s").style(header_style),
    Cell::from("DISK W/s").style(header_style),
    Cell::from("PIDS").style(header_style),
]);
```

4. Add a helper `format_rate(bytes_per_sec: u64) -> String` that produces
   `"1.2 MB/s"`, `"340 KB/s"`, `"88 B/s"` — reuse `format_bytes` from main.rs
   but append `/s`.

5. Add sparklines for net in/out below the main table — use `ratatui::widgets::Sparkline`
   (already imported) with a 60-sample history. Store history in the existing
   `VecDeque` pattern already in the TUI state.

6. Color code the NET IN/OUT cells: green < 1MB/s, yellow 1–10MB/s, red > 10MB/s.

---

## PART C — Detector Overhaul

### Task C1 — Replace file-existence checks with signal scoring

**File:** `crates/crush-build/src/detect.rs`

Currently every `try_*` function returns `Option<Detection>` and checks a few
files. Replace this with a **scoring system** where multiple signals vote
for a framework.

Add a `Score` helper (local to this module):
```rust
struct Signals {
    scores: std::collections::HashMap<String, f32>,
}

impl Signals {
    fn new() -> Self { Self { scores: std::collections::HashMap::new() } }
    fn add(&mut self, framework: &str, weight: f32) {
        *self.scores.entry(framework.to_string()).or_insert(0.0) += weight;
    }
    fn winner(&self) -> Option<(&str, f32)> {
        self.scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.as_str(), *v))
    }
}
```

Then rewrite `detect_node_framework` to use it:
```rust
fn detect_node_framework(&self, json: &serde_json::Value, root: &Path) -> (String, f32) {
    let deps = Self::merge_deps(json);
    let dep_set: std::collections::HashSet<&str> = deps.iter().map(|s| s.as_str()).collect();
    let has_file = |name: &str| root.join(name).exists();
    let has_script = |key: &str| {
        json["scripts"][key].as_str().is_some()
    };
    let script_contains = |key: &str, needle: &str| {
        json["scripts"][key].as_str().map(|s| s.contains(needle)).unwrap_or(false)
    };

    let mut s = Signals::new();

    // High-confidence: specific config files
    if has_file("next.config.js") || has_file("next.config.ts") || has_file("next.config.mjs") { s.add("Next.js", 10.0); }
    if has_file("nuxt.config.ts") || has_file("nuxt.config.js") { s.add("Nuxt", 10.0); }
    if has_file("svelte.config.js") || has_file("svelte.config.ts") { s.add("SvelteKit", 10.0); }
    if has_file("astro.config.mjs") || has_file("astro.config.ts") { s.add("Astro", 10.0); }
    if has_file("nest-cli.json") || has_file(".nestrc") { s.add("NestJS", 10.0); }
    if has_file("remix.config.js") || has_file("remix.config.ts") { s.add("Remix", 10.0); }
    if has_file("qwik.config.ts") { s.add("Qwik", 10.0); }

    // High-confidence: explicit dependencies
    if dep_set.contains("next") { s.add("Next.js", 8.0); }
    if dep_set.contains("nuxt") { s.add("Nuxt", 8.0); }
    if dep_set.contains("@sveltejs/kit") { s.add("SvelteKit", 8.0); }
    if dep_set.contains("astro") { s.add("Astro", 8.0); }
    if dep_set.contains("@nestjs/core") { s.add("NestJS", 8.0); }
    if dep_set.contains("remix") || dep_set.contains("@remix-run/node") { s.add("Remix", 8.0); }
    if dep_set.contains("@builder.io/qwik") { s.add("Qwik", 8.0); }
    if dep_set.contains("@solidjs/start") { s.add("SolidStart", 8.0); }
    if dep_set.contains("fastify") { s.add("Fastify", 8.0); }
    if dep_set.contains("express") { s.add("Express", 6.0); }
    if dep_set.contains("hono") { s.add("Hono", 8.0); }
    if dep_set.contains("elysia") { s.add("Elysia", 8.0); }
    if dep_set.contains("@trpc/server") { s.add("tRPC", 4.0); }

    // Medium-confidence: start script patterns
    if script_contains("dev", "next dev") || script_contains("start", "next start") { s.add("Next.js", 5.0); }
    if script_contains("dev", "nuxt dev") { s.add("Nuxt", 5.0); }
    if script_contains("dev", "vite") || has_file("vite.config.ts") || has_file("vite.config.js") { s.add("Vite", 5.0); }
    if script_contains("dev", "fastify") { s.add("Fastify", 4.0); }
    if script_contains("start", "fastify") { s.add("Fastify", 4.0); }
    if script_contains("dev", "svelte-kit") || script_contains("dev", "vite") && dep_set.contains("svelte") {
        s.add("SvelteKit", 4.0);
    }

    match s.winner() {
        Some((framework, score)) if score >= 4.0 => (framework.to_string(), (score / 20.0).min(0.08)),
        _ => (String::new(), 0.0),
    }
}
```

---

### Task C2 — Full requirements.txt and pyproject.toml parsing for Python

**File:** `crates/crush-build/src/detect.rs`

Replace the fragile `try_python` framework detection. Instead of looking at filenames,
parse the actual dependency files:

```rust
fn parse_python_deps(root: &Path) -> Vec<String> {
    let mut deps = Vec::new();

    // requirements.txt — each line is "package[extras]>=version" or "package==version"
    if let Ok(content) = std::fs::read_to_string(root.join("requirements.txt")) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('-') { continue; }
            let name = line.split(['=', '>', '<', '!', '[', ';']).next().unwrap_or("").trim().to_lowercase();
            if !name.is_empty() { deps.push(name); }
        }
    }

    // pyproject.toml — [project.dependencies] or [tool.poetry.dependencies]
    if let Ok(content) = std::fs::read_to_string(root.join("pyproject.toml")) {
        if let Ok(val) = toml::from_str::<serde_json::Value>(&content) {
            // PEP 621: [project.dependencies]
            if let Some(arr) = val["project"]["dependencies"].as_array() {
                for dep in arr {
                    if let Some(s) = dep.as_str() {
                        let name = s.split(['=', '>', '<', '!', '[', ';', ' ']).next().unwrap_or("").trim().to_lowercase();
                        if !name.is_empty() { deps.push(name); }
                    }
                }
            }
            // Poetry: [tool.poetry.dependencies]
            if let Some(obj) = val["tool"]["poetry"]["dependencies"].as_object() {
                for key in obj.keys() {
                    if key.to_lowercase() != "python" { deps.push(key.to_lowercase()); }
                }
            }
        }
    }

    deps
}
```

Then in `try_python`, replace the framework detection with:
```rust
let py_deps = Self::parse_python_deps(root);
let has_dep = |name: &str| py_deps.iter().any(|d| d == name);

let mut s = Signals::new();
if has_dep("fastapi") { s.add("FastAPI", 10.0); }
if has_dep("flask") { s.add("Flask", 10.0); }
if has_dep("django") { s.add("Django", 10.0); }
if has_dep("tornado") { s.add("Tornado", 8.0); }
if has_dep("aiohttp") { s.add("aiohttp", 8.0); }
if has_dep("starlette") && !has_dep("fastapi") { s.add("Starlette", 7.0); }
if has_dep("litestar") { s.add("Litestar", 8.0); }
if root.join("manage.py").exists() { s.add("Django", 9.0); }

let (framework, entry, port) = match s.winner() {
    Some(("FastAPI", _)) => ("FastAPI", "main.py", 8000),
    Some(("Flask", _)) => ("Flask", "app.py", 5000),
    Some(("Django", _)) => ("Django", "manage.py", 8000),
    Some(("Tornado", _)) => ("Tornado", "main.py", 8888),
    Some(("aiohttp", _)) => ("aiohttp", "main.py", 8080),
    Some(("Starlette", _)) => ("Starlette", "main.py", 8000),
    Some(("Litestar", _)) => ("Litestar", "app.py", 8000),
    _ => ("Python", "main.py", 8080),
};
```

---

### Task C3 — Full Gemfile parsing for Ruby

**File:** `crates/crush-build/src/detect.rs`

In `try_ruby`, instead of just checking if `config/application.rb` exists, parse
`Gemfile`:

```rust
fn parse_gemfile(root: &Path) -> Vec<String> {
    let mut gems = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("Gemfile")) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("gem ") {
                // gem 'rails', '~> 7.0'  or  gem "sinatra"
                let name = line.trim_start_matches("gem ")
                    .trim_matches(&[' ', '\'', '"'][..])
                    .split(['\'', '"', ','])
                    .next().unwrap_or("").trim().to_lowercase();
                if !name.is_empty() { gems.push(name); }
            }
        }
    }
    gems
}
```

Use it in `try_ruby`:
```rust
let gems = Self::parse_gemfile(root);
let has_gem = |name: &str| gems.iter().any(|g| g == name);

let (framework, entry, port) = if has_gem("rails") {
    ("Rails", "config.ru", 3000)
} else if has_gem("sinatra") {
    ("Sinatra", "app.rb", 4567)
} else if has_gem("hanami") {
    ("Hanami", "config.ru", 2300)
} else if has_gem("grape") {
    ("Grape", "app.rb", 9292)
} else {
    ("", "app.rb", 8080)
};

let confidence = if !framework.is_empty() { 0.96 } else { 0.87 };
```

---

### Task C4 — Composer.json full parsing for PHP

**File:** `crates/crush-build/src/detect.rs`

In `try_php`, parse `composer.json` fully instead of checking a substring:

```rust
fn parse_composer_deps(root: &Path) -> Vec<String> {
    let mut deps = Vec::new();
    if let Ok(content) = std::fs::read_to_string(root.join("composer.json")) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            for section in &["require", "require-dev"] {
                if let Some(obj) = val[section].as_object() {
                    deps.extend(obj.keys().map(|k| k.to_lowercase()));
                }
            }
        }
    }
    deps
}
```

Use it to detect framework with scoring:
```rust
let deps = Self::parse_composer_deps(root);
let has_dep = |name: &str| deps.iter().any(|d| d.contains(name));

if has_dep("laravel/framework") { s.add("Laravel", 10.0); }
if has_dep("symfony/framework-bundle") { s.add("Symfony", 10.0); }
if has_dep("slim/slim") { s.add("Slim", 8.0); }
if has_dep("codeigniter4") { s.add("CodeIgniter", 8.0); }
if has_dep("cakephp/cakephp") { s.add("CakePHP", 8.0); }
if root.join("artisan").exists() { s.add("Laravel", 9.0); }
if root.join("bin/console").exists() { s.add("Symfony", 7.0); }
```

---

### Task C5 — Raise minimum confidence for direct dependency matches to 0.99

**File:** `crates/crush-build/src/detect.rs`

When a framework dependency is found **directly in the package manager manifest**
(not just a config file), the detection confidence should be near-certain.

Update the confidence calculation in `try_node`:
```rust
// Current:
let mut confidence = if has_ts { 0.97 } else { 0.93 };
confidence += confidence_bump;

// New: if a framework was detected from a hard dep (score >= 8), bump to 0.99
let (framework, confidence_bump) = self.detect_node_framework(&json, root);
let mut confidence = if framework.is_empty() {
    if has_ts { 0.97 } else { 0.93 }
} else if confidence_bump >= 0.08 {
    0.99  // direct dependency match — certain
} else {
    if has_ts { 0.97 } else { 0.93 } + confidence_bump
};
```

Apply same logic to Python (direct dep in requirements.txt = 0.99),
Ruby (direct gem = 0.99), PHP (direct composer dep = 0.99).

---

## PART D — Version bump and release

### Task D1 — Add `crush-deploy` to workspace

**File:** `Cargo.toml` (workspace root)

In `members`, add:
```toml
"crates/crush-deploy",
```

In `[workspace.dependencies]`, add:
```toml
crush-deploy = { path = "crates/crush-deploy" }
ssh2 = "0.9"
```

Also add `ssh2` to `crush-deploy/Cargo.toml` dependencies.

---

### Task D2 — Bump version to 0.4.0

**File:** `Cargo.toml` (workspace root)

Change `version = "0.3.0"` to `version = "0.4.0"`.

---

### Task D3 — Build, tag, and release

On the Linux VPS:
```sh
cd /root/crush
git pull origin main

# Linux build
/root/.cargo/bin/cargo build --release -p crush-cli

# Windows cross-compile
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  /root/.cargo/bin/cargo build --release --target x86_64-pc-windows-gnu -p crush-cli

x86_64-w64-mingw32-strip target/x86_64-pc-windows-gnu/release/crush-cli.exe
cp target/x86_64-pc-windows-gnu/release/crush-cli.exe crush-0.4.0-windows-x86_64.exe

git tag v0.4.0
git push origin v0.4.0
gh release create v0.4.0 crush-0.4.0-windows-x86_64.exe \
  --title "Crush v0.4.0" \
  --notes-file scripts/release_notes_v4.md
```

---

## Priority order

1. **A1 + A2 + A3 + A9 + A10** — deploy core (SSH provider + Crushfile extension + state + CLI wiring). This is the most visible feature — ship it first even with only SSH support.
2. **A4** (Hetzner) — cheapest to test, simplest API.
3. **C1 + C2** (detector scoring + Python dep parsing) — correctness fixes that affect every project.
4. **C3 + C4 + C5** (Ruby/PHP parsing + confidence) — completeness.
5. **B1 + B2** (eBPF metric programs) — Linux only, requires kernel >= 5.4 + BTF.
6. **B3** (enhanced TUI) — builds on B1+B2 but can fall back to /proc.
7. **A5 + A6 + A7 + A8** (DO + AWS + GCP + Fly) — additional providers.
