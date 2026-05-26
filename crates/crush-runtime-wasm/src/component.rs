use wasmtime::component::{Component, Linker, Instance, ResourceTable};
use wasmtime::Store;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, p2::{WasiHttpView, WasiHttpCtxView}};
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

        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| CrushError::WasmError(format!("WASI linker add failed: {}", e)))?;

        wasmtime_wasi_http::p2::add_only_http_to_linker_sync(&mut linker)
            .map_err(|e| CrushError::WasmError(format!("WASI HTTP linker add failed: {}", e)))?;

        Ok(linker)
    }
}

pub struct HostContext {
    pub wasi_ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
    pub table: ResourceTable,
}

impl HostContext {
    pub fn new(wasi_ctx: WasiCtx, http_ctx: WasiHttpCtx) -> Self {
        Self { wasi_ctx, http_ctx, table: ResourceTable::new() }
    }
}

impl WasiView for HostContext {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi_ctx, table: &mut self.table }
    }
}

impl WasiHttpView for HostContext {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http_ctx,
            table: &mut self.table,
            hooks: Default::default(),
        }
    }
}
