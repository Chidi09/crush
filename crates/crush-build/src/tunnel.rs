//! Localhost tunneling — expose a local port to the public internet so webhook
//! senders (Paystack, Stripe, Flutterwave, Clerk, …) can reach a dev server.
//!
//! Provider model is a **fallback chain**, not a hosted relay:
//!   1. `cloudflared` quick tunnel — free, no account, no domain. Auto-provisioned
//!      to the crush data dir if it isn't on `PATH`. This is the default.
//!   2. `ngrok` — used as a fallback only when `NGROK_AUTHTOKEN` is set (ngrok
//!      reads that env var itself). Must be installed.
//!   3. `outray` — used as a fallback only when `OUTRAY_TOKEN` is set. Must be
//!      installed.
//!
//! An explicit `--provider` pins one of the above. The chain means a developer
//! with zero setup still gets an HTTPS URL, while a token unlocks a sticky
//! provider when cloudflared is unavailable. A future `SelfHosted` provider
//! (own domain + edge server) slots in here without touching callers.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelProvider {
    Cloudflared,
    Ngrok,
    Outray,
}

impl TunnelProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            TunnelProvider::Cloudflared => "cloudflared",
            TunnelProvider::Ngrok => "ngrok",
            TunnelProvider::Outray => "outray",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "cloudflared" | "cloudflare" | "cf" => Some(TunnelProvider::Cloudflared),
            "ngrok" => Some(TunnelProvider::Ngrok),
            "outray" => Some(TunnelProvider::Outray),
            _ => None,
        }
    }
}

/// Streamed status from a live tunnel, for the CLI/GUI to surface.
#[derive(Debug, Clone)]
pub enum TunnelEvent {
    /// The public URL is up. Webhook senders can be pointed here.
    Ready { url: String, provider: TunnelProvider },
    /// One line of the provider's own output (kept for diagnostics).
    Log { line: String },
    /// The tunnel process exited.
    Exited { code: i32 },
}

/// A running tunnel. Holds the child process; drop or call [`Tunnel::shutdown`]
/// to tear it down.
pub struct Tunnel {
    pub url: String,
    pub provider: TunnelProvider,
    child: Child,
}

impl Tunnel {
    pub fn url(&self) -> &str { &self.url }
    pub fn provider(&self) -> TunnelProvider { self.provider }

    /// Wait until the tunnel process exits on its own (e.g. it crashed or the
    /// edge dropped it). Returns the exit code.
    pub async fn wait(&mut self) -> i32 {
        self.child.wait().await.ok().and_then(|s| s.code()).unwrap_or(-1)
    }

    /// Tear the tunnel down.
    pub async fn shutdown(mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }
}

/// Decide which providers to try, in order. An explicit name pins exactly one.
/// Otherwise cloudflared leads, with token-gated fallbacks appended.
pub fn provider_chain(explicit: Option<&str>) -> Vec<TunnelProvider> {
    if let Some(name) = explicit {
        return TunnelProvider::from_name(name)
            .map(|p| vec![p])
            .unwrap_or_else(|| vec![TunnelProvider::Cloudflared]);
    }
    let mut chain = vec![TunnelProvider::Cloudflared];
    if std::env::var("NGROK_AUTHTOKEN").is_ok() {
        chain.push(TunnelProvider::Ngrok);
    }
    if std::env::var("OUTRAY_TOKEN").is_ok() {
        chain.push(TunnelProvider::Outray);
    }
    chain
}

