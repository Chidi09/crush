use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use wasmtime::{Engine, Config, Module, Store, FuelConsumed};
use wasmtime_wasi::WasiCtxBuilder;
use crush_types::{Result, CrushError};

pub struct WasmEngine {
    engine: Engine,
    aot_cache_dir: std::path::PathBuf,
}

impl WasmEngine {
    pub fn new(aot_cache_dir: std::path::PathBuf) -> Result<Self> {
        let mut config = Config::new();
        config.cranelift_opt_level(wasmtime::OptLevel::SpeedAndSize);
        config.async_support(true);
        config.wasm_component_model(true);
        config.wasm_multi_memory(true);
        config.wasm_memory64(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);

        let engine = Engine::new(&config)
            .map_err(|e| CrushError::WasmError(format!("Engine creation failed: {}", e)))?;

        std::fs::create_dir_all(&aot_cache_dir).ok();

        Ok(Self { engine, aot_cache_dir })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn compile_module(&self, wasm_bytes: &[u8]) -> Result<Module> {
        Module::new(&self.engine, wasm_bytes)
            .map_err(|e| CrushError::WasmError(format!("Module compilation failed: {}", e)))
    }

    pub fn load_or_compile_cached(&self, wasm_bytes: &[u8], cache_key: &str) -> Result<Module> {
        let cwasm_path = self.aot_cache_dir.join(format!("{}.cwasm", cache_key));

        if cwasm_path.exists() {
            unsafe {
                if let Ok(module) = Module::deserialize(&self.engine, &std::fs::read(&cwasm_path).unwrap()) {
                    return Ok(module);
                }
            }
        }

        let module = self.compile_module(wasm_bytes)?;

        let serialized = module.serialize()
            .map_err(|e| CrushError::WasmError(format!("Module serialization failed: {}", e)))?;
        std::fs::write(&cwasm_path, &serialized)
            .map_err(|e| CrushError::WasmError(format!("AOT cache write failed: {}", e)))?;

        Ok(module)
    }

    pub fn make_store<T: Send + 'static>(&self, data: T) -> Store<T> {
        Store::new(&self.engine, data)
    }

    pub fn fuel_remaining(store: &Store<impl Send>) -> Result<u64> {
        store.fuel_consumed()
            .map(|f| f.0)
            .map_err(|e| CrushError::WasmError(format!("Fuel read error: {}", e)))
    }

    pub fn set_fuel(store: &mut Store<impl Send>, fuel: u64) {
        store.set_fuel(fuel).ok();
    }

    pub fn interrupt_epoch(engine: &Engine) {
        engine.increment_epoch();
    }
}
