//! crush-proxy: built-in reverse proxy for dev/prod URL parity.
//!
//! Spawns a hyper 1.x HTTP/1.1 server that routes requests to sub-services
//! using longest-prefix-match. Handles WebSocket upgrades (Vite HMR, Next HMR).
//!
//! Unit A of PARITY_PLAN.md.

use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::anyhow;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::{Body, Frame, Incoming, SizeHint};
use hyper::client::conn::http1 as client_http1;
use hyper::server::conn::http1 as server_http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};

use crush_build::InferredStack;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyRoute {
    pub path_prefix: String,
    pub target_port: u16,
    pub strip_prefix: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyConfig {
    pub bind_port: u16,
    pub routes: Vec<ProxyRoute>,
}

// ── Route inference ───────────────────────────────────────────────────────────

/// Build a `ProxyConfig` from a detected multi-service stack.
/// Returns `None` for single-service stacks (no proxy needed).
///
/// TODO(parity): read `crush.toml` `[proxy]` table as priority-1 source.
pub fn infer_routes(stack: &InferredStack) -> Option<ProxyConfig> {
    if !stack.is_monorepo || stack.services.len() < 2 {
        return None;
    }

    // Pick a "backend" service: Go/Rust/Java/Python runtime, or "api" in name.
    let backend = stack.services.iter().find(|s| {
        let rt = s.runtime_type.to_lowercase();
        matches!(rt.as_str(), "go" | "rust" | "java" | "python")
            || s.name.to_lowercase().contains("api")
    })?;

    // Pick a "frontend" – first service that is NOT the backend.
    let frontend = stack.services.iter().find(|s| s.name != backend.name)?;

    Some(ProxyConfig {
        bind_port: 8000,
        routes: vec![
            ProxyRoute {
                path_prefix: "/api".to_string(),
                target_port: backend.port,
                strip_prefix: false,
            },
            ProxyRoute {
                path_prefix: "/".to_string(),
                target_port: frontend.port,
                strip_prefix: false,
            },
        ],
    })
}

// ── Body wrapper (streaming, never buffers) ───────────────────────────────────

/// Unified body type so we can return either a forwarded `Incoming` stream
/// or a small `Full<Bytes>` error body without boxing on the happy path.
enum ProxyBodyInner {
    Incoming(Incoming),
    Full(Full<Bytes>),
}

pub struct ProxyBody(ProxyBodyInner);

impl ProxyBody {
    fn incoming(body: Incoming) -> Self {
        Self(ProxyBodyInner::Incoming(body))
    }
    fn full(b: impl Into<Bytes>) -> Self {
        Self(ProxyBodyInner::Full(Full::new(b.into())))
    }
    fn error(msg: impl Into<String>) -> Self {
        Self::full(msg.into())
    }
}

impl Body for ProxyBody {
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut self.get_mut().0 {
            ProxyBodyInner::Incoming(inner) => {
                match Pin::new(inner).poll_frame(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Ready(Some(Ok(frame))) => Poll::Ready(Some(Ok(frame))),
                    Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    ))),
                }
            }
            ProxyBodyInner::Full(inner) => {
                // Full<Bytes> is infallible; map the impossible error into our trait object.
                match Pin::new(inner).poll_frame(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Ready(Some(Ok(frame))) => Poll::Ready(Some(Ok(frame))),
                    Poll::Ready(Some(Err(e))) => match e {},
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.0 {
            ProxyBodyInner::Incoming(inner) => inner.is_end_stream(),
            ProxyBodyInner::Full(inner) => inner.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.0 {
            ProxyBodyInner::Incoming(inner) => inner.size_hint(),
            ProxyBodyInner::Full(inner) => inner.size_hint(),
        }
    }
}

// ── Request handler ───────────────────────────────────────────────────────────

async fn handle(
    mut req: Request<Incoming>,
    routes: Vec<ProxyRoute>,
    peer: SocketAddr,
) -> Result<Response<ProxyBody>, std::convert::Infallible> {
    let path = req.uri().path().to_owned();

    // Longest-prefix-match
    let route = routes
        .iter()
        .filter(|r| path.starts_with(&r.path_prefix))
        .max_by_key(|r| r.path_prefix.len());

    let route = match route {
        Some(r) => r.clone(),
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(ProxyBody::error("404 Not Found (crush-proxy: no route matched)"))
                .unwrap());
        }
    };

    let is_ws_upgrade = req
        .headers()
        .get(hyper::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase().contains("websocket"))
        .unwrap_or(false);

    // Rewrite URI for upstream
    let new_path = if route.strip_prefix {
        let stripped = path
            .strip_prefix(&route.path_prefix)
            .unwrap_or(&path);
        if stripped.is_empty() { "/" } else { stripped }.to_owned()
    } else {
        path.clone()
    };
    let query = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
    let upstream_uri = format!("{new_path}{query}")
        .parse::<hyper::Uri>()
        .unwrap_or_else(|_| "/".parse().unwrap());

    // Connect to upstream TCP
    let stream = match TcpStream::connect(("127.0.0.1", route.target_port)).await {
        Ok(s) => s,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(ProxyBody::error(format!(
                    "502 Bad Gateway: upstream :{} unreachable: {e}",
                    route.target_port
                )))
                .unwrap());
        }
    };

    let io = TokioIo::new(stream);
    let (mut sender, conn) = match client_http1::Builder::new()
        .handshake::<_, Incoming>(io)
        .await
    {
        Ok(pair) => pair,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(ProxyBody::error(format!("502 Bad Gateway: handshake failed: {e}")))
                .unwrap());
        }
    };
    // Drive the connection in the background; ignore errors (upstream closed).
    tokio::spawn(async move { let _ = conn.await; });

    // Capture upgrade future *before* consuming req
    let upgrade_fut = if is_ws_upgrade {
        Some(hyper::upgrade::on(&mut req))
    } else {
        None
    };

    // Build forwarded request
    let (parts, body) = req.into_parts();
    let mut fwd = Request::builder()
        .method(parts.method.clone())
        .uri(upstream_uri)
        .version(parts.version);

    // Forward all headers except Host (rewrite it)
    for (k, v) in &parts.headers {
        if k == hyper::header::HOST {
            continue;
        }
        fwd = fwd.header(k, v);
    }
    fwd = fwd
        .header(hyper::header::HOST, format!("localhost:{}", route.target_port))
        .header("X-Forwarded-For", peer.ip().to_string())
        .header("X-Forwarded-Proto", "http")
        .header("X-Real-IP", peer.ip().to_string());

    let fwd_req = fwd.body(body).unwrap();

    // Send to upstream
    let mut upstream_resp = match sender.send_request(fwd_req).await {
        Ok(r) => r,
        Err(e) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(ProxyBody::error(format!("502 Bad Gateway: upstream send failed: {e}")))
                .unwrap());
        }
    };

    // WebSocket tunnel: if upstream agreed to upgrade, splice both sides.
    if is_ws_upgrade && upstream_resp.status() == StatusCode::SWITCHING_PROTOCOLS {
        if let Some(client_upgrade) = upgrade_fut {
            let upstream_upgrade = hyper::upgrade::on(&mut upstream_resp);
            tokio::spawn(async move {
                match tokio::try_join!(client_upgrade, upstream_upgrade) {
                    Ok((client_io, upstream_io)) => {
                        // hyper 1.x Upgraded implements hyper::rt::Read/Write;
                        // TokioIo adapts that to tokio::io::AsyncRead/Write.
                        let mut c = TokioIo::new(client_io);
                        let mut u = TokioIo::new(upstream_io);
                        let _ = tokio::io::copy_bidirectional(&mut c, &mut u).await;
                    }
                    Err(e) => {
                        eprintln!("[proxy] WebSocket upgrade failed: {e}");
                    }
                }
            });
        }
    }

    // Stream response body back to client
    let (resp_parts, resp_body) = upstream_resp.into_parts();
    let mut resp = Response::builder()
        .status(resp_parts.status)
        .version(resp_parts.version);
    for (k, v) in &resp_parts.headers {
        resp = resp.header(k, v);
    }
    Ok(resp.body(ProxyBody::incoming(resp_body)).unwrap())
}

