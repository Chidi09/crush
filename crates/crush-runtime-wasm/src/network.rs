use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError, PortMapping, Protocol};

pub struct WasmNetworkProxy {
    allowed_outbound_hosts: Vec<String>,
    proxy_ports: Arc<Mutex<HashMap<u16, tokio::net::TcpListener>>>,
}

impl WasmNetworkProxy {
    pub fn new(outbound_allowlist: Vec<String>) -> Self {
        Self {
            allowed_outbound_hosts: outbound_allowlist,
            proxy_ports: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_defaults() -> Self {
        // Restricted default: only common package registries and APIs.
        // ⚠ "*" would give WASM unrestricted outbound access.
        Self::new(vec![
            "registry.npmjs.org".to_string(),
            "crates.io".to_string(),
            "pypi.org".to_string(),
            "files.pythonhosted.org".to_string(),
            "github.com".to_string(),
            "api.github.com".to_string(),
        ])
    }

    pub async fn bind_host_port(&self, port_mapping: &PortMapping) -> Result<()> {
        let addr = format!("{}:{}", port_mapping.host_ip, port_mapping.host_port);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| CrushError::NetworkError(format!("WASM port bind failed: {}", e)))?;

        let mut ports = self.proxy_ports.lock().await;
        ports.insert(port_mapping.host_port, listener);
        Ok(())
    }

    pub async fn start_proxy_loop(port: u16, container_port: u16) -> Result<()> {
        let proxy_addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&proxy_addr).await
            .map_err(|e| CrushError::NetworkError(format!("Proxy bind failed: {}", e)))?;

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((inbound, _)) => {
                        let outbound = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", container_port)).await;
                        if let Ok(outbound) = outbound {
                            tokio::spawn(proxy_bidirectional(inbound, outbound));
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(())
    }

    pub fn is_host_allowed(&self, host: &str) -> bool {
        if self.allowed_outbound_hosts.contains(&"*".to_string()) {
            return true;
        }
        self.allowed_outbound_hosts.iter().any(|allowed| {
            if allowed.starts_with('*') {
                host.ends_with(&allowed[1..])
            } else {
                host == allowed
            }
        })
    }
}

async fn proxy_bidirectional(mut inbound: tokio::net::TcpStream, mut outbound: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        loop {
            match ri.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if wo.write_all(&buf[..n]).await.is_err() { break; }
                }
            }
        }
    });

    let server_to_client = tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        loop {
            match ro.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if wi.write_all(&buf[..n]).await.is_err() { break; }
                }
            }
        }
    });

    let _ = tokio::join!(client_to_server, server_to_client);
}
