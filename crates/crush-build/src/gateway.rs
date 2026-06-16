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
use std::sync::Arc;
use tokio::sync::RwLock;

// L7 specific imports
use std::collections::HashMap;
use std::sync::RwLock as StdRwLock;
use std::time::{Duration, SystemTime};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, header};
use hyper_util::rt::TokioIo;
use rustls::ServerConfig;
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;

// Domains config format
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct DomainRecord {
    pub host: String,
    pub project: String,
    pub port: u16,
}

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

// ----------------------------------------------------------------------------
// L7 Gateway (HTTP + TLS)
// ----------------------------------------------------------------------------

/// Shared TLS cert table, keyed by SNI host. `__default__` is the fallback used
/// when a connection has no SNI or an unknown host. Sync `RwLock` because rustls'
/// `resolve` is a synchronous hot-path callback; async tasks take brief write locks.
const DEFAULT_SNI: &str = "__default__";
type CertMap = Arc<StdRwLock<HashMap<String, Arc<CertifiedKey>>>>;

#[derive(Clone)]
pub struct L7State {
    pub domains: Arc<RwLock<std::collections::HashMap<String, u16>>>,
    pub certs_dir: PathBuf,
    /// token -> key-authorization, served on port 80 for ACME HTTP-01 challenges.
    pub challenges: Arc<RwLock<HashMap<String, String>>>,
    pub certs: CertMap,
}

impl L7State {
    pub async fn reload_domains(&self, domains_file: &Path) {
        if let Ok(text) = tokio::fs::read_to_string(domains_file).await {
            if let Ok(records) = serde_json::from_str::<Vec<DomainRecord>>(&text) {
                let mut map = std::collections::HashMap::new();
                for r in &records {
                    map.insert(r.host.to_lowercase(), r.port);
                }
                *self.domains.write().await = map;
                // Make sure every mapped host has *some* working cert (self-signed
                // until ACME upgrades it), so TLS handshakes never fail.
                for r in &records {
                    ensure_cert_for_domain(&self.certs_dir, &r.host.to_lowercase(), &self.certs);
                }
            }
        }
    }
}

/// rustls cert picker: choose the cert matching the SNI host, else the default.
struct CertResolver { certs: CertMap }
impl std::fmt::Debug for CertResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("CertResolver") }
}
impl ResolvesServerCert for CertResolver {
    fn resolve(&self, hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let map = self.certs.read().ok()?;
        let name = hello.server_name().unwrap_or(DEFAULT_SNI);
        map.get(name).cloned().or_else(|| map.get(DEFAULT_SNI).cloned())
    }
}

/// Build a rustls `CertifiedKey` from PEM cert chain + private key.
fn certified_key_from_pem(cert_pem: &str, key_pem: &str) -> anyhow::Result<CertifiedKey> {
    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut cert_pem.as_bytes()).collect::<Result<Vec<_>, _>>()?;
    if certs.is_empty() {
        anyhow::bail!("no certificates found in PEM");
    }
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_pem.as_bytes())?
        .ok_or_else(|| anyhow::anyhow!("no private key found in PEM"))?;
    let signing_key = rustls::crypto::ring::sign::any_supported_type(&key)?;
    Ok(CertifiedKey::new(certs, signing_key))
}

fn self_signed_pem(domain: &str) -> anyhow::Result<(String, String)> {
    let c = rcgen::generate_simple_self_signed(vec![domain.to_string()])?;
    Ok((c.cert.pem(), c.key_pair.serialize_pem()))
}

fn cert_paths(dir: &Path, domain: &str) -> (PathBuf, PathBuf) {
    let safe: String = domain.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    (dir.join(format!("{safe}.crt")), dir.join(format!("{safe}.key")))
}

fn load_pem(dir: &Path, domain: &str) -> Option<(String, String)> {
    let (cp, kp) = cert_paths(dir, domain);
    Some((std::fs::read_to_string(cp).ok()?, std::fs::read_to_string(kp).ok()?))
}

fn save_pem(dir: &Path, domain: &str, cert_pem: &str, key_pem: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let (cp, kp) = cert_paths(dir, domain);
    std::fs::write(cp, cert_pem)?;
    std::fs::write(kp, key_pem)?;
    Ok(())
}

