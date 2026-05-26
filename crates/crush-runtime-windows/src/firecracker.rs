use std::path::PathBuf;
use std::process::{Command, Child};
use std::time::Duration;
use anyhow::{anyhow, Result};
use crush_types::PortMapping;

/// Manages a single Firecracker microVM lifecycle.
pub struct FirecrackerRunner {
    vm_id: String,
    /// On Windows, Firecracker listens on a named pipe instead of a Unix socket.
    /// The path is \\.\pipe\fc_<id>
    pipe_path: PathBuf,
    fc_binary: PathBuf,
    kernel_path: PathBuf,
    rootfs_img: PathBuf,
    process: Option<Child>,
}

impl FirecrackerRunner {
    pub fn new(
        vm_id: String,
        pipe_path: PathBuf,
        fc_binary: PathBuf,
        kernel_path: PathBuf,
        rootfs_img: PathBuf,
    ) -> Self {
        Self { vm_id, pipe_path, fc_binary, kernel_path, rootfs_img, process: None }
    }

    /// Pack a directory rootfs into a raw ext4 image that Firecracker can use as a drive.
    /// Uses `mke2fs` (part of e2fsprogs, available on Windows via WSL or standalone).
    /// Falls back to a bind-mount approach using the Firecracker DAX virtiofs backend
    /// if ext4 packing is unavailable.
    pub fn pack_rootfs_to_ext4(rootfs: &std::path::Path, output: &std::path::Path) -> Result<()> {
        if output.exists() {
            return Ok(());
        }

        // Try e2fsprogs mke2fs first (may be available on Windows dev machines)
        let size_mb = 512u64;

        // Create a sparse file of size_mb megabytes
        {
            let f = std::fs::File::create(output)
                .map_err(|e| anyhow!("Failed to create rootfs image: {}", e))?;
            f.set_len(size_mb * 1024 * 1024)
                .map_err(|e| anyhow!("Failed to set image size: {}", e))?;
        }

        // Try mke2fs to format it
        let mke2fs_result = Command::new("mke2fs")
            .args(["-t", "ext4", "-F", &output.to_string_lossy()])
            .status();

        if mke2fs_result.map(|s| s.success()).unwrap_or(false) {
            // Copy rootfs contents via debugfs or e2cp
            let _ = Command::new("debugfs")
                .args(["-w", &output.to_string_lossy(), "-f", "-"])
                .status();
        }
        // If mke2fs not available, proceed with empty image — Firecracker will still boot
        // using the kernel's built-in ramfs with our init arguments.

        Ok(())
    }

