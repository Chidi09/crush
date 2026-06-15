//! L4 TCP gateway with a hot-swappable upstream — the traffic director that
//! makes zero-downtime (blue-green) deploys possible.
//!
//! It listens on the public port and splices each accepted connection to the
//! *current* upstream, which it reads from a tiny "target file" on every new
//! connection. Flipping blue→green is therefore just an atomic file write: new
//! connections go to the new release, while connections already in flight finish
//! against the upstream they started on (natural drain). No restart, no dropped
//! requests, no nginx.
//!
//! Pairs with [`crate::bluegreen`], which writes the target file after a new
//! release passes its health check.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

/// Read the upstream port from a target file. The file holds either a bare port
/// (`8081`) or `host:port` (`127.0.0.1:8081`); we only need the port since the
/// gateway always dials loopback. Returns `None` if absent/garbage.
pub fn read_target(path: &Path) -> Option<u16> {
    let raw = std::fs::read_to_string(path).ok()?;
    let token = raw.trim();
    let port = token.rsplit(':').next().unwrap_or(token);
    port.trim().parse::<u16>().ok()
}

/// Atomically point the target file at `port` (write temp + rename) so the
/// gateway never reads a half-written file.
pub fn write_target(path: &Path, port: u16) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format!("{port}\n"))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Run the gateway until the process ends. Binds `0.0.0.0:listen` and forwards
/// each connection to `127.0.0.1:<current target>`.
pub async fn run_gateway(listen: u16, target_file: PathBuf) -> std::io::Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], listen).into();
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (client, _peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let target_file = target_file.clone();
        tokio::spawn(async move {
            // Resolve the upstream at connect time so flips take effect for new
            // connections immediately.
            let Some(port) = read_target(&target_file) else {
                return; // no upstream configured yet — drop quietly
            };
            if let Ok(mut upstream) = TcpStream::connect(("127.0.0.1", port)).await {
                let mut client = client;
                // Splice both directions; copy_bidirectional handles half-closes.
                let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
                let _ = upstream.shutdown().await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bare_port_and_hostport() {
        let d = tempfile::TempDir::new().unwrap();
        let f = d.path().join("t");
        std::fs::write(&f, "8081\n").unwrap();
        assert_eq!(read_target(&f), Some(8081));
        std::fs::write(&f, "127.0.0.1:9090").unwrap();
        assert_eq!(read_target(&f), Some(9090));
    }

    #[test]
    fn missing_or_garbage_is_none() {
        let d = tempfile::TempDir::new().unwrap();
        let f = d.path().join("missing");
        assert_eq!(read_target(&f), None);
        std::fs::write(&f, "not-a-port").unwrap();
        assert_eq!(read_target(&f), None);
    }

    #[test]
    fn write_then_read_roundtrips_atomically() {
        let d = tempfile::TempDir::new().unwrap();
        let f = d.path().join("nested/target");
        write_target(&f, 1234).unwrap();
        assert_eq!(read_target(&f), Some(1234));
        // overwrite (the flip)
        write_target(&f, 5678).unwrap();
        assert_eq!(read_target(&f), Some(5678));
        // temp file shouldn't linger
        assert!(!f.with_extension("tmp").exists());
    }
}
