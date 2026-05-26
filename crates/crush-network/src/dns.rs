use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use crush_types::{CrushError, Result};

pub struct DnsResolver {
    upstream: Vec<IpAddr>,
    records: Arc<Mutex<HashMap<String, Vec<IpAddr>>>>,
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            upstream: Self::read_host_resolv(),
            records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_container(&self, name: &str, ip: Ipv4Addr) {
        let mut records = self.records.blocking_lock();
        records
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(IpAddr::V4(ip));
    }

    pub fn unregister_container(&self, name: &str) {
        let mut records = self.records.blocking_lock();
        records.remove(name);
    }

    pub fn register_alias(&self, alias: &str, container_name: &str) {
        // Acquire lock ONCE to prevent self-deadlock
        let mut records = self.records.blocking_lock();
        let ips = records.get(container_name).cloned().unwrap_or_default();
        if !ips.is_empty() {
            records.insert(alias.to_string(), ips);
        }
    }

    pub async fn resolve(&self, name: &str) -> Option<Vec<IpAddr>> {
        let records = self.records.lock().await;
        records.get(name).cloned()
    }

    pub fn write_resolv_conf(container_root: &Path, nameservers: &[IpAddr]) -> Result<()> {
        let resolv_path = container_root.join("etc").join("resolv.conf");
        std::fs::create_dir_all(resolv_path.parent().unwrap())
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        let mut content = "nameserver 127.0.0.11\noptions ndots:0\n".to_string();
        for ns in nameservers {
            content.push_str(&format!("nameserver {}\n", ns));
        }
        std::fs::write(&resolv_path, content)
            .map_err(|e| CrushError::StorageError(e.to_string()))
    }

    /// Start a full async UDP DNS server on `bind_addr` (e.g. "127.0.0.11:53").
    ///
    /// The server:
    /// - Parses the query name from DNS wire format
    /// - Returns an A-record response if the name is found in `self.records`
    /// - Returns NXDOMAIN if not found and upstream forwarding also fails/is absent
    /// - Forwards unknown queries to 8.8.8.8:53 and relays the reply
    pub async fn start_server(&self, bind_addr: &str) -> Result<()> {
        let socket = UdpSocket::bind(bind_addr)
            .await
            .map_err(|e| CrushError::NetworkError(format!("DNS bind {}: {}", bind_addr, e)))?;

        // Share the socket via Arc so we can send replies from within the loop
        let socket = Arc::new(socket);
        let records = Arc::clone(&self.records);

        // Clone upstream list for use inside the async loop
        let upstream: Vec<IpAddr> = self.upstream.clone();

        tracing::info!("DNS server listening on {}", bind_addr);

        let mut buf = [0u8; 512];
        loop {
            let (len, peer) = socket
                .recv_from(&mut buf)
                .await
                .map_err(|e| CrushError::NetworkError(format!("DNS recv: {}", e)))?;

            if len < 12 {
                // Malformed – too short to be a DNS message
                continue;
            }

            let packet = buf[..len].to_vec();
            let sock = Arc::clone(&socket);
            let recs = Arc::clone(&records);
            let ups = upstream.clone();

            // Spawn a task so a single slow query doesn't block the loop
            tokio::spawn(async move {
                if let Err(e) = handle_dns_query(packet, peer, sock, recs, ups).await {
                    tracing::warn!("DNS handler error: {}", e);
                }
            });
        }
    }

