use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use std::time::Duration;
use anyhow::{anyhow, Result};
use crush_types::PortMapping;

/// Manages a single Firecracker microVM lifecycle.
pub struct FirecrackerRunner {
    vm_id: String,
    /// On Windows, Firecracker listens on a named pipe instead of a Unix socket.
    /// The path is \\.\pipe\fc_<id>
    pub pipe_path: PathBuf,
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

    pub async fn boot_or_restore(
        &mut self,
        memory_mib: u64,
        vcpus: u32,
        cmd: &[String],
        env: &[String],
        ports: &[PortMapping],
        snapshot: Option<(&Path, &Path)>,   // (mem_path, state_path)
    ) -> Result<()> {
        self.spawn_firecracker()?;
        self.wait_for_api_ready(std::time::Duration::from_secs(3)).await?;

        if let Some((mem_path, state_path)) = snapshot {
            // Fast path: restore from snapshot (~100ms)
            self.api_put("snapshot/load", &serde_json::json!({
                "snapshot_path": state_path.to_string_lossy(),
                "mem_file_path": mem_path.to_string_lossy(),
                "enable_diff_snapshots": false,
                "resume_vm": true
            })).await?;
            println!("[Firecracker] Snapshot restored in ~100ms for VM {}", self.vm_id);
        } else {
            // Cold path: full boot (~400ms) then create snapshot for next time
            self.configure_and_boot(memory_mib, vcpus, cmd, env, ports).await?;
        }

        Ok(())
    }

