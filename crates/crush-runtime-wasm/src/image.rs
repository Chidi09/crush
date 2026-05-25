use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crush_types::{Result, CrushError};

const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D]; // b"\0asm"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmMeta {
    pub runtime: String,
    pub abi: String,
    pub entry_point: Option<String>,
    pub memory_max_mb: Option<u64>,
    pub fuel_max: Option<u64>,
}

pub struct WasmImage;

impl WasmImage {
    pub fn detect(wasm_bytes: &[u8]) -> bool {
        wasm_bytes.len() >= 4 && wasm_bytes[0..4] == WASM_MAGIC
    }

    pub fn detect_from_path(path: &Path) -> bool {
        if let Ok(mut f) = fs::File::open(path) {
            use std::io::Read;
            let mut header = [0u8; 4];
            if f.read_exact(&mut header).is_ok() {
                return header == WASM_MAGIC;
            }
        }
        false
    }

    pub fn compile_and_cache(wasm_bytes: &[u8], cache_dir: &Path) -> Result<PathBuf> {
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        let hash = hex::encode(hasher.finalize());
        let cache_path = cache_dir.join(format!("{}.wasm", hash));

        if !cache_path.exists() {
            fs::write(&cache_path, wasm_bytes)
                .map_err(|e| CrushError::StorageError(format!("CACHE write error: {}", e)))?;
        }

        Ok(cache_path)
    }

    pub fn load_wasm(path: &Path) -> Result<Vec<u8>> {
        fs::read(path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read WASM: {}", e)))
    }

    pub fn runtime_meta() -> WasmMeta {
        WasmMeta {
            runtime: "wasmtime".to_string(),
            abi: "wasi-preview2".to_string(),
            entry_point: Some("_start".to_string()),
            memory_max_mb: Some(256),
            fuel_max: Some(10_000_000),
        }
    }

    pub fn is_component(wasm_bytes: &[u8]) -> bool {
        wasm_bytes.windows(8).any(|w| w == b"\x00asm\x01\x00\x00\x00")
    }
}