// ── Server ────────────────────────────────────────────────────────────────────

/// Start the reverse proxy. Tries ports `cfg.bind_port` → `cfg.bind_port + 10`.
/// Returns the actual bound port.
/// The proxy stops when `shutdown` fires or the sender is dropped.
pub async fn run_proxy(
    cfg: ProxyConfig,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) -> anyhow::Result<u16> {
    let mut listener: Option<TcpListener> = None;
    let mut bound_port = 0u16;

    for port in cfg.bind_port..=(cfg.bind_port + 10) {
        let addr = format!("127.0.0.1:{port}");
        match TcpListener::bind(&addr).await {
            Ok(l) => {
                bound_port = port;
                listener = Some(l);
                break;
            }
            Err(_) => continue,
        }
    }

    let listener = listener.ok_or_else(|| {
        anyhow!(
            "crush-proxy: could not bind to any port in {}-{}",
            cfg.bind_port,
            cfg.bind_port + 10
        )
    })?;

    let routes = cfg.routes;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                res = listener.accept() => {
                    match res {
                        Ok((stream, peer)) => {
                            let routes_clone = routes.clone();
                            tokio::spawn(async move {
                                let io = TokioIo::new(stream);
                                let svc = service_fn(move |req| {
                                    let r = routes_clone.clone();
                                    async move { handle(req, r, peer).await }
                                });
                                let _ = server_http1::Builder::new()
                                    .serve_connection(io, svc)
                                    .with_upgrades() // enables WebSocket pass-through
                                    .await;
                            });
                        }
                        Err(e) => {
                            eprintln!("[proxy] accept error: {e}");
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok(bound_port)
}