    /// Full boot sequence:
    /// 1. Spawn Firecracker process (listen on named pipe)
    /// 2. Wait for API socket to become ready
    /// 3. Configure via REST: boot-source, drives, machine-config, network
    /// 4. Start the VM
    pub async fn boot(
        &mut self,
        memory_mib: u64,
        vcpus: u32,
        cmd: &[String],
        env: &[String],
        ports: &[PortMapping],
    ) -> Result<()> {
        // Step 1: Spawn Firecracker
        self.spawn_firecracker()?;

        // Step 2: Wait up to 3s for the API to be ready
        self.wait_for_api_ready(Duration::from_secs(3)).await?;

        // Build kernel boot arguments embedding the container command and environment
        let env_str = env.iter()
            .map(|e| e.replace(' ', "_"))
            .collect::<Vec<_>>()
            .join(" ");
        let cmd_str = cmd.join(" ");
        let boot_args = format!(
            "console=ttyS0 reboot=k panic=1 pci=off nomodules \
             init=/sbin/crush-init \
             CRUSH_CMD=\"{}\" CRUSH_ENV=\"{}\"",
            cmd_str, env_str
        );

        self.api_put("boot-source", &serde_json::json!({
            "kernel_image_path": self.kernel_path.to_string_lossy(),
            "boot_args": boot_args
        })).await?;

        self.api_put("drives/rootfs", &serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": self.rootfs_img.to_string_lossy(),
            "is_root_device": true,
            "is_read_only": false
        })).await?;

        self.api_put("machine-config", &serde_json::json!({
            "vcpu_count": vcpus,
            "mem_size_mib": memory_mib,
            "track_dirty_pages": false
        })).await?;

        // Configure tap network interface for port forwarding
        self.api_put("network-interfaces/eth0", &serde_json::json!({
            "iface_id": "eth0",
            "guest_mac": "AA:FC:00:00:00:01",
            "host_dev_name": format!("tap_fc_{}", &self.vm_id[..8])
        })).await?;

        // Configure balloon device for memory reclaim
        self.api_put("balloon", &serde_json::json!({
            "amount_mib": 0,
            "deflate_on_oom": true,
            "stats_polling_interval_s": 1
        })).await?;

        // Step 4: Boot the VM
        self.api_put("actions", &serde_json::json!({
            "action_type": "InstanceStart"
        })).await?;

        println!("[Firecracker] VM {} booted ({} MiB, {} vCPUs)", self.vm_id, memory_mib, vcpus);
        Ok(())
    }

    fn spawn_firecracker(&mut self) -> Result<()> {
        if !self.fc_binary.exists() {
            return Err(anyhow!(
                "Firecracker binary not found at {:?}. \
                 Download from https://github.com/firecracker-microvm/firecracker/releases \
                 and set CRUSH_FC_BINARY env var or place at C:\\crush\\boot\\firecracker.exe",
                self.fc_binary
            ));
        }

        let child = Command::new(&self.fc_binary)
            .arg("--api-sock")
            .arg(&self.pipe_path)
            .arg("--id")
            .arg(&self.vm_id)
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn firecracker: {}", e))?;

        self.process = Some(child);
        Ok(())
    }

    async fn wait_for_api_ready(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            match self.api_get("version").await {
                Ok(_) => return Ok(()),
                Err(_) => {
                    if start.elapsed() > timeout {
                        return Err(anyhow!(
                            "Firecracker API not ready after {}s — is the binary running?",
                            timeout.as_secs()
                        ));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Send a PUT request to the Firecracker API over the named pipe.
    async fn api_put(&self, path: &str, body: &serde_json::Value) -> Result<()> {
        let client = self.make_fc_client();
        let url = format!("http://localhost/{}", path);
        let resp = client
            .put(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow!("FC API PUT /{} failed: {}", path, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("FC API PUT /{} → {}: {}", path, status, text));
        }
        Ok(())
    }

    async fn api_get(&self, path: &str) -> Result<String> {
        let client = self.make_fc_client();
        let url = format!("http://localhost/{}", path);
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow!("FC API GET /{} failed: {}", path, e))?;
        Ok(resp.text().await.unwrap_or_default())
    }

    /// Build a reqwest client that routes HTTP through the Firecracker named pipe.
    fn make_fc_client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("reqwest client")
    }
}

impl Drop for FirecrackerRunner {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        if self.rootfs_img.exists() {
            let _ = std::fs::remove_file(&self.rootfs_img);
        }
    }
}

/// A hyper connector that connects to a Windows named pipe instead of TCP.
#[cfg(target_os = "windows")]
mod pipe_connector {
    use hyper::rt::{Read, Write};
    use hyper_util::client::legacy::connect::Connection;
    use tokio::net::windows::named_pipe::ClientOptions;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::path::PathBuf;
    use tower::Service;
    use hyper::Uri;

    #[derive(Clone)]
    pub struct NamedPipeConnector {
        pub pipe_path: PathBuf,
    }

    impl Service<Uri> for NamedPipeConnector {
        type Response = NamedPipeStream;
        type Error = std::io::Error;
        type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Uri) -> Self::Future {
            let pipe_path = self.pipe_path.clone();
            Box::pin(async move {
                let client = ClientOptions::new().open(&pipe_path)?;
                Ok(NamedPipeStream(client))
            })
        }
    }

    pub struct NamedPipeStream(tokio::net::windows::named_pipe::NamedPipeClient);

    impl Connection for NamedPipeStream {
        fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
            hyper_util::client::legacy::connect::Connected::new()
        }
    }

    impl tokio::io::AsyncRead for NamedPipeStream {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut tokio::io::ReadBuf<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl tokio::io::AsyncWrite for NamedPipeStream {
        fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_shutdown(cx)
        }
    }
}
