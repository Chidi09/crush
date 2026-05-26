pub mod engine;
pub mod wasi;
pub mod component;
pub mod limits;
pub mod network;
pub mod image;
pub mod router;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use tokio::sync::Mutex;
use sha2::Digest;
use wasmtime::Store;
use crush_types::{RuntimeBackend, Container, Result, CrushError};
use engine::WasmEngine;
use component::{HostContext, ComponentLoader};
use limits::WasmResourceLimits;
use network::WasmNetworkProxy;
use image::WasmImage;

/// Default fuel budget for each WASM module execution (10 billion instructions).
pub const DEFAULT_FUEL: u64 = 10_000_000_000;


pub use router::{RuntimeRouter, ContainerRuntime};

pub struct WasmRuntime {
    engine: WasmEngine,
    instances: Arc<Mutex<HashMap<String, WasmInstance>>>,
    limits: WasmResourceLimits,
    limits_memory: u64,
    network: WasmNetworkProxy,
    data_dir: PathBuf,
}

#[derive(Clone)]
struct WasmInstance {
    container_id: String,
    wasm_path: PathBuf,
    compiled: bool,
    exit_code: Option<i32>,
}

impl WasmRuntime {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let aot_cache_dir = data_dir.join("wasm-cache");
        let engine = WasmEngine::new(aot_cache_dir)?;

        Ok(Self {
            engine,
            instances: Arc::new(Mutex::new(HashMap::new())),
            limits: WasmResourceLimits::default(),
            limits_memory: 256,
            network: WasmNetworkProxy::with_defaults(),
            data_dir,
        })
    }

    pub fn with_memory_limit(mut self, mb: u64) -> Self {
        self.limits = self.limits.with_memory_mb(mb);
        self.limits_memory = mb;
        self
    }

    pub fn with_fuel(mut self, fuel: u64) -> Self {
        self.limits = self.limits.with_fuel(fuel);
        self
    }

    pub fn with_outbound_allowlist(mut self, hosts: Vec<String>) -> Self {
        self.network = WasmNetworkProxy::new(hosts);
        self
    }

    pub fn engine(&self) -> &WasmEngine {
        &self.engine
    }
}

#[async_trait]
impl RuntimeBackend for WasmRuntime {
    async fn create(&self, container: &Container, spec_path: &PathBuf) -> Result<()> {
        let wasm_source = self.resolve_wasm_source(container, spec_path)?;
        let wasm_bytes = WasmImage::load_wasm(&wasm_source)?;

        if !WasmImage::detect(&wasm_bytes) {
            return Err(CrushError::WasmError("Not a valid WASM binary (missing \\0asm magic)".to_string()));
        }

        let cache_dir = self.data_dir.join("wasm-cache");
        WasmImage::compile_and_cache(&wasm_bytes, &cache_dir)?;

        let cache_key = sha256_hex(&wasm_bytes);
        let _module = self.engine.load_or_compile_cached(&wasm_bytes, &cache_key)?;

        for port in &container.ports {
            self.network.bind_host_port(port).await?;
        }

        let mut instances = self.instances.lock().await;
        instances.insert(container.id.clone(), WasmInstance {
            container_id: container.id.clone(),
            wasm_path: wasm_source,
            compiled: true,
            exit_code: None,
        });

        Ok(())
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        let instance = {
            let instances = self.instances.lock().await;
            instances.get(container_id).cloned()
                .ok_or_else(|| CrushError::ContainerNotFound(container_id.to_string()))?
        };

        let wasm_bytes = WasmImage::load_wasm(&instance.wasm_path)?;
        let module = self.engine.load_or_compile_cached(
            &wasm_bytes,
            &sha256_hex(&wasm_bytes),
        )?;

        let ctx = wasi::WasiContext::build(
            &[container_id.to_string()],
            &[],
            &[],
            Some(&self.data_dir),
            self.limits_memory,
        )?;

        let host_ctx = HostContext::new(ctx, wasmtime_wasi_http::WasiHttpCtx::new());

        let mut store = Store::new(self.engine.engine(), host_ctx);
        self.limits.apply_to_store(&mut store);


        let linker = ComponentLoader::create_linker(self.engine.engine())?;

        let _instance = ComponentLoader::load_and_link(
            &mut store,
            &linker,
            &wasm_bytes,
            self.engine.engine(),
        )?;

        Ok(())
    }

    async fn stop(&self, container_id: &str, _timeout_seconds: u32) -> Result<()> {
        let mut instances = self.instances.lock().await;
        if let Some(inst) = instances.get_mut(container_id) {
            inst.exit_code = Some(0);
        }
        Ok(())
    }

    async fn pause(&self, _container_id: &str) -> Result<()> {
        Ok(())
    }

    async fn resume(&self, _container_id: &str) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, container_id: &str) -> Result<()> {
        let mut instances = self.instances.lock().await;
        instances.remove(container_id);
        Ok(())
    }

    async fn exec(&self, container_id: &str, _command: &[String], _tty: bool) -> Result<i32> {
        let instances = self.instances.lock().await;
        let inst = instances.get(container_id)
            .ok_or_else(|| CrushError::ContainerNotFound(container_id.to_string()))?;
        Ok(inst.exit_code.unwrap_or(-1))
    }

    async fn get_pid(&self, _container_id: &str) -> Result<Option<u32>> {
        Ok(None)
    }
}

impl WasmRuntime {
    fn resolve_wasm_source(&self, container: &Container, spec_path: &PathBuf) -> Result<PathBuf> {
        if Path::new(&container.image).exists() {
            return Ok(PathBuf::from(&container.image));
        }

        let meta_path = spec_path.join("runtime-meta.json");
        if meta_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<image::WasmMeta>(&content) {
                    if meta.abi == "wasi-preview2" {
                        let image_path = spec_path.join("image.wasm");
                        if image_path.exists() {
                            return Ok(image_path);
                        }
                    }
                }
            }
        }

        Err(CrushError::WasmError(format!(
            "WASM binary not found for container {}. Provide a .wasm file or runtime-meta.json",
            container.id
        )))
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Returns the default epoch-tick deadline applied to every WASM execution.
pub fn default_epoch_deadline() -> u64 {
    DEFAULT_FUEL
}