/// Open a tunnel to `localhost:port`, trying each provider in the chain until
/// one yields a public URL. Ongoing provider output is forwarded to `tx` as
/// [`TunnelEvent::Log`]; the returned [`Tunnel`] owns the live process.
pub async fn open(
    port: u16,
    explicit: Option<&str>,
    tx: mpsc::Sender<TunnelEvent>,
) -> anyhow::Result<Tunnel> {
    let chain = provider_chain(explicit);
    let mut last_err: Option<anyhow::Error> = None;

    for provider in chain {
        match open_one(provider, port, tx.clone()).await {
            Ok(tunnel) => {
                let _ = tx
                    .send(TunnelEvent::Ready { url: tunnel.url.clone(), provider })
                    .await;
                return Ok(tunnel);
            }
            Err(e) => {
                let _ = tx
                    .send(TunnelEvent::Log {
                        line: format!("{} unavailable: {e}", provider.as_str()),
                    })
                    .await;
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no tunnel provider available")))
}

/// Spawn one provider and wait (up to ~30s) for it to announce a public URL.
async fn open_one(
    provider: TunnelProvider,
    port: u16,
    tx: mpsc::Sender<TunnelEvent>,
) -> anyhow::Result<Tunnel> {
    let mut cmd = build_command(provider, port).await?;
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to start {}: {e}", provider.as_str()))?;

    // Merge stdout+stderr into one line channel — providers print the URL to
    // either depending on version.
    let (line_tx, mut line_rx) = mpsc::channel::<String>(256);
    pipe_lines(child.stdout.take(), line_tx.clone());
    pipe_lines(child.stderr.take(), line_tx);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let url = loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            let _ = child.start_kill();
            anyhow::bail!("{} did not produce a public URL in time", provider.as_str());
        }
        tokio::select! {
            line = line_rx.recv() => {
                let Some(line) = line else {
                    // pipes closed → process exited before a URL appeared.
                    let code = child.wait().await.ok().and_then(|s| s.code()).unwrap_or(-1);
                    anyhow::bail!("{} exited ({code}) before producing a URL", provider.as_str());
                };
                if let Some(url) = parse_url(provider, &line) {
                    // Keep draining remaining output in the background so the
                    // pipe never fills and the process stays alive.
                    spawn_drainer(line_rx, tx);
                    break url;
                }
                let _ = tx.send(TunnelEvent::Log { line }).await;
            }
            _ = tokio::time::sleep(remaining) => {
                let _ = child.start_kill();
                anyhow::bail!("{} did not produce a public URL in time", provider.as_str());
            }
        }
    };

    Ok(Tunnel { url, provider, child })
}

/// Build the provider command, provisioning cloudflared on demand.
async fn build_command(provider: TunnelProvider, port: u16) -> anyhow::Result<Command> {
    let target = format!("http://localhost:{port}");
    match provider {
        TunnelProvider::Cloudflared => {
            let bin = ensure_cloudflared().await?;
            let mut cmd = Command::new(bin);
            cmd.arg("tunnel")
                .arg("--no-autoupdate")
                .arg("--url")
                .arg(&target);
            Ok(cmd)
        }
        TunnelProvider::Ngrok => {
            ensure_on_path("ngrok").await?;
            let mut cmd = Command::new("ngrok");
            // JSON log to stdout gives us a stable `"url":"https://…"` field.
            cmd.arg("http")
                .arg(port.to_string())
                .arg("--log")
                .arg("stdout")
                .arg("--log-format")
                .arg("json");
            Ok(cmd)
        }
        TunnelProvider::Outray => {
            ensure_on_path("outray").await?;
            let mut cmd = Command::new("outray");
            cmd.arg("http").arg(port.to_string());
            if let Ok(token) = std::env::var("OUTRAY_TOKEN") {
                // Pass through; outray also reads this from its own config.
                cmd.env("OUTRAY_TOKEN", token);
            }
            Ok(cmd)
        }
    }
}

/// Extract the public URL from a provider's output line.
fn parse_url(provider: TunnelProvider, line: &str) -> Option<String> {
    match provider {
        TunnelProvider::Cloudflared => {
            // "... https://random-words.trycloudflare.com ..."
            if !line.contains("trycloudflare.com") {
                return None;
            }
            extract_https(line)
        }
        TunnelProvider::Ngrok => {
            // JSON: {"lvl":"info","msg":"started tunnel",...,"url":"https://x.ngrok-free.app"}
            if let Some(rest) = line.split("\"url\":\"").nth(1) {
                let url = rest.split('"').next().unwrap_or("");
                if url.starts_with("https://") {
                    return Some(url.to_string());
                }
            }
            None
        }
        TunnelProvider::Outray => extract_https(line),
    }
}

/// Find the first `https://…` token in a line, trimmed of trailing punctuation
/// and any surrounding box-drawing/whitespace.
fn extract_https(line: &str) -> Option<String> {
    let start = line.find("https://")?;
    let tail = &line[start..];
    let end = tail
        .find(|c: char| c.is_whitespace() || c == '|' || c == ')' || c == '"' || c == '\'')
        .unwrap_or(tail.len());
    let url = tail[..end].trim_end_matches(['.', ',']);
    if url.len() > "https://".len() {
        Some(url.to_string())
    } else {
        None
    }
}

/// Forward a child stream's lines into a channel.
fn pipe_lines(
    pipe: Option<impl tokio::io::AsyncRead + Unpin + Send + 'static>,
    tx: mpsc::Sender<String>,
) {
    let Some(pipe) = pipe else { return };
    tokio::spawn(async move {
        let mut lines = tokio::io::BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if tx.send(line).await.is_err() {
                break;
            }
        }
    });
}

