use std::path::Path;
use wasmtime::{Store, Component, Linker, Instance};
use wasmtime_wasi::{WasiCtx, WasiImpl, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
use crush_types::{Result, CrushError};

pub struct ComponentLoader;

impl ComponentLoader {
    pub fn load_and_link(
        store: &mut Store<HostContext>,
        linker: &Linker<HostContext>,
        wasm_bytes: &[u8],
        engine: &wasmtime::Engine,
    ) -> Result<Instance> {
        let component = Component::new(engine, wasm_bytes)
            .map_err(|e| CrushError::WasmError(format!("Component compilation failed: {}", e)))?;

        let instance = linker.instantiate(store, &component)
            .map_err(|e| CrushError::WasmError(format!("Component instantiation failed: {}", e)))?;

        Ok(instance)
    }

    pub fn create_linker(engine: &wasmtime::Engine) -> Result<Linker<HostContext>> {
        let mut linker = Linker::<HostContext>::new(engine);

        wasmtime_wasi::add_to_linker_sync(&mut linker, |ctx| &mut ctx.wasi_ctx)
            .map_err(|e| CrushError::WasmError(format!("WASI linker add failed: {}", e)))?;

        wasmtime_wasi_http::add_only_http_to_linker_sync(&mut linker, |ctx| &mut ctx.http_ctx)
            .map_err(|e| CrushError::WasmError(format!("WASI HTTP linker add failed: {}", e)))?;

        Ok(linker)
    }
}

pub struct HostContext {
    pub wasi_ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
}
