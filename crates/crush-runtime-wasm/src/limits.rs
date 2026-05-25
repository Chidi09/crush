use std::time::Duration;
use wasmtime::Store;
use crush_types::{Result, CrushError};
use crate::component::HostContext;

pub struct WasmResourceLimits {
    pub max_memory_bytes: u64,
    pub max_fuel: u64,
    pub max_file_descriptors: u32,
    pub max_socket_connections: u32,
    pub epoch_ticks: u64,
    pub epoch_tick_duration_ms: u64,
}

impl Default for WasmResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024,
            max_fuel: 10_000_000,
            max_file_descriptors: 512,
            max_socket_connections: 32,
            epoch_ticks: 100,
            epoch_tick_duration_ms: 10,
        }
    }
}

impl WasmResourceLimits {
    pub fn with_memory_mb(mut self, mb: u64) -> Self {
        self.max_memory_bytes = mb * 1024 * 1024;
        self
    }

    pub fn with_fuel(mut self, fuel: u64) -> Self {
        self.max_fuel = fuel;
        self
    }

    pub fn apply_to_store(&self, store: &mut Store<HostContext>) {
        store.set_fuel(self.max_fuel).ok();
    }

    pub fn check_memory(store: &Store<HostContext>, max_bytes: u64) -> Result<()> {
        if let Some(mem) = store.as_context().data().wasi_ctx.memory() {
            let current = mem.current_pages(&store) as u64 * 65536;
            if current > max_bytes {
                return Err(CrushError::WasmError(format!(
                    "WASM memory exceeded: {} pages ({} MB) > limit {} MB",
                    mem.current_pages(&store),
                    current / 1024 / 1024,
                    max_bytes / 1024 / 1024
                )));
            }
        }
        Ok(())
    }

    pub fn check_fuel(store: &Store<HostContext>) -> Result<u64> {
        store.fuel_consumed()
            .map_err(|e| CrushError::WasmError(format!("Fuel check error: {}", e)))
    }
}
