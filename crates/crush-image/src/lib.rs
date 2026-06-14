pub mod blob;
pub mod compress;
pub mod layer;
pub mod db;
pub mod multiarch;
pub mod gc;
pub mod lazy;
#[cfg(not(target_os = "windows"))]
pub mod fuse;

use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::io::Read;
use crush_types::{StorageBackend, Image, Result, CrushError};
use blob::BlobStore;
use db::ImageDatabase;
use compress::CompressionFormat;
use layer::LayerExtractor;

pub use crush_registry::RegistryClientHandle;

pub struct ImageStore {
    base_dir: PathBuf,
    blobs: BlobStore,
    db: ImageDatabase,
    registry_client: Arc<Mutex<RegistryClientHandle>>,
    lazy_mode: bool,
}

impl ImageStore {
    pub async fn new(base_dir: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&base_dir).await
            .map_err(|e| CrushError::StorageError(format!("Failed to create base dir: {}", e)))?;
        let db = ImageDatabase::new(&base_dir)?;
        let blobs = BlobStore::new(&base_dir);
        let config_path = base_dir.parent().map(|p| p.to_path_buf());
        Ok(Self {
            base_dir,
            blobs,
            db,
            registry_client: Arc::new(Mutex::new(RegistryClientHandle::new(config_path))),
            lazy_mode: false,
        })
    }

    pub fn enable_lazy_mode(&mut self) { self.lazy_mode = true; }
    pub fn blob_store(&self) -> &BlobStore { &self.blobs }
    pub fn database(&self) -> &ImageDatabase { &self.db }

    pub fn registry_for_tag(tag: &str) -> (String, String, String) {
        let tag = tag.trim_start_matches("docker://");
        let (registry, rest) = if let Some(pos) = tag.find('/') {
            let possible_registry = &tag[..pos];
            if possible_registry.contains('.') || possible_registry.contains(':') {
                (possible_registry.to_string(), tag[pos + 1..].to_string())
            } else {
                ("registry-1.docker.io".to_string(), tag.to_string())
            }
        } else {
            ("registry-1.docker.io".to_string(), tag.to_string())
        };
        let (image, reference) = if let Some(pos) = rest.rfind(':') {
            (rest[..pos].to_string(), rest[pos + 1..].to_string())
        } else {
            (rest.clone(), "latest".to_string())
        };
        let image = if !image.contains('/') { format!("library/{}", image) } else { image };
        (registry, image, reference)
    }

    pub async fn check_rate_limit(&self, resp: &reqwest::Response) -> Result<()> {
        if let Some(remaining) = resp.headers().get("RateLimit-Remaining") {
            if let Ok(val) = remaining.to_str().unwrap_or("9999").parse::<u32>() {
                if val < 5 {
                    let reset_str = resp.headers()
                        .get("RateLimit-Reset")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("60");
                    let _reset_secs: u64 = reset_str.parse().unwrap_or(60);
                }
            }
        }
        Ok(())
    }

    pub async fn set_registry_client(&self, client: RegistryClientHandle) {
        let mut c = self.registry_client.lock().await;
        *c = client;
    }

    pub async fn export_image(&self, tag: &str, output: &Path) -> Result<()> {
        let image = match self.db.get_image_by_digest(tag).await? {
            Some(img) => img,
            None => self.db.get_image_by_tag(tag).await?
                .ok_or_else(|| CrushError::ContainerNotFound(format!("Image {} not found", tag)))?,
        };

        // Create the output tarball
        let file = std::fs::File::create(output)
            .map_err(|e| CrushError::StorageError(format!("Failed to create export file: {}", e)))?;
        let mut archive = tar::Builder::new(file);

        // 1. Add layers as individual tar entries
        let mut manifest_layers = Vec::new();
        for (i, layer_digest) in image.layers.iter().enumerate() {
            let layer_path = self.blobs.path_for_digest(layer_digest);
            if layer_path.exists() {
                let layer_filename = format!("layer_{}.tar", i);
                let data = std::fs::read(&layer_path)
                    .map_err(|e| CrushError::StorageError(format!("Failed to read layer: {}", e)))?;
                
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                archive.append_data(&mut header, &layer_filename, std::io::Cursor::new(data))
                    .map_err(|e| CrushError::StorageError(format!("Failed to write layer to archive: {}", e)))?;
                
                manifest_layers.push(layer_filename);
            }
        }

        // 2. Add config.json
        let config_data = if let Some(ref cfg_digest) = image.config_digest {
            self.blobs.read_blob(cfg_digest).unwrap_or_default()
        } else {
            // Generate a fallback config
            serde_json::to_vec(&serde_json::json!({
                "config": {
                    "Cmd": image.cmd,
                    "Entrypoint": image.entrypoint,
                    "Env": image.env,
                },
                "architecture": image.architecture,
                "os": image.os,
                "os.version": image.os_version,
            })).unwrap_or_default()
        };

        let mut config_header = tar::Header::new_gnu();
        config_header.set_size(config_data.len() as u64);
        config_header.set_mode(0o644);
        archive.append_data(&mut config_header, "config.json", std::io::Cursor::new(config_data))
            .map_err(|e| CrushError::StorageError(format!("Failed to write config to archive: {}", e)))?;

        // 3. Add manifest.json
        let manifest_json = serde_json::json!([
            {
                "Config": "config.json",
                "RepoTags": vec![image.tag.clone()],
                "Layers": manifest_layers,
            }
        ]);
        let manifest_data = serde_json::to_vec(&manifest_json)
            .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;

        let mut manifest_header = tar::Header::new_gnu();
        manifest_header.set_size(manifest_data.len() as u64);
        manifest_header.set_mode(0o644);
        archive.append_data(&mut manifest_header, "manifest.json", std::io::Cursor::new(manifest_data))
            .map_err(|e| CrushError::StorageError(format!("Failed to write manifest to archive: {}", e)))?;

        archive.into_inner()
            .map_err(|e| CrushError::StorageError(format!("Failed to finalize archive: {}", e)))?;

        Ok(())
    }

    /// Export a stored image as a **complete, valid OCI image layout tarball**
    /// that `docker load` / `skopeo` / `podman` can ingest, then `docker run`.
    ///
    /// The previous version only wrote the layer blobs + an `index.json` with a
    /// zero-size, dangling manifest reference (no manifest blob, no config blob)
    /// — so nothing could load it. Here we write every blob the manifest points
    /// at and rebuild the manifest from the stored image so all digests/sizes are
    /// correct regardless of how the image was created (pulled OR locally
    /// crushed). When an image has no config blob we synthesize a valid one
    /// (computing each layer's `diff_id`), which `docker load` requires.
    pub async fn export_oci_tarball(&self, image_id: &str, dest: &Path) -> Result<()> {
        use sha2::{Digest, Sha256};

        fn sha256_hex(data: &[u8]) -> String { hex::encode(Sha256::digest(data)) }

        // Read a blob straight from its content-addressed path. We avoid
        // `Blobs::read_blob` here because its per-digest lock file requires a
        // locks dir that may not exist, and export is a pure read.
        let read_blob = |digest: &str| -> Result<Vec<u8>> {
            std::fs::read(self.blobs.path_for_digest(digest))
                .map_err(|e| CrushError::StorageError(format!("Failed to read blob {}: {}", digest, e)))
        };

        fn put<W: std::io::Write>(tar: &mut tar::Builder<W>, name: &str, data: &[u8]) -> Result<()> {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            tar.append_data(&mut h, name, data)
                .map_err(|e| CrushError::StorageError(e.to_string()))
        }

        let image = match self.db.get_image_by_digest(image_id).await? {
            Some(img) => img,
            None => self.db.get_image_by_tag(image_id).await?
                .ok_or_else(|| CrushError::ImageError(format!("Image not found: {}", image_id)))?,
        };

        let file = std::fs::File::create(dest)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        let mut tar = tar::Builder::new(file);

        // Docker-archive format (`docker save` layout): manifest.json + a config
        // JSON + one *uncompressed* tar per layer. We decompress each stored
        // layer blob so the layer file's sha256 equals its `diff_id` — which is
        // exactly what `docker load` recomputes and matches against the config's
        // `rootfs.diff_ids`. (OCI-layout archives load inconsistently across
        // Docker versions and often fail to materialize the tag; docker-archive
        // is the portable, reliably-runnable path.)
        let mut diff_ids = Vec::new();
        let mut layer_files = Vec::new();
        for (i, layer_digest) in image.layers.iter().enumerate() {
            let raw = read_blob(layer_digest)?;
            let fmt = crate::compress::detect_format(&raw);
            let plain = match fmt {
                crate::compress::CompressionFormat::Uncompressed => raw,
                _ => crate::compress::decompress_stream(&raw[..], fmt)?,
            };
            diff_ids.push(format!("sha256:{}", sha256_hex(&plain)));
            let name = format!("layer_{}.tar", i);
            put(&mut tar, &name, &plain)?;
            layer_files.push(name);
        }

        // Config: start from the stored OCI/Docker config when present (keeps the
        // real Env/Cmd/Entrypoint/WorkingDir), else synthesize a minimal one;
        // then force `rootfs.diff_ids` to the layers we actually wrote.
        let mut cfg: serde_json::Value = image.config_digest.as_ref()
            .filter(|c| self.blobs.contains(c))
            .and_then(|cd| read_blob(cd).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| serde_json::json!({
                "architecture": image.architecture,
                "os": image.os,
                "config": {
                    "Env": image.env,
                    "Entrypoint": image.entrypoint,
                    "Cmd": image.cmd,
                },
            }));
        cfg["rootfs"] = serde_json::json!({ "type": "layers", "diff_ids": diff_ids });
        if cfg.get("architecture").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            cfg["architecture"] = serde_json::json!(image.architecture);
        }
        if cfg.get("os").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            cfg["os"] = serde_json::json!(image.os);
        }
        let config_bytes = serde_json::to_vec(&cfg)
            .map_err(|e| CrushError::ImageError(e.to_string()))?;
        let config_name = format!("{}.json", sha256_hex(&config_bytes));
        put(&mut tar, &config_name, &config_bytes)?;

        // manifest.json — the entry `docker load` reads to tag + assemble.
        let repo_tags: Vec<String> = if image.tag.is_empty() {
            Vec::new()
        } else if image.tag.contains(':') {
            vec![image.tag.clone()]
        } else {
            vec![format!("{}:latest", image.tag)]
        };
        let manifest = serde_json::json!([{
            "Config": config_name,
            "RepoTags": repo_tags,
            "Layers": layer_files,
        }]);
        let manifest_bytes = serde_json::to_vec(&manifest)
            .map_err(|e| CrushError::ImageError(e.to_string()))?;
        put(&mut tar, "manifest.json", &manifest_bytes)?;

        tar.finish().map_err(|e| CrushError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Assemble and register a **real, runnable OCI image** from a natively-built
    /// project: pull `base_ref`, lay the project tree on top as one app layer
    /// (Crush's native-first model — build on the host, then package the result,
    /// `node_modules`/`dist` included), write a proper image config, and store it
    /// via `put_image` so it shows up in `crush images`, can be run, and exports
    /// to a `docker load`-able tarball.
    ///
    /// This replaces the old behaviour where a "crush"ed project only wrote a
    /// zstd tar of the source into the build cache and never registered an image.
    pub async fn commit_app_image(
        &self,
        tag: &str,
        base_ref: &str,
        project_root: &Path,
        workdir: &str,
        cmd: Vec<String>,
        extra_env: Vec<String>,
    ) -> Result<Image> {
        use sha2::{Digest, Sha256};
        fn sha256_hex(d: &[u8]) -> String { hex::encode(Sha256::digest(d)) }

        // 1. Base image (pull if we don't already have it).
        let base = match self.db.get_image_by_tag(base_ref).await? {
            Some(b) => b,
            None => self.pull_image(base_ref).await?,
        };

        let base_cfg: serde_json::Value = base.config_digest.as_ref()
            .and_then(|c| std::fs::read(self.blobs.path_for_digest(c)).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let mut diff_ids: Vec<String> = base_cfg["rootfs"]["diff_ids"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // 2. App layer: tar the project tree under `workdir` (skip VCS/build noise
        //    that must never ship, but keep node_modules/dist so the image runs).
        let prefix = workdir.trim_start_matches('/').trim_end_matches('/');
        let mut builder = tar::Builder::new(Vec::new());
        fn add_dir(b: &mut tar::Builder<Vec<u8>>, root: &Path, dir: &Path, prefix: &str) -> std::io::Result<()> {
            const SKIP: &[&str] = &[".git", ".hg", ".svn", "target", ".crush", ".cache", ".DS_Store"];
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if SKIP.iter().any(|s| *s == name) { continue; }
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let tar_name = format!("{}/{}", prefix, rel.to_string_lossy());
                let ft = entry.file_type()?;
                if ft.is_dir() {
                    add_dir(b, root, &path, prefix)?;
                } else if ft.is_file() {
                    b.append_path_with_name(&path, &tar_name)?;
                } else if ft.is_symlink() {
                    // best-effort: dereference; skip if dangling
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if meta.is_file() { let _ = b.append_path_with_name(&path, &tar_name); }
                    }
                }
            }
            Ok(())
        }
        add_dir(&mut builder, project_root, project_root, prefix)
            .map_err(|e| CrushError::StorageError(format!("packing app layer: {}", e)))?;
        let raw_tar = builder.into_inner()
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        diff_ids.push(format!("sha256:{}", sha256_hex(&raw_tar)));

        // gzip the layer (OCI/Docker-standard layer compression).
        let gz = {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            std::io::Write::write_all(&mut enc, &raw_tar)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            enc.finish().map_err(|e| CrushError::StorageError(e.to_string()))?
        };
        let app_layer_digest = self.blobs.atomic_write(&gz)?;

        // 3. Image config: inherit the base env, then layer the app on top.
        let mut env: Vec<String> = base_cfg["config"]["Env"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        for e in extra_env { if !env.iter().any(|x| x == &e) { env.push(e); } }
        let arch = if base.architecture.is_empty() { "amd64".to_string() } else { base.architecture.clone() };
        let os = if base.os.is_empty() { "linux".to_string() } else { base.os.clone() };
        let cfg = serde_json::json!({
            "architecture": arch,
            "os": os,
            "config": { "Env": env, "Cmd": cmd, "WorkingDir": workdir },
            "rootfs": { "type": "layers", "diff_ids": diff_ids },
        });
        let cfg_bytes = serde_json::to_vec(&cfg)
            .map_err(|e| CrushError::ImageError(e.to_string()))?;
        let config_digest = self.blobs.atomic_write(&cfg_bytes)?;

        // 4. Register the image (base layers + app layer).
        let mut layers = base.layers.clone();
        layers.push(app_layer_digest);
        let size_bytes: u64 = layers.iter()
            .filter_map(|d| std::fs::metadata(self.blobs.path_for_digest(d)).ok().map(|m| m.len()))
            .sum();
        let id = format!("sha256:{}", sha256_hex(&cfg_bytes));
        let image = Image {
            id: id.clone(),
            tag: tag.to_string(),
            digest: id,
            size_bytes,
            layers,
            architecture: arch,
            os,
            os_version: base.os_version.clone(),
            entrypoint: Vec::new(),
            cmd: image_cmd_from(&cfg),
            env,
            config_digest: Some(config_digest),
        };
        self.db.put_image(&image).await?;
        Ok(image)
    }
}

fn image_cmd_from(cfg: &serde_json::Value) -> Vec<String> {
    cfg["config"]["Cmd"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

#[async_trait]
impl StorageBackend for ImageStore {
    async fn pull_image(&self, tag: &str) -> Result<Image> {
        let (registry, image, reference) = Self::registry_for_tag(tag);

        // Fetch manifests, then release the lock before store_image_from_manifest,
        // which also needs to acquire the registry client lock for blob downloads.
        let (final_manifest, digest) = {
            let client = self.registry_client.lock().await;
            let manifest_json = client.fetch_manifest(&registry, &image, &reference).await?;

            if manifest_json.get("manifests").is_some() {
                let entry = if let Some(p_str) = std::env::var("CRUSH_DEFAULT_PLATFORM").ok().and_then(|s| multiarch::Platform::parse(&s)) {
                    multiarch::MultiArchResolver::resolve_manifest_with_platform(&manifest_json, &p_str)?
                } else {
                    multiarch::MultiArchResolver::resolve_manifest(&manifest_json)?
                };
                let plat_digest = entry["digest"].as_str().unwrap_or("").to_string();
                let plat_manifest = client.fetch_manifest(&registry, &image, &plat_digest).await?;
                (plat_manifest, plat_digest)
            } else {
                let digest = manifest_json["config"]["digest"].as_str().unwrap_or("").to_string();
                (manifest_json, digest)
            }
        };

        let image = self.store_image_from_manifest(&registry, &image, tag, &final_manifest, &digest).await?;

        Ok(image)
    }

    async fn push_image(&self, image_id: &str, registry: &str) -> Result<()> {
        let image = match self.db.get_image_by_digest(image_id).await? {
            Some(img) => img,
            None => self.db.get_image_by_tag(image_id).await?
                .ok_or_else(|| CrushError::ContainerNotFound(format!("Image {} not found", image_id)))?,
        };

        let client = self.registry_client.lock().await;
        let (reg, img, _) = Self::registry_for_tag(registry);

        for layer_digest in &image.layers {
            if self.blobs.contains(layer_digest) {
                let blob_data = self.blobs.read_blob(layer_digest)?;
                let tmp = std::env::temp_dir().join(format!("crush_push_{}", hex::encode(&blob_data[..8.min(blob_data.len())])));
                std::fs::write(&tmp, &blob_data).ok();
                client.upload_blob(&reg, &img, &tmp).await?;
                let _ = std::fs::remove_file(&tmp);
            }
        }

        let manifest_path = self.base_dir.join("manifests").join(&image.id);
        if manifest_path.exists() {
            let manifest_str = tokio::fs::read_to_string(&manifest_path).await
                .map_err(|e| CrushError::StorageError(format!("Failed to read manifest: {}", e)))?;
            client.put_manifest(&reg, &img, &image.tag, &manifest_str).await?;
        }

        Ok(())
    }

    async fn list_images(&self) -> Result<Vec<Image>> {
        self.db.list_images().await
    }

    async fn delete_image(&self, image_id: &str) -> Result<()> {
        self.db.delete_image(image_id).await
    }

    async fn extract_layers(&self, image_id: &str, destination: &PathBuf) -> Result<()> {
        #[cfg(not(target_os = "windows"))]
        fn build_inode_map_from_tar(raw: &[u8]) -> Result<std::collections::HashMap<u64, fuse::InodeMetadata>> {
            use std::collections::HashMap;
            use fuser::FileType;

            let mut inodes: HashMap<u64, fuse::InodeMetadata> = HashMap::new();
            let mut name_to_ino: HashMap<String, u64> = HashMap::new();
            let mut ino_counter = 2u64;

            inodes.insert(1, fuse::InodeMetadata {
                ino: 1, name: "/".to_string(), kind: FileType::Directory,
                size: 0, offset_in_layer: 0, children: vec![],
            });
            name_to_ino.insert("/".to_string(), 1);
            name_to_ino.insert(String::new(), 1);

            let cursor = std::io::Cursor::new(raw);
            let mut archive = tar::Archive::new(cursor);
            let entries = archive.entries()
                .map_err(|e| CrushError::ImageError(format!("Tar entries: {}", e)))?;

            for entry in entries {
                let mut entry = entry
                    .map_err(|e| CrushError::ImageError(format!("Tar entry: {}", e)))?;
                let raw_path = entry.path()
                    .map_err(|e| CrushError::ImageError(format!("Entry path: {}", e)))?
                    .into_owned();
                let clean = raw_path.to_string_lossy();
                let clean = clean.trim_start_matches("./");
                if clean.is_empty() || clean == "/" { continue; }

                let kind = if entry.header().entry_type().is_dir() {
                    FileType::Directory
                } else {
                    FileType::RegularFile
                };
                let size = entry.header().size().unwrap_or(0);
                let offset_in_layer = entry.raw_file_position();

                let fname = std::path::Path::new(clean)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if fname.is_empty() { continue; }

                let parent_str = std::path::Path::new(clean)
                    .parent()
                    .map(|p| {
                        let s = p.to_string_lossy();
                        let s = s.trim_start_matches("./");
                        if s.is_empty() { "/".to_string() } else { format!("/{}", s) }
                    })
                    .unwrap_or_else(|| "/".to_string());

                let parent_ino = *name_to_ino.get(&parent_str).unwrap_or(&1);
                let this_ino = ino_counter;
                ino_counter += 1;

                let full = if parent_str == "/" {
                    format!("/{}", fname)
                } else {
                    format!("{}/{}", parent_str, fname)
                };
                name_to_ino.insert(full, this_ino);

                inodes.insert(this_ino, fuse::InodeMetadata {
                    ino: this_ino, name: fname, kind, size, offset_in_layer, children: vec![],
                });
                if let Some(p) = inodes.get_mut(&parent_ino) {
                    p.children.push(this_ino);
                }
            }

            Ok(inodes)
        }

        let image = match self.db.get_image_by_digest(image_id).await? {
            Some(img) => img,
            None => self.db.get_image_by_tag(image_id).await?
                .ok_or_else(|| CrushError::ContainerNotFound(format!("Image {} not found", image_id)))?,
        };

        tokio::fs::create_dir_all(destination).await
            .map_err(|e| CrushError::StorageError(format!("Failed to create destination: {}", e)))?;

        for layer_digest in &image.layers {
            if !self.blobs.contains(layer_digest) {
                continue;
            }
            let blob_file = self.blobs.read_blob_stream(layer_digest)?;
            // ⚠ FIX: Actually read blob bytes for format detection, not zero-initialized
            let mut header = [0u8; 2];
            let format = if (&blob_file).read_exact(&mut header).is_ok() {
                compress::detect_format(&header)
            } else {
                CompressionFormat::Gzip
            };

            // Windows: always eager extraction (no FUSE support)
            #[cfg(target_os = "windows")]
            {
                let blob_file = self.blobs.read_blob_stream(layer_digest)?;
                let extractor = LayerExtractor::new(destination);
                extractor.extract_layer_streamed(blob_file, format)?;
                continue;
            }

            // Linux/macOS: FUSE lazy loading or eager extraction
            #[cfg(not(target_os = "windows"))]
            {
                let blob_file = self.blobs.read_blob_stream(layer_digest)?;
                if self.lazy_mode {
                    let (reg, img, _) = Self::registry_for_tag(&image.tag);
                    let mut loader = lazy::LazyLoader::new(destination.clone(), reg, img);

                    let mut raw: Vec<u8> = Vec::new();
                    match format {
                        CompressionFormat::Gzip => {
                            let mut dec = flate2::read::GzDecoder::new(blob_file);
                            dec.read_to_end(&mut raw)
                                .map_err(|e| CrushError::ImageError(format!("Gzip decompress: {}", e)))?;
                        }
                        CompressionFormat::Zstd => {
                            let mut dec = zstd::Decoder::new(blob_file)
                                .map_err(|e| CrushError::ImageError(format!("Zstd decoder: {}", e)))?;
                            dec.read_to_end(&mut raw)
                                .map_err(|e| CrushError::ImageError(format!("Zstd decompress: {}", e)))?;
                        }
                        _ => {
                            let mut r: Box<dyn Read> = Box::new(blob_file);
                            r.read_to_end(&mut raw)
                                .map_err(|e| CrushError::ImageError(format!("Raw read: {}", e)))?;
                        }
                    }

                    loader.load_from_blob(&raw, layer_digest)?;

                    let inodes = build_inode_map_from_tar(&raw)?;
                    let loader = Arc::new(loader);
                    let dest = destination.clone();
                    let fs = fuse::LazyImageFs::new(loader, inodes);
                    std::thread::spawn(move || {
                        let options = vec![
                            fuser::MountOption::RO,
                            fuser::MountOption::AllowOther,
                            fuser::MountOption::FSName("crush-lazyfs".to_string()),
                        ];
                        let _ = fuser::mount2(fs, dest, &options);
                    });
                    continue;
                }

                let extractor = LayerExtractor::new(destination);
                extractor.extract_layer_streamed(blob_file, format)?;
            }
        }

        Ok(())
    }
}

impl ImageStore {
    async fn store_image_from_manifest(
        &self, registry: &str, image_name: &str, tag: &str,
        manifest: &serde_json::Value, digest: &str,
    ) -> Result<Image> {
        let layers: Vec<String> = manifest["layers"].as_array()
            .map(|a| a.iter().filter_map(|l| l["digest"].as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let config_digest = manifest["config"]["digest"].as_str().unwrap_or("").to_string();

        if !self.blobs.contains(digest) {
            let manifest_str = serde_json::to_string(manifest)
                .map_err(|e| CrushError::ImageError(format!("Serialization error: {}", e)))?;
            self.blobs.atomic_write(manifest_str.as_bytes())?;
        }

        // Fetch config blob and parse OCI image config for entrypoint/cmd/env
        // and the platform fields (architecture/os/os.version). These are read
        // dynamically so Windows images (os: "windows", with an os.version that
        // must match the host kernel build) are stored correctly rather than
        // being forced to linux/amd64.
        let mut entrypoint = Vec::new();
        let mut cmd_vec = Vec::new();
        let mut env_vec = Vec::new();
        let mut architecture = "amd64".to_string();
        let mut os = "linux".to_string();
        let mut os_version: Option<String> = None;

        if !config_digest.is_empty() {
            let config_data = if !self.blobs.contains(&config_digest) {
                let client = self.registry_client.lock().await;
                match client.fetch_blob(registry, image_name, &config_digest).await {
                    Ok(data) => { self.blobs.atomic_write(&data).ok(); Some(data) }
                    Err(_) => None,
                }
            } else {
                self.blobs.read_blob(&config_digest).ok()
            };

            if let Some(data) = config_data {
                if let Ok(cfg) = serde_json::from_slice::<serde_json::Value>(&data) {
                    if let Some(arr) = cfg["config"]["Entrypoint"].as_array() {
                        entrypoint = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(arr) = cfg["config"]["Cmd"].as_array() {
                        cmd_vec = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(arr) = cfg["config"]["Env"].as_array() {
                        env_vec = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
                    }
                    if let Some(a) = cfg["architecture"].as_str() {
                        architecture = a.to_string();
                    }
                    if let Some(o) = cfg["os"].as_str() {
                        os = o.to_string();
                    }
                    // os.version is present on Windows images (e.g. "10.0.20348.x");
                    // absent on Linux, where it stays None.
                    os_version = cfg["os.version"].as_str().map(String::from);
                }
            }
        }

        // Download layer blobs, store as-is (OCI blobs are content-addressed by their sha256,
        // so atomic_write returns the OCI digest unchanged). No re-compression — that would
        // create a double-compressed zstd(gzip(tar)) that extract_layers can't handle.
        let mut stored_digests = Vec::new();
        for layer_digest in &layers {
            if self.blobs.contains(layer_digest) {
                stored_digests.push(layer_digest.clone());
                continue;
            }

            let client = self.registry_client.lock().await;
            let blob_data = client.fetch_blob(registry, image_name, layer_digest).await?;
            drop(client);

            let stored = self.blobs.atomic_write(&blob_data)?;
            stored_digests.push(stored);
        }

        let total_size: u64 = stored_digests.iter()
            .filter_map(|d| std::fs::metadata(self.blobs.path_for_digest(d)).ok().map(|m| m.len()))
            .sum();

        let image_id = if digest.is_empty() {
            stored_digests.first().cloned()
                .unwrap_or_else(|| format!("sha256:{}", hex::encode(tag.as_bytes())))
        } else {
            digest.to_string()
        };

        let image = Image {
            id: image_id.clone(),
            tag: tag.to_string(),
            digest: image_id,
            size_bytes: total_size,
            layers: stored_digests,
            architecture,
            os,
            os_version,
            entrypoint,
            cmd: cmd_vec,
            env: env_vec,
            config_digest: if config_digest.is_empty() { None } else { Some(config_digest) },
        };

        self.db.put_image(&image).await?;

        Ok(image)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
}

pub async fn extract_packages(_image_id: &str, rootfs: &Path) -> Vec<Package> {
    let mut packages = Vec::new();
    packages.extend(extract_dpkg(rootfs));
    packages.extend(extract_apk(rootfs));
    packages
}

fn extract_dpkg(rootfs: &Path) -> Vec<Package> {
    let mut packages = Vec::new();
    let status_path = rootfs.join("var/lib/dpkg/status");
    if let Ok(content) = std::fs::read_to_string(&status_path) {
        let mut current_name = None;
        let mut current_version = None;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                    packages.push(Package {
                        name,
                        version,
                        ecosystem: "Debian".to_string(),
                    });
                }
            } else if trimmed.starts_with("Package:") {
                current_name = Some(trimmed["Package:".len()..].trim().to_string());
            } else if trimmed.starts_with("Version:") {
                current_version = Some(trimmed["Version:".len()..].trim().to_string());
            }
        }
        if let (Some(name), Some(version)) = (current_name, current_version) {
            packages.push(Package {
                name,
                version,
                ecosystem: "Debian".to_string(),
            });
        }
    }
    packages
}

fn extract_apk(rootfs: &Path) -> Vec<Package> {
    let mut packages = Vec::new();
    let paths = [
        rootfs.join("lib/apk/db/installed"),
        rootfs.join("var/lib/apk/db/installed"),
    ];
    for status_path in &paths {
        if let Ok(content) = std::fs::read_to_string(status_path) {
            let mut current_name = None;
            let mut current_version = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                        packages.push(Package {
                            name,
                            version,
                            ecosystem: "Alpine".to_string(),
                        });
                    }
                } else if let Some(rest) = trimmed.strip_prefix("P:") {
                    current_name = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("V:") {
                    current_version = Some(rest.trim().to_string());
                }
            }
            if let (Some(name), Some(version)) = (current_name, current_version) {
                packages.push(Package {
                    name,
                    version,
                    ecosystem: "Alpine".to_string(),
                });
            }
            break;
        }
    }
    packages
}

#[cfg(test)]
mod tests {
    use super::ImageStore;

    // registry_for_tag(tag) → (registry, image, reference)

    #[test]
    fn bare_image_defaults_to_dockerhub_library() {
        let (reg, img, reference) = ImageStore::registry_for_tag("ubuntu");
        assert_eq!(reg, "registry-1.docker.io");
        assert_eq!(img, "library/ubuntu");
        assert_eq!(reference, "latest");
    }

    #[test]
    fn image_with_tag() {
        let (reg, img, reference) = ImageStore::registry_for_tag("ubuntu:22.04");
        assert_eq!(reg, "registry-1.docker.io");
        assert_eq!(img, "library/ubuntu");
        assert_eq!(reference, "22.04");
    }

    #[test]
    fn namespaced_image_on_dockerhub() {
        let (reg, img, reference) = ImageStore::registry_for_tag("nginx/nginx-prometheus-exporter:latest");
        assert_eq!(reg, "registry-1.docker.io");
        assert_eq!(img, "nginx/nginx-prometheus-exporter");
        assert_eq!(reference, "latest");
    }

    #[test]
    fn explicit_dockerhub_prefix_stripped() {
        let (reg, img, reference) = ImageStore::registry_for_tag("docker://alpine:3.19");
        assert_eq!(reg, "registry-1.docker.io");
        assert_eq!(img, "library/alpine");
        assert_eq!(reference, "3.19");
    }

    #[test]
    fn ghcr_registry_detected() {
        let (reg, img, reference) = ImageStore::registry_for_tag("ghcr.io/owner/repo:v1.0");
        assert_eq!(reg, "ghcr.io");
        assert_eq!(img, "owner/repo");
        assert_eq!(reference, "v1.0");
    }

    #[test]
    fn private_registry_with_port() {
        // localhost:5000/myapp:dev — registry detected via ':' in host segment.
        // Single-segment image paths always get the "library/" prefix in the current
        // implementation, even for non-Docker-Hub registries.
        let (reg, img, reference) = ImageStore::registry_for_tag("localhost:5000/myapp:dev");
        assert_eq!(reg, "localhost:5000");
        assert_eq!(img, "library/myapp");
        assert_eq!(reference, "dev");
    }

    #[test]
    fn image_no_tag_defaults_latest() {
        let (_, _, reference) = ImageStore::registry_for_tag("nginx");
        assert_eq!(reference, "latest");
    }

    #[test]
    fn tag_with_multiple_colons_splits_on_last() {
        // registry_for_tag uses rfind(':'), so "nginx:1.25.3" splits correctly.
        // OCI digest refs (@sha256:...) are not yet supported by this function.
        let (reg, img, reference) = ImageStore::registry_for_tag("nginx:1.25.3");
        assert_eq!(reg, "registry-1.docker.io");
        assert_eq!(img, "library/nginx");
        assert_eq!(reference, "1.25.3");
    }
}