/// After the URL is found, keep draining the provider's output as log events.
fn spawn_drainer(mut line_rx: mpsc::Receiver<String>, tx: mpsc::Sender<TunnelEvent>) {
    tokio::spawn(async move {
        while let Some(line) = line_rx.recv().await {
            if tx.send(TunnelEvent::Log { line }).await.is_err() {
                break;
            }
        }
    });
}

/// Verify a tool is runnable on PATH, else fail with an actionable message.
async fn ensure_on_path(tool: &str) -> anyhow::Result<()> {
    let ok = Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        anyhow::bail!("`{tool}` not found on PATH")
    }
}

/// Locate cloudflared: prefer one on PATH, else the crush-managed copy,
/// downloading it on first use. macOS isn't auto-provisioned (the release is a
/// tarball) — point at Homebrew instead.
async fn ensure_cloudflared() -> anyhow::Result<PathBuf> {
    if ensure_on_path("cloudflared").await.is_ok() {
        return Ok(PathBuf::from("cloudflared"));
    }
    let managed = managed_cloudflared_path();
    if managed.exists() {
        return Ok(managed);
    }
    download_cloudflared(&managed).await?;
    Ok(managed)
}

fn managed_cloudflared_path() -> PathBuf {
    let bin_dir = crush_types::dirs_or_default().join("bin");
    let name = if cfg!(windows) { "cloudflared.exe" } else { "cloudflared" };
    bin_dir.join(name)
}

/// Download the static cloudflared binary for this OS/arch into `dest`.
async fn download_cloudflared(dest: &Path) -> anyhow::Result<()> {
    let url = cloudflared_release_url()?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| anyhow::anyhow!("downloading cloudflared: {e}"))?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("downloading cloudflared: {e}"))?;
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| anyhow::anyhow!("reading cloudflared download: {e}"))?;
    std::fs::write(dest, &bytes)
        .map_err(|e| anyhow::anyhow!("writing cloudflared to {}: {e}", dest.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(dest, perms)?;
    }
    Ok(())
}

fn cloudflared_release_url() -> anyhow::Result<String> {
    const BASE: &str =
        "https://github.com/cloudflare/cloudflared/releases/latest/download";
    let asset = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "cloudflared-windows-amd64.exe",
        ("windows", "aarch64") => "cloudflared-windows-amd64.exe", // emulated; no native arm64 asset
        ("linux", "x86_64") => "cloudflared-linux-amd64",
        ("linux", "aarch64") => "cloudflared-linux-arm64",
        ("linux", "arm") => "cloudflared-linux-arm",
        ("macos", _) => anyhow::bail!(
            "install cloudflared on macOS with `brew install cloudflared`, then re-run"
        ),
        (os, arch) => anyhow::bail!("no cloudflared build for {os}/{arch} — install it manually"),
    };
    Ok(format!("{BASE}/{asset}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudflared_url_parse() {
        let line = "2024-01-01T00:00:00Z INF |  https://brave-cat-meadow.trycloudflare.com   |";
        assert_eq!(
            parse_url(TunnelProvider::Cloudflared, line).as_deref(),
            Some("https://brave-cat-meadow.trycloudflare.com")
        );
    }

    #[test]
    fn cloudflared_ignores_non_url_lines() {
        let line = "2024-01-01T00:00:00Z INF Requesting new quick Tunnel on trycloudflare.com...";
        // contains the host but no https URL token
        assert_eq!(parse_url(TunnelProvider::Cloudflared, line), None);
    }

    #[test]
    fn ngrok_json_url_parse() {
        let line = r#"{"lvl":"info","msg":"started tunnel","name":"command_line","addr":"http://localhost:8000","url":"https://1a2b.ngrok-free.app"}"#;
        assert_eq!(
            parse_url(TunnelProvider::Ngrok, line).as_deref(),
            Some("https://1a2b.ngrok-free.app")
        );
    }

    #[test]
    fn ngrok_ignores_http_url() {
        let line = r#"{"msg":"x","url":"http://localhost:4040"}"#;
        assert_eq!(parse_url(TunnelProvider::Ngrok, line), None);
    }

    #[test]
    fn chain_default_is_cloudflared_only_without_tokens() {
        // Note: relies on no tokens in the test env.
        std::env::remove_var("NGROK_AUTHTOKEN");
        std::env::remove_var("OUTRAY_TOKEN");
        assert_eq!(provider_chain(None), vec![TunnelProvider::Cloudflared]);
    }

    #[test]
    fn explicit_provider_pins_one() {
        assert_eq!(provider_chain(Some("ngrok")), vec![TunnelProvider::Ngrok]);
        assert_eq!(provider_chain(Some("outray")), vec![TunnelProvider::Outray]);
        assert_eq!(
            provider_chain(Some("bogus")),
            vec![TunnelProvider::Cloudflared]
        );
    }
}