    fn read_host_resolv() -> Vec<IpAddr> {
        std::fs::read_to_string("/etc/resolv.conf")
            .map(|c| {
                c.lines()
                    .filter_map(|l| l.strip_prefix("nameserver "))
                    .filter_map(|ns| ns.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// DNS wire-format helpers
// ---------------------------------------------------------------------------

/// Parse the DNS query name starting at byte offset 12 (immediately after the
/// 12-byte fixed header).  Returns `(name_lowercase, question_end_offset)` on
/// success, or `None` on malformed input.
///
/// Wire format: sequence of `<len><label>` segments terminated by a zero byte.
fn parse_query_name(packet: &[u8]) -> Option<(String, usize)> {
    let mut pos = 12usize;
    let mut labels: Vec<String> = Vec::new();

    loop {
        if pos >= packet.len() {
            return None;
        }
        let len = packet[pos] as usize;
        if len == 0 {
            pos += 1; // skip the terminating zero
            break;
        }
        pos += 1;
        if pos + len > packet.len() {
            return None;
        }
        let label = std::str::from_utf8(&packet[pos..pos + len]).ok()?;
        labels.push(label.to_ascii_lowercase());
        pos += len;
    }

    // After the QNAME there are QTYPE (2 bytes) + QCLASS (2 bytes)
    let question_end = pos + 4;
    if question_end > packet.len() {
        return None;
    }

    Some((labels.join("."), question_end))
}

/// Build a DNS A-record response for `ip`.
fn build_a_response(query: &[u8], question_end: usize, ip: Ipv4Addr) -> Vec<u8> {
    let mut resp = Vec::with_capacity(question_end + 16);

    // Transaction ID
    resp.extend_from_slice(&query[0..2]);
    // Flags: QR=1, Opcode=0, AA=1, TC=0, RD=1, RA=1, RCODE=0  → 0x8180
    resp.extend_from_slice(&[0x81, 0x80]);
    // QDCOUNT = 1
    resp.extend_from_slice(&[0x00, 0x01]);
    // ANCOUNT = 1
    resp.extend_from_slice(&[0x00, 0x01]);
    // NSCOUNT = 0
    resp.extend_from_slice(&[0x00, 0x00]);
    // ARCOUNT = 0
    resp.extend_from_slice(&[0x00, 0x00]);

    // Question section (copy verbatim from the query)
    resp.extend_from_slice(&query[12..question_end]);

    // Answer RR
    // Name: pointer to offset 12 (0xC00C)
    resp.extend_from_slice(&[0xC0, 0x0C]);
    // Type A (1)
    resp.extend_from_slice(&[0x00, 0x01]);
    // Class IN (1)
    resp.extend_from_slice(&[0x00, 0x01]);
    // TTL = 60 seconds
    resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]);
    // RDLENGTH = 4
    resp.extend_from_slice(&[0x00, 0x04]);
    // RDATA: IPv4 address in network byte order
    resp.extend_from_slice(&ip.octets());

    resp
}

/// Build an NXDOMAIN response (flags=0x8183).
fn build_nxdomain_response(query: &[u8], question_end: usize) -> Vec<u8> {
    let mut resp = Vec::with_capacity(question_end);

    // Transaction ID
    resp.extend_from_slice(&query[0..2]);
    // Flags: QR=1, AA=1, RD=1, RA=1, RCODE=3 (NXDOMAIN)  → 0x8183
    resp.extend_from_slice(&[0x81, 0x83]);
    // QDCOUNT = 1
    resp.extend_from_slice(&[0x00, 0x01]);
    // ANCOUNT = 0
    resp.extend_from_slice(&[0x00, 0x00]);
    // NSCOUNT = 0
    resp.extend_from_slice(&[0x00, 0x00]);
    // ARCOUNT = 0
    resp.extend_from_slice(&[0x00, 0x00]);

    // Question section (copy verbatim)
    resp.extend_from_slice(&query[12..question_end]);

    resp
}

/// Forward a raw DNS query to an upstream resolver and return the raw reply.
async fn forward_to_upstream(query: &[u8], upstream_ip: IpAddr) -> std::io::Result<Vec<u8>> {
    let upstream_addr: SocketAddr = SocketAddr::new(upstream_ip, 53);

    // Bind to an ephemeral local port
    let fwd = UdpSocket::bind("0.0.0.0:0").await?;
    fwd.send_to(query, upstream_addr).await?;

    let mut reply = vec![0u8; 512];
    // Wait up to 3 seconds for a reply
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        fwd.recv_from(&mut reply),
    )
    .await;

    match result {
        Ok(Ok((n, _))) => {
            reply.truncate(n);
            Ok(reply)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "upstream DNS timed out",
        )),
    }
}

/// Core per-query handler, executed in a spawned task.
async fn handle_dns_query(
    packet: Vec<u8>,
    peer: SocketAddr,
    socket: Arc<UdpSocket>,
    records: Arc<Mutex<HashMap<String, Vec<IpAddr>>>>,
    upstream: Vec<IpAddr>,
) -> std::io::Result<()> {
    // Only handle standard queries (QR=0, Opcode=0)
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    let is_query = (flags & 0x8000) == 0;
    let opcode = (flags >> 11) & 0x0F;

    if !is_query || opcode != 0 {
        // Not a standard query – ignore
        return Ok(());
    }

    let Some((name, question_end)) = parse_query_name(&packet) else {
        tracing::debug!("DNS: could not parse query name");
        return Ok(());
    };

    // QTYPE check – only handle A records (type 1) locally
    let qtype = u16::from_be_bytes([packet[question_end - 4], packet[question_end - 3]]);

    let response: Vec<u8> = if qtype == 1 {
        // A-record query: look up in local container records first
        let resolved = {
            let recs = records.lock().await;
            recs.get(&name).and_then(|ips| {
                ips.iter().find_map(|ip| {
                    if let IpAddr::V4(v4) = ip {
                        Some(*v4)
                    } else {
                        None
                    }
                })
            })
        };

        if let Some(ipv4) = resolved {
            tracing::debug!("DNS: local hit {} → {}", name, ipv4);
            build_a_response(&packet, question_end, ipv4)
        } else {
            // Forward to upstream
            let mut forwarded = None;
            for up in &upstream {
                match forward_to_upstream(&packet, *up).await {
                    Ok(reply) => {
                        forwarded = Some(reply);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!("DNS upstream {} failed: {}", up, e);
                    }
                }
            }

            // Fall back to Google DNS if our upstream list is empty/exhausted
            if forwarded.is_none() {
                let google = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
                match forward_to_upstream(&packet, google).await {
                    Ok(reply) => forwarded = Some(reply),
                    Err(e) => tracing::warn!("DNS Google fallback failed: {}", e),
                }
            }

            forwarded.unwrap_or_else(|| build_nxdomain_response(&packet, question_end))
        }
    } else {
        // Non-A query: forward directly upstream
        let mut forwarded = None;
        for up in &upstream {
            if let Ok(reply) = forward_to_upstream(&packet, *up).await {
                forwarded = Some(reply);
                break;
            }
        }
        if forwarded.is_none() {
            let google = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
            if let Ok(reply) = forward_to_upstream(&packet, google).await {
                forwarded = Some(reply);
            }
        }
        forwarded.unwrap_or_else(|| build_nxdomain_response(&packet, question_end))
    };

    socket.send_to(&response, peer).await?;
    Ok(())
}