/// A public FQDN that Let's Encrypt can actually validate (has a dot, isn't a
/// local-only TLD or a bare IP). Local domains (`*.local`, `*.crush.local`,
/// `localhost`) correctly use a self-signed cert — ACME is impossible for them.
fn is_public_fqdn(domain: &str) -> bool {
    if domain == "localhost" || !domain.contains('.') { return false; }
    if domain.ends_with(".local") || domain.ends_with(".internal") || domain.ends_with(".test") || domain.ends_with(".localhost") {
        return false;
    }
    // crude IP check: all label chars numeric/dots
    if domain.chars().all(|c| c.is_ascii_digit() || c == '.') { return false; }
    true
}

/// Ensure `domain` has a cert in the resolver map: load from disk, else generate
/// and persist a self-signed one. Idempotent; never overwrites an existing entry.
fn ensure_cert_for_domain(certs_dir: &Path, domain: &str, certs: &CertMap) {
    if certs.read().map(|m| m.contains_key(domain)).unwrap_or(false) {
        return;
    }
    let pem = load_pem(certs_dir, domain).or_else(|| {
        let ss = self_signed_pem(domain).ok()?;
        let _ = save_pem(certs_dir, domain, &ss.0, &ss.1);
        Some(ss)
    });
    if let Some((cert_pem, key_pem)) = pem {
        if let Ok(ck) = certified_key_from_pem(&cert_pem, &key_pem) {
            if let Ok(mut m) = certs.write() {
                m.insert(domain.to_string(), Arc::new(ck));
            }
        }
    }
}

async fn handle_l7_request(req: Request<hyper::body::Incoming>, state: L7State) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let host = req.headers().get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|h| h.split(':').next().unwrap_or(h).to_lowercase());

    let port = if let Some(h) = host {
        state.domains.read().await.get(&h).copied()
    } else {
        None
    };

    let Some(port) = port else {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Domain not mapped")))
            .unwrap());
    };

    // Forward to localhost:port
    let client_stream = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(s) => s,
        Err(_) => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("Upstream offline")))
                .unwrap());
        }
    };
    
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(client_stream)).await?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("upstream connection failed: {:?}", e);
        }
    });

    let mut fw_req = Request::builder()
        .method(req.method())
        .uri(req.uri())
        .version(req.version());
    for (k, v) in req.headers() {
        fw_req = fw_req.header(k.clone(), v.clone());
    }
    let fw_req = fw_req.body(req.into_body()).unwrap();

    let res = sender.send_request(fw_req).await?;
    let (parts, body) = res.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    
    let mut fw_res = Response::builder().status(parts.status).version(parts.version);
    for (k, v) in parts.headers {
        if let Some(k) = k {
            fw_res = fw_res.header(k, v);
        }
    }
    Ok(fw_res.body(Full::new(body_bytes)).unwrap())
}

/// Port-80 handler: answer ACME HTTP-01 challenges from the in-memory token map,
/// and 301-redirect everything else to HTTPS.
async fn handle_port80(req: Request<hyper::body::Incoming>, state: L7State) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path().to_string();
    if let Some(token) = path.strip_prefix("/.well-known/acme-challenge/") {
        if let Some(key_auth) = state.challenges.read().await.get(token).cloned() {
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Full::new(Bytes::from(key_auth)))
                .unwrap());
        }
        return Ok(Response::builder().status(StatusCode::NOT_FOUND).body(Full::new(Bytes::new())).unwrap());
    }
    let host = req.headers().get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("");
    let location = format!("https://{host}{path}");
    Ok(Response::builder()
        .status(StatusCode::MOVED_PERMANENTLY)
        .header(header::LOCATION, location)
        .body(Full::new(Bytes::new()))
        .unwrap())
}

// ── R4.1: ACME circuit breaker ──────────────────────────────────────────────

/// Per-domain ACME failure state, persisted to `certs_dir/acme-state.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct AcmeDomainState {
    failures: u32,
    next_attempt_at: u64, // Unix seconds; 0 = may attempt immediately
}

type AcmeStateMap = std::collections::HashMap<String, AcmeDomainState>;

fn acme_state_path(certs_dir: &std::path::Path) -> std::path::PathBuf {
    certs_dir.join("acme-state.json")
}

