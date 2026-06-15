# Crush L7 Gateway & TLS/ACME Design

## 1. Goal
Provide a built-in "Traefik-equivalent" L7 reverse proxy for `crush`. This allows users to map domains to their local deployed projects, with automatic TLS provisioning via Let's Encrypt.

## 2. Architecture
- **Proxy Engine:** We will evolve `crates/crush-build/src/gateway.rs` from a simple L4 TCP splice into a full HTTP/1.1 and HTTP/2 proxy using `hyper` and `hyper-util`.
- **Configuration Hot-Reload:** 
  - The domains configuration is stored in `~/.crush/domains.json` (managed via the GUI's Domains tab).
  - The gateway will use the `notify` crate to watch `~/.crush/domains.json` for changes. Upon change, it atomically updates its internal `HashMap<String, u16>` mapping Host headers to upstream loopback ports.
- **TLS/ACME:**
  - We will embed `instant-acme` and `rustls`.
  - The gateway will bind to ports 80 and 443.
  - Port 80 will answer `.well-known/acme-challenge` requests directly from a local challenge cache, and redirect all other traffic to HTTPS (port 443).
  - When a new domain is added to `domains.json`, the proxy background task will automatically trigger `instant-acme` to negotiate a certificate with Let's Encrypt (using the HTTP-01 challenge).
  - Certificates and private keys will be cached on disk under `~/.crush/certs/`. `rustls` will be configured with a custom `ResolvesServerCert` implementation to dynamically load these certificates at handshake time, allowing zero-downtime certificate renewal and domain additions.

## 3. Security Considerations
- **ACME Rate Limits:** We will implement simple backoff and avoid hammering Let's Encrypt if validation fails.
- **Exposure:** The proxy will bind `0.0.0.0:80` and `0.0.0.0:443`. It will *only* forward requests for domains explicitly listed in `domains.json`, returning 404/403 for unknown Host headers.
- **Upstream Protection:** Upstreams remain bound only to `127.0.0.1`.

## 4. Execution Plan
1. Add `rustls`, `tokio-rustls`, and `instant-acme` to `crates/crush-build/Cargo.toml`.
2. Implement the core HTTP reverse proxy (`hyper`) mapping `Host` headers to upstreams using the watched `domains.json`.
3. Implement the ACME worker thread that polls the domains list, checks certificate expiration, and negotiates new certs.
4. Integrate `rustls` into the Hyper server to serve the certificates.

Please review this sub-design and provide sign-off to proceed with the implementation.
