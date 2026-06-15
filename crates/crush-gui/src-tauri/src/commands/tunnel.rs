//! Public tunneling for the dashboard — expose a running app's port to the
//! internet so webhook senders (Paystack/Stripe/Clerk) can reach it in dev.
//!
//! Thin wrapper over `crush_build::tunnel`: the live tunnel process is parked
//! in [`AppState::tunnels`] keyed by local port so the UI can start, query, and
//! stop it. cloudflared is auto-provisioned; ngrok/outray need a token.

use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct TunnelInfo {
    pub url: String,
    pub provider: String,
    pub port: u16,
}

/// Open a public tunnel to `localhost:port`. Idempotent per port — if one is
/// already live for that port, its info is returned unchanged.
#[tauri::command]
pub async fn start_tunnel(
    port: u16,
    provider: Option<String>,
    state: State<'_, AppState>,
) -> Result<TunnelInfo, String> {
    if let Some(t) = state.tunnels.read().await.get(&port) {
        return Ok(TunnelInfo {
            url: t.url().to_string(),
            provider: t.provider().as_str().to_string(),
            port,
        });
    }

    // Logs are drained into the void here — the GUI surfaces the URL, not the
    // provider's chatter. (A future Mailbox-style panel could keep them.)
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let tunnel = crush_build::tunnel::open(port, provider.as_deref(), tx)
        .await
        .map_err(|e| e.to_string())?;

    let info = TunnelInfo {
        url: tunnel.url().to_string(),
        provider: tunnel.provider().as_str().to_string(),
        port,
    };
    state.tunnels.write().await.insert(port, tunnel);
    Ok(info)
}

/// Tear down the tunnel for a given port (no-op if none).
#[tauri::command]
pub async fn stop_tunnel(port: u16, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(tunnel) = state.tunnels.write().await.remove(&port) {
        tunnel.shutdown().await;
    }
    Ok(())
}

/// List currently-live tunnels.
#[tauri::command]
pub async fn list_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelInfo>, String> {
    let tunnels = state.tunnels.read().await;
    Ok(tunnels
        .iter()
        .map(|(port, t)| TunnelInfo {
            url: t.url().to_string(),
            provider: t.provider().as_str().to_string(),
            port: *port,
        })
        .collect())
}