fn load_acme_state(certs_dir: &std::path::Path) -> AcmeStateMap {
    std::fs::read_to_string(acme_state_path(certs_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_acme_state(certs_dir: &std::path::Path, state: &AcmeStateMap) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(acme_state_path(certs_dir), json);
    }
}

/// Compute the cooldown duration for `failures` attempts using exponential
/// back-off capped at 24 hours.
fn acme_cooldown(failures: u32) -> Duration {
    // 15 min, 1 h, 6 h, 24 h (max)
    let minutes: u64 = match failures {
        0 => 0,
        1 => 15,
        2 => 60,
        3 => 360,
        _ => 1440,
    };
    Duration::from_secs(minutes * 60)
}

/// Do a cheap DNS A/AAAA probe for `domain`. Returns `true` if the domain
/// resolves at all — we don't check whether it resolves to *us* (that would
/// require knowing our public IP), but at least avoid spending an ACME order
/// on a domain that has no DNS record at all.
fn domain_resolves(domain: &str) -> bool {
    use std::net::ToSocketAddrs;
    (domain, 443u16).to_socket_addrs().map(|mut i| i.next().is_some()).unwrap_or(false)
}

/// ACME worker: for each public FQDN without a real (non-self-signed, fresh)
/// certificate, obtain one from Let's Encrypt over HTTP-01 and swap it into the
/// live resolver. Self-signed local domains are skipped. Resilient: failures are
/// logged and retried next pass; the self-signed cert keeps serving meanwhile.
///
/// R4.1: Circuit breaker — exponential cooldown on failures; pre-flight DNS
/// check; state persisted to `certs_dir/acme-state.json`.
async fn acme_worker(state: L7State) {
    // ~/.crush/certs/acme-account.json — reuse one ACME account across runs.
    let account_path = state.certs_dir.join("acme-account.json");
    loop {
        let hosts: Vec<String> = {
            state.domains.read().await.keys().filter(|h| is_public_fqdn(h)).cloned().collect()
        };

        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut acme_state = load_acme_state(&state.certs_dir);

        for host in hosts {
            // Skip if we already hold an ACME cert younger than 60 days (LE = 90d).
            let (cp, _) = cert_paths(&state.certs_dir, &host);
            let marker = state.certs_dir.join(format!("{}.acme", host));
            let cert_fresh = marker.exists()
                && std::fs::metadata(&cp).and_then(|m| m.modified()).map(|t| {
                    SystemTime::now().duration_since(t).map(|d| d < Duration::from_secs(60 * 24 * 3600)).unwrap_or(false)
                }).unwrap_or(false);
            if cert_fresh { continue; }

            // R4.1: Check the per-domain circuit-breaker cooldown.
            let domain_state = acme_state.entry(host.clone()).or_default();
            if domain_state.next_attempt_at > now_secs {
                let wait = domain_state.next_attempt_at - now_secs;
                eprintln!("gateway: ACME for {host} in cooldown ({wait}s remaining); self-signed still serving");
                continue;
            }

            // R4.1: Pre-flight DNS probe — don't burn an ACME order if DNS isn't set.
            if !domain_resolves(&host) {
                eprintln!("gateway: ACME for {host}: domain does not resolve, skipping order");
                domain_state.failures += 1;
                domain_state.next_attempt_at = now_secs + acme_cooldown(domain_state.failures).as_secs();
                save_acme_state(&state.certs_dir, &acme_state);
                continue;
            }

            match obtain_acme_cert(&host, &account_path, &state).await {
                Ok(()) => {
                    let _ = std::fs::write(&marker, b"1");
                    println!("gateway: issued Let's Encrypt cert for {host}");
                    // Reset failure counter on success.
                    domain_state.failures = 0;
                    domain_state.next_attempt_at = 0;
                    save_acme_state(&state.certs_dir, &acme_state);
                }
                Err(e) => {
                    eprintln!("gateway: ACME for {host} failed (self-signed still serving): {e}");
                    domain_state.failures += 1;
                    let cooldown = acme_cooldown(domain_state.failures);
                    domain_state.next_attempt_at = now_secs + cooldown.as_secs();
                    save_acme_state(&state.certs_dir, &acme_state);
                    eprintln!("gateway: next ACME attempt for {host} in {}min", cooldown.as_secs() / 60);
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(3600)).await; // re-check hourly
    }
}

/// Run the full ACME HTTP-01 order for one domain and install the resulting cert.
async fn obtain_acme_cert(domain: &str, account_path: &Path, state: &L7State) -> anyhow::Result<()> {
    use instant_acme::{Account, AccountCredentials, NewAccount, NewOrder, Identifier, ChallengeType, OrderStatus, AuthorizationStatus, LetsEncrypt};

    // Load or create the ACME account.
    let account = if let Ok(text) = std::fs::read_to_string(account_path) {
        let creds: AccountCredentials = serde_json::from_str(&text)?;
        Account::from_credentials(creds).await?
    } else {
        let (account, creds) = Account::create(
            &NewAccount { contact: &[], terms_of_service_agreed: true, only_return_existing: false },
            LetsEncrypt::Production.url(),
            None,
        ).await?;
        if let Some(parent) = account_path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(account_path, serde_json::to_string(&creds)?);
        account
    };

    let identifier = Identifier::Dns(domain.to_string());
    let mut order = account.new_order(&NewOrder { identifiers: &[identifier] }).await?;

    let authorizations = order.authorizations().await?;
    for authz in &authorizations {
        if matches!(authz.status, AuthorizationStatus::Valid) { continue; }
        let challenge = authz.challenges.iter()
            .find(|c| c.r#type == ChallengeType::Http01)
            .ok_or_else(|| anyhow::anyhow!("no HTTP-01 challenge offered for {domain}"))?;
        let key_auth = order.key_authorization(challenge);
        state.challenges.write().await.insert(challenge.token.clone(), key_auth.as_str().to_string());
        order.set_challenge_ready(&challenge.url).await?;
    }

    // Poll until the order is ready (or fails).
    let mut delay = Duration::from_millis(500);
    let mut ok = false;
    for _ in 0..15 {
        tokio::time::sleep(delay).await;
        let state_now = order.refresh().await?;
        match state_now.status {
            OrderStatus::Ready => { ok = true; break; }
            OrderStatus::Invalid => anyhow::bail!("ACME order for {domain} became invalid"),
            _ => { delay = (delay * 2).min(Duration::from_secs(8)); }
        }
    }
    if !ok { anyhow::bail!("ACME order for {domain} not ready in time"); }

    // CSR with a fresh keypair, finalize, fetch the issued chain.
    let key_pair = rcgen::KeyPair::generate()?;
    let mut params = rcgen::CertificateParams::new(vec![domain.to_string()])?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    let csr = params.serialize_request(&key_pair)?;
    order.finalize(csr.der()).await?;

    let cert_chain_pem = {
        let mut chain = None;
        for _ in 0..10 {
            if let Some(c) = order.certificate().await? { chain = Some(c); break; }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        chain.ok_or_else(|| anyhow::anyhow!("certificate not issued for {domain}"))?
    };
    let key_pem = key_pair.serialize_pem();

    save_pem(&state.certs_dir, domain, &cert_chain_pem, &key_pem)?;
    let ck = certified_key_from_pem(&cert_chain_pem, &key_pem)?;
    if let Ok(mut m) = state.certs.write() {
        m.insert(domain.to_string(), Arc::new(ck));
    }
    // Clean up served challenge tokens for this order.
    state.challenges.write().await.clear();
    Ok(())
}

pub async fn run_l7_gateway(domains_file: PathBuf, certs_dir: PathBuf, mut shutdown: tokio::sync::broadcast::Receiver<()>) -> std::io::Result<()> {
    std::fs::create_dir_all(&certs_dir).ok();
    let certs: CertMap = Arc::new(StdRwLock::new(HashMap::new()));

    // A default self-signed cert so handshakes with no/unknown SNI still complete.
    if let Ok((c, k)) = self_signed_pem("localhost") {
        if let Ok(ck) = certified_key_from_pem(&c, &k) {
            certs.write().unwrap().insert(DEFAULT_SNI.to_string(), Arc::new(ck));
        }
    }

    let state = L7State {
        domains: Arc::new(RwLock::new(std::collections::HashMap::new())),
        certs_dir,
        challenges: Arc::new(RwLock::new(HashMap::new())),
        certs: certs.clone(),
    };
    state.reload_domains(&domains_file).await; // also seeds per-domain self-signed certs

    // Watch domains.json for changes (reload mapping + ensure certs).
    let df = domains_file.clone();
    let st = state.clone();
    tokio::spawn(async move {
        let mut last_mtime = std::time::UNIX_EPOCH;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Ok(meta) = tokio::fs::metadata(&df).await {
                if let Ok(mtime) = meta.modified() {
                    if mtime > last_mtime { last_mtime = mtime; st.reload_domains(&df).await; }
                }
            }
        }
    });

    // ACME issuance worker (no-op for local-only domains).
    tokio::spawn(acme_worker(state.clone()));

    // Port 80: ACME challenges + HTTPS redirect.
    let (conn_tx, mut conn_rx) = tokio::sync::mpsc::channel::<()>(1);
    
    {
        let st80 = state.clone();
        let mut shutdown80 = shutdown.resubscribe();
        let conn_tx80 = conn_tx.clone();
        tokio::spawn(async move {
            let Ok(l80) = TcpListener::bind::<SocketAddr>(([0, 0, 0, 0], 80).into()).await else {
                eprintln!("gateway: could not bind :80 (ACME challenges + redirect disabled)");
                return;
            };
            loop {
                tokio::select! {
                    _ = shutdown80.recv() => break,
                    res = l80.accept() => {
                        let Ok((stream, _)) = res else { continue };
                        let st = st80.clone();
                        let tx80 = conn_tx80.clone();
                        let mut task_shutdown = shutdown80.resubscribe();
                        tokio::spawn(async move {
                            let _tx = tx80;
                            let io = TokioIo::new(stream);
                            let mut conn = http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| handle_port80(req, st.clone())));
                            let mut conn = std::pin::pin!(conn);
                            tokio::select! {
                                _ = &mut conn => {}
                                _ = task_shutdown.recv() => {
                                    conn.as_mut().graceful_shutdown();
                                    let _ = conn.await;
                                }
                            }
                        });
                    }
                }
            }
        });
    }

    // Port 443: TLS via the dynamic per-SNI resolver.
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(CertResolver { certs }));
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let addr: SocketAddr = ([0, 0, 0, 0], 443).into();
    let listener = TcpListener::bind(addr).await?;
    println!("L7 Gateway listening on https://0.0.0.0:443 (+ :80 redirect/ACME)");

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("gateway: graceful shutdown initiated, draining connections...");
                break;
            }
            res = listener.accept() => {
                let (stream, _peer) = match res {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };
                let acceptor = acceptor.clone();
                let state = state.clone();
                let mut shutdown_clone = shutdown.resubscribe();
                let tx443 = conn_tx.clone();
                tokio::spawn(async move {
                    let _tx = tx443;
                    match acceptor.accept(stream).await {
                        Ok(tls_stream) => {
                            let io = TokioIo::new(tls_stream);
                            let mut conn = http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| handle_l7_request(req, state.clone())))
                                .with_upgrades();
                            let mut conn = std::pin::pin!(conn);
                            tokio::select! {
                                res = &mut conn => {
                                    if let Err(e) = res {
                                        eprintln!("HTTP serving error: {}", e);
                                    }
                                }
                                _ = shutdown_clone.recv() => {
                                    conn.as_mut().graceful_shutdown();
                                    let _ = conn.await;
                                }
                            }
                        }
                        Err(e) => eprintln!("TLS error: {}", e),
                    }
                });
            }
        }
    }
    
    drop(conn_tx);
    // Wait for in-flight requests to complete (bounded to 30s)
    let _ = tokio::time::timeout(Duration::from_secs(30), conn_rx.recv()).await;
    println!("gateway: shutdown complete.");
    Ok(())
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
    fn public_fqdn_vs_local() {
        assert!(is_public_fqdn("app.example.com"));
        assert!(is_public_fqdn("crush.dev"));
        assert!(!is_public_fqdn("localhost"));
        assert!(!is_public_fqdn("myapp.crush.local"));
        assert!(!is_public_fqdn("site.local"));
        assert!(!is_public_fqdn("api")); // no dot
        assert!(!is_public_fqdn("192.168.1.10")); // bare IP
    }

    #[test]
    fn self_signed_then_certified_key_roundtrips() {
        let (cert, key) = self_signed_pem("example.com").unwrap();
        assert!(cert.contains("BEGIN CERTIFICATE"));
        // Building a rustls CertifiedKey from the generated PEM must succeed.
        assert!(certified_key_from_pem(&cert, &key).is_ok());
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