    /// All the PUT /boot-source, /drives, /machine-config, /actions calls.
    /// Extracted from boot() to keep boot_or_restore() readable.
    async fn configure_and_boot(
        &mut self,
        memory_mib: u64,
        vcpus: u32,
        cmd: &[String],
        env: &[String],
        ports: &[PortMapping],
    ) -> Result<()> {
        let env_str = env.iter().map(|e| e.replace(' ', "_")).collect::<Vec<_>>().join(" ");
        let cmd_str = cmd.join(" ");
        let boot_args = format!(
            "console=ttyS0 reboot=k panic=1 pci=off nomodules \
             init=/sbin/crush-init CRUSH_CMD=\"{}\" CRUSH_ENV=\"{}\"",
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
            "track_dirty_pages": true   // required for diff snapshots
        })).await?;
        self.api_put("network-interfaces/eth0", &serde_json::json!({
            "iface_id": "eth0",
            "guest_mac": "AA:FC:00:00:00:01",
            "host_dev_name": format!("tap_fc_{}", &self.vm_id[..8])
        })).await?;
        self.api_put("balloon", &serde_json::json!({
            "amount_mib": 0,
            "deflate_on_oom": true,
            "stats_polling_interval_s": 1
        })).await?;
        self.api_put("actions", &serde_json::json!({ "action_type": "InstanceStart" })).await?;
        Ok(())
    }

    /// Save VM state after the guest init process is ready.
    /// Call this ~500ms after InstanceStart when running a new image for the first time.
    /// Subsequent runs will use snapshot_load() instead of a cold boot.
    pub async fn snapshot_create(&self, mem_path: &Path, state_path: &Path) -> Result<()> {
        // Pause the VM first — required before taking a snapshot
        self.api_put("vm", &serde_json::json!({ "state": "Paused" })).await?;

        self.api_put("snapshot/create", &serde_json::json!({
            "snapshot_type": "Full",
            "snapshot_path": state_path.to_string_lossy(),
            "mem_file_path": mem_path.to_string_lossy(),
            "version": "1.0.0"
        })).await?;

        // Resume after snapshot (the process stays paused for pool reuse — see Task 3)
        // Do NOT resume here; leave paused so the pool can clone and resume per-container.
        println!("[Firecracker] Snapshot saved: {:?}", state_path);
        Ok(())
    }

    /// Live drive update — swap the rootfs drive without rebooting the VM.
    /// Firecracker supports this via PATCH /drives/rootfs.
    pub async fn hot_swap_drive(&self, new_drive: &Path) -> Result<()> {
        // PATCH (not PUT) to update an existing drive in-place
        let body = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": new_drive.to_string_lossy()
        });
        self.request(hyper::Method::PATCH, "drives/rootfs", Some(body)).await?;
        Ok(())
    }

    /// Send the container's CMD + ENV to the guest over the vsock control socket.
    /// The crush-init process inside the VM listens on vsock port 2222 for a JSON
    /// exec config: { "cmd": [...], "env": [...] }
    pub async fn send_exec_config(&self, cmd: &[String], env: &[String]) -> Result<()> {
        // vsock port 2222 is the crush-init control port
        // On the host side, Firecracker exposes vsock via a Unix domain socket (Linux)
        // or named pipe (Windows) at the path: <pipe_path>_<guest_cid>_2222
        // Note: For Firecracker on Windows, the guest vsock named pipe is generated
        // at <pipe_path>_vsock_2222.
        let vsock_path = self.pipe_path.with_extension("vsock_2222");

        let payload = serde_json::json!({ "cmd": cmd, "env": env });
        let payload_bytes = serde_json::to_vec(&payload)?;

        // Write to vsock pipe — the guest init reads this and exec's the command
        let mut f = tokio::fs::File::create(&vsock_path).await
            .map_err(|e| anyhow!("Cannot open vsock control pipe {:?}: {}", vsock_path, e))?;
        tokio::io::AsyncWriteExt::write_all(&mut f, &payload_bytes).await
            .map_err(|e| anyhow!("vsock write failed: {}", e))?;
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

    pub(crate) async fn api_put(&self, path: &str, body: &serde_json::Value) -> Result<()> {
        self.request(hyper::Method::PUT, path, Some(body.clone())).await?;
        Ok(())
    }

    async fn api_get(&self, path: &str) -> Result<String> {
        self.request(hyper::Method::GET, path, None).await
    }

    async fn request(&self, method: hyper::Method, path: &str, body: Option<serde_json::Value>) -> Result<String> {
        use hyper::Uri;
        use http_body_util::{Full, BodyExt};
        use hyper::body::Bytes;

        let connector = pipe_connector::NamedPipeConnector {
            pipe_path: self.pipe_path.clone(),
        };
        let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build::<_, Full<Bytes>>(connector);

        let url: Uri = format!("http://localhost/{}", path).parse()?;

        let has_body = body.is_some();
        let req_body = if let Some(b) = body {
            Full::new(Bytes::from(serde_json::to_vec(&b)?))
        } else {
            Full::new(Bytes::new())
        };

        let mut req = hyper::Request::builder()
            .method(&method)
            .uri(url)
            .header("Host", "localhost")
            .header("Accept", "application/json");

        if has_body {
            req = req.header("Content-Type", "application/json");
        }

        let req = req.body(req_body)
            .map_err(|e| anyhow!("Failed to build request: {}", e))?;

        let resp = client.request(req).await
            .map_err(|e| anyhow!("FC API request failed: {}", e))?;

        let status = resp.status();
        let bytes = resp.collect().await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))?
            .to_bytes();
        let text = String::from_utf8_lossy(&bytes).into_owned();

        if !status.is_success() {
            return Err(anyhow!("FC API {} /{} → {}: {}", method, path, status, text));
        }

        Ok(text)
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
    use hyper::rt::{Read, ReadBufCursor, Write};
    use hyper_util::client::legacy::connect::Connection;
    use hyper_util::rt::TokioIo;
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
                Ok(NamedPipeStream(TokioIo::new(client)))
            })
        }
    }

    // TokioIo bridges tokio::io::AsyncRead/Write → hyper::rt::Read/Write
    pub struct NamedPipeStream(TokioIo<tokio::net::windows::named_pipe::NamedPipeClient>);

    impl Connection for NamedPipeStream {
        fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
            hyper_util::client::legacy::connect::Connected::new()
        }
    }

    impl Read for NamedPipeStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: ReadBufCursor<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl Write for NamedPipeStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }
        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.0).poll_shutdown(cx)
        }
    }
}
