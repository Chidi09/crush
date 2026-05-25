use std::path::{Path, PathBuf};
use std::fs;
use crush_types::{Result, CrushError};

pub struct ConversionReport {
    pub before_size_mb: u64,
    pub after_size_mb: u64,
    pub layers_before: usize,
    pub layers_after: usize,
    pub compression: String,
    pub chunk_manifest: bool,
}

pub struct OciImageConverter;

impl OciImageConverter {
    pub fn new() -> Self { Self }

    pub fn convert(oci_path: &Path, output_path: &Path) -> Result<ConversionReport> {
        let before_size = dir_size(oci_path);
        let layer_count = count_layers(oci_path);

        let converted = output_path.join("converted");
        fs::create_dir_all(&converted)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;

        let mut layer_index = 0;
        let manifests_dir = oci_path.join("blobs").join("sha256");
        let layers_dir = oci_path.join("layers");

        let image_config = oci_path.join("config.json");
        if image_config.exists() {
            fs::copy(&image_config, converted.join("config.json"))
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
        }

        let entries = if layers_dir.exists() {
            let mut e: Vec<_> = fs::read_dir(&layers_dir)
                .map_err(|e| CrushError::StorageError(e.to_string()))?
                .filter_map(|e| e.ok())
                .collect();
            e.sort_by_key(|e| e.file_name());
            e
        } else {
            Vec::new()
        };

        for entry in &entries {
            let path = entry.path();
            if path.extension().map_or(true, |e| e != "tar" && e != "gz" && e != "zst") {
                continue;
            }

            let data = fs::read(&path)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;

            let zstd_data = recompress_to_zstd(&data)?;

            let layer_name = format!("layer_{:04}.tar.zst", layer_index);
            let out_path = converted.join(&layer_name);
            fs::write(&out_path, &zstd_data)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;

            if layer_index == 0 {
                generate_chunk_manifest(&zstd_data, &converted, &layer_name)?;
            }

            layer_index += 1;
        }

        let after_size = dir_size(&converted);
        let dedup_count = deduplicate_layers(&converted)?;

        Ok(ConversionReport {
            before_size_mb: before_size / 1024 / 1024,
            after_size_mb: after_size / 1024 / 1024,
            layers_before: entries.len(),
            layers_after: layer_index - dedup_count,
            compression: "zstd (level 3)".to_string(),
            chunk_manifest: true,
        })
    }
}

fn recompress_to_zstd(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;

    if data.len() < 2 { return Ok(data.to_vec()); }

    let decompressed = match (data[0], data[1]) {
        (0x1f, 0x8b) => {
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut buf = Vec::new();
            decoder.read_to_end(&mut buf)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            buf
        }
        _ => data.to_vec(),
    };

    let mut encoder = zstd::Encoder::new(Vec::new(), 3)
        .map_err(|e| CrushError::StorageError(e.to_string()))?;
    use std::io::Write;
    encoder.write_all(&decompressed)
        .map_err(|e| CrushError::StorageError(e.to_string()))?;
    encoder.finish()
        .map_err(|e| CrushError::StorageError(e.to_string()))
}

fn generate_chunk_manifest(data: &[u8], output_dir: &Path, layer_name: &str) -> Result<()> {
    use sha2::{Sha256, Digest};
    const CHUNK_SIZE: usize = 8 * 1024 * 1024;

    let mut chunks = Vec::new();
    let mut offset = 0u64;
    let mut index = 0u32;

    while offset < data.len() as u64 {
        let end = (offset as usize + CHUNK_SIZE).min(data.len());
        let chunk = &data[offset as usize..end];
        let mut hasher = Sha256::new();
        hasher.update(chunk);
        let hash = hex::encode(hasher.finalize());

        chunks.push(serde_json::json!({
            "index": index,
            "offset": offset,
            "size": chunk.len(),
            "sha256": hash,
        }));
        offset = end as u64;
        index += 1;
    }

    let manifest = serde_json::json!({
        "layer": layer_name,
        "chunk_size": CHUNK_SIZE,
        "chunks": chunks,
    });

    let manifest_path = output_dir.join("chunk_manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)
        .map_err(|e| CrushError::ImageError(e.to_string()))?)
        .map_err(|e| CrushError::StorageError(e.to_string()))?;

    Ok(())
}

fn deduplicate_layers(dir: &Path) -> Result<usize> {
    use sha2::{Sha256, Digest};
    let mut seen = std::collections::HashSet::new();
    let mut removed = 0usize;

    if !dir.is_dir() { return Ok(0); }

    for entry in fs::read_dir(dir)
        .map_err(|e| CrushError::StorageError(e.to_string()))?
        .flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        if let Ok(data) = fs::read(&path) {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let hash = hex::encode(hasher.finalize());
            if !seen.insert(hash) {
                fs::remove_file(&path).ok();
                removed += 1;
            }
        }
    }
    Ok(removed)
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn count_layers(path: &Path) -> usize {
    let layers = path.join("layers");
    if layers.exists() {
        fs::read_dir(&layers).map(|e| e.count()).unwrap_or(0)
    } else {
        0
    }
}
