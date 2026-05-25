use std::sync::Arc;
use tokio::sync::Mutex;
use crush_types::{Result, CrushError, Container};
use crate::WasmRuntime;
use crate::image::WasmImage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContainerRuntime {
    Wasm,
    Native,
    Unknown,
}

pub struct RuntimeRouter {
    wasm_runtime: Arc<Mutex<WasmRuntime>>,
}

impl RuntimeRouter {
    pub fn new(wasm_runtime: WasmRuntime) -> Self {
        Self {
            wasm_runtime: Arc::new(Mutex::new(wasm_runtime)),
        }
    }

    pub fn detect_runtime(image_bytes: &[u8], container: &Container) -> ContainerRuntime {
        if container.image.ends_with(".wasm") {
            return ContainerRuntime::Wasm;
        }

        if let Some(meta) = container.image.strip_prefix("wasm:") {
            if !meta.is_empty() {
                return ContainerRuntime::Wasm;
            }
        }

        if WasmImage::detect(image_bytes) {
            return ContainerRuntime::Wasm;
        }

        ContainerRuntime::Native
    }

    pub async fn dispatch(
        &self,
        runtime: ContainerRuntime,
        container_id: &str,
    ) -> Result<()> {
        match runtime {
            ContainerRuntime::Wasm => {
                println!("Router: dispatching to WASM runtime for {}", container_id);
                Ok(())
            }
            ContainerRuntime::Native => {
                println!("Router: dispatching to native runtime for {}", container_id);
                Ok(())
            }
            ContainerRuntime::Unknown => {
                Err(CrushError::WasmError(format!(
                    "Unable to determine runtime for container {}", container_id
                )))
            }
        }
    }
}
