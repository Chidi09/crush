use std::path::{Path, PathBuf};
use std::fs;
use crush_types::{Result, CrushError};
use crate::compress::{self, CompressionFormat};

const OPAQUE_WHITEOUT: &str = ".wh..wh..opq";
const WHITEOUT_PREFIX: &str = ".wh.";

pub fn safe_unpack(archive: &mut tar::Archive<impl std::io::Read>, destination: &Path) -> Result<()> {
    let dest = destination.canonicalize()
        .unwrap_or_else(|_| destination.to_path_buf());

    for entry in archive.entries()
        .map_err(|e| CrushError::StorageError(format!("Tar entry error: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| CrushError::StorageError(format!("Tar entry read error: {}", e)))?;

        let path = entry.path()
            .map_err(|e| CrushError::StorageError(format!("Tar path error: {}", e)))?
            .to_path_buf();

        // ⚠ CRITICAL: Prevent tar-slip / path traversal
        // Join then canonicalize, verify containment in destination
        let target = dest.join(&path);
        let canonical = if target.exists() {
            target.canonicalize().unwrap_or(target.clone())
        } else {
            // For new files, resolve parent dir and check containment
            if let Some(parent) = target.parent() {
                let parent_canon = parent.canonicalize()
                    .map_err(|e| CrushError::StorageError(format!("Parent resolve: {}", e)))?;
                if !parent_canon.starts_with(&dest) {
                    return Err(CrushError::StorageError(format!(
                        "Path traversal blocked: {:?} escapes destination {:?}", path, dest
                    )));
                }
            }
            target.clone()
        };

        if !canonical.starts_with(&dest) {
            return Err(CrushError::StorageError(format!(
                "Path traversal blocked: {:?} resolves outside {:?}", path, dest
            )));
        }

        // Handle whiteout files
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();

        let whiteout_target = dest.join(&parent);

        if filename == OPAQUE_WHITEOUT {
            if let Ok(entries) = fs::read_dir(&whiteout_target) {
                for e in entries.flatten() {
                    let _ = fs::remove_file(e.path());
                }
            }
            continue;
        }

        if let Some(whiteout_name) = filename.strip_prefix(WHITEOUT_PREFIX) {
            let w_path = whiteout_target.join(whiteout_name);
            let _ = fs::remove_file(&w_path);
            let _ = fs::remove_dir_all(&w_path);
            continue;
        }

        let entry_type = entry.header().entry_type();

        if entry_type == tar::EntryType::Directory {
            fs::create_dir_all(&target)
                .map_err(|e| CrushError::StorageError(format!("mkdir error: {}", e)))?;
        } else if entry_type == tar::EntryType::Symlink {
            let link = entry.link_name()
                .map_err(|e| CrushError::StorageError(format!("symlink error: {}", e)))?
                .unwrap_or_default();
            // Validate symlink target stays within destination
            let link_target = dest.join(&link);
            if link_target.exists() {
                if let Ok(canon) = link_target.canonicalize() {
                    if !canon.starts_with(&dest) {
                        return Err(CrushError::StorageError("Symlink traversal blocked".to_string()));
                    }
                }
            }
            let _ = fs::remove_file(&target);
            #[cfg(unix)]
            { std::os::unix::fs::symlink(&link, &target)
                .map_err(|e| CrushError::StorageError(format!("symlink: {}", e)))?; }
        } else if entry_type == tar::EntryType::Link {
            let link_target = entry.link_name()
                .map_err(|e| CrushError::StorageError(format!("link error: {}", e)))?
                .unwrap_or_default();
            let hard_target = dest.join(&link_target);
            if hard_target.exists() {
                if let Ok(canon) = hard_target.canonicalize() {
                    if !canon.starts_with(&dest) {
                        return Err(CrushError::StorageError("Hardlink traversal blocked".to_string()));
                    }
                }
            }
            let _ = fs::remove_file(&target);
            fs::hard_link(&hard_target, &target)
                .map_err(|e| CrushError::StorageError(format!("hardlink: {}", e)))?;
        } else {
            if let Some(parent_dir) = target.parent() {
                fs::create_dir_all(parent_dir)
                    .map_err(|e| CrushError::StorageError(format!("mkdir: {}", e)))?;
            }
            entry.unpack(&target)
                .map_err(|e| CrushError::StorageError(format!("unpack: {}", e)))?;
        }
    }
    Ok(())
}

pub struct LayerExtractor { destination: PathBuf }

impl LayerExtractor {
    pub fn new(destination: &Path) -> Self { Self { destination: destination.to_path_buf() } }

    pub fn extract_layer<R: std::io::Read>(&self, reader: R, format: CompressionFormat) -> Result<()> {
        let decompressed = compress::decompress_stream(reader, format)?;
        let mut archive = tar::Archive::new(&decompressed[..]);
        safe_unpack(&mut archive, &self.destination)
    }

    pub fn extract_layer_streamed<R: std::io::Read>(&self, reader: R, format: CompressionFormat) -> Result<()> {
        let mut pipe: Vec<u8> = Vec::new();
        compress::decompress_stream_into(reader, &mut pipe, format)?;
        let mut archive = tar::Archive::new(&pipe[..]);
        safe_unpack(&mut archive, &self.destination)
    }
}
