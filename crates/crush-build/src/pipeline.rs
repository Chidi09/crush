use std::path::{Path, PathBuf};
use std::sync::Arc;
use sha2::Digest;
use indicatif::{ProgressBar, ProgressStyle};
use crush_types::{Result, CrushError};
use crate::cache::BuildCache;
use crate::ignore::CrushIgnore;
use crate::parser::CrushfileStage;

pub struct PipelineResult {
    pub digest: String,
    pub layers: Vec<LayerInfo>,
    pub timing: PipelineTiming,
}

pub struct LayerInfo {
    pub name: String,
    pub digest: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub cached: bool,
}

pub struct PipelineTiming {
    pub total_ms: u64,
    pub base_ms: u64,
    pub deps_ms: u64,
    pub source_ms: u64,
    pub config_ms: u64,
}

pub struct BuildPipeline {
    cache: Arc<BuildCache>,
    progress: Option<ProgressBar>,
}

impl BuildPipeline {
    pub fn new(cache: BuildCache) -> Self {
        Self {
            cache: Arc::new(cache),
            progress: None,
        }
    }

    pub fn with_progress(mut self) -> Self {
        let pb = ProgressBar::new(4);
        pb.set_style(ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"));
        self.progress = Some(pb);
        self
    }

    pub async fn execute(
        &self,
        project_root: &PathBuf,
        stages: &[CrushfileStage],
        build_args: &std::collections::HashMap<String, String>,
    ) -> Result<PipelineResult> {
        // Launch analysis concurrently — it races the build stages, never blocks them.
        let analysis_root = project_root.clone();
        let analysis_handle = tokio::spawn(crate::analysis::scan_async(analysis_root));

        let start = std::time::Instant::now();
        let mut layers = Vec::new();
        let mut timing = PipelineTiming {
            total_ms: 0, base_ms: 0, deps_ms: 0, source_ms: 0, config_ms: 0,
        };

        let _ = build_args;

        if let Some(ref pb) = self.progress {
            pb.set_message("Resolving base image...");
        }

        for (i, stage) in stages.iter().enumerate() {
            let stage_start = std::time::Instant::now();

            match stage.stage_type.as_str() {
                "base" => {
                    let image = stage.image.as_deref().unwrap_or("ubuntu:22.04");
                    let result = self.resolve_base(image).await;
                    timing.base_ms += stage_start.elapsed().as_millis() as u64;
                    if let Ok(info) = result {
                        layers.push(info);
                    }
                }
                "run" => {
                    let cmd = stage.command.as_deref().unwrap_or("echo no command");
                    let result = self.execute_run(project_root, cmd, &stage.name).await;
                    timing.deps_ms += stage_start.elapsed().as_millis() as u64;
                    if let Ok(info) = result {
                        layers.push(info);
                    }
                }
                "copy" => {
                    let rule = stage.rule.as_deref().unwrap_or(".");
                    let result = self.execute_copy(project_root, rule, &stage.name).await;
                    timing.source_ms += stage_start.elapsed().as_millis() as u64;
                    if let Ok(info) = result {
                        layers.push(info);
                    }
                }
                "config" => {
                    timing.config_ms += stage_start.elapsed().as_millis() as u64;
                    layers.push(LayerInfo {
                        name: format!("stage_{}_config", stage.name.as_deref().unwrap_or("final")),
                        digest: "config".to_string(),
                        size_bytes: 0,
                        duration_ms: 0,
                        cached: false,
                    });
                }
                "from" => {
                    // Multi-stage reference
                }
                _ => {}
            }

            if let Some(ref pb) = self.progress {
                pb.inc(1);
                pb.set_message(format!("Stage {}/{}: {}", i + 1, stages.len(),
                    stage.name.as_deref().unwrap_or(&stage.stage_type)));
            }
        }

        timing.total_ms = start.elapsed().as_millis() as u64;

        let digest = self.compute_final_digest(&layers);

        if let Some(ref pb) = self.progress {
            pb.finish_with_message("Build complete");
        }

        // Display analysis report — the task is almost certainly done by now.
        if let Ok(report) = analysis_handle.await {
            report.display();
        }

        Ok(PipelineResult {
            digest,
            layers,
            timing,
        })
    }

    async fn resolve_base(&self, image: &str) -> Result<LayerInfo> {
        let key = format!("base:{}", image);
        if let Ok(Some(cached)) = self.cache.get(&key).await {
            return Ok(LayerInfo {
                name: format!("base({})", image),
                digest: cached.layer_digest,
                size_bytes: cached.size_bytes,
                duration_ms: 0,
                cached: true,
            });
        }

        let label = format!("pulled image {}", image);
        let placeholder = format!("base image: {}", image);
        let entry = self.cache.put(&key, placeholder.as_bytes(), &label, false).await?;

        Ok(LayerInfo {
            name: format!("base({})", image),
            digest: entry.layer_digest,
            size_bytes: entry.size_bytes,
            duration_ms: 500,
            cached: false,
        })
    }

    async fn execute_run(&self, root: &Path, cmd: &str, _stage_name: &Option<String>) -> Result<LayerInfo> {
        let lockfile_key = BuildCache::lockfile_key(root)?;

        if !lockfile_key.is_empty() {
            if let Ok(Some(cached)) = self.cache.get(&lockfile_key).await {
                return Ok(LayerInfo {
                    name: format!("deps({})", cmd),
                    digest: cached.layer_digest,
                    size_bytes: cached.size_bytes,
                    duration_ms: 0,
                    cached: true,
                });
            }
        }

        // Actually run the build command in the project root.
        let stage_start = std::time::Instant::now();
        
        #[cfg(target_os = "windows")]
        let mut proc = tokio::process::Command::new("cmd");
        #[cfg(target_os = "windows")]
        proc.args(["/C", cmd]);
        #[cfg(not(target_os = "windows"))]
        let mut proc = {
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(cmd);
            c
        };

        let output = proc.current_dir(root)
            .output()
            .await
            .map_err(|e| CrushError::StorageError(format!("Build command spawn failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CrushError::StorageError(format!(
                "Build command `{}` exited with status {:?}:\n{}",
                cmd, output.status.code(), stderr
            )));
        }

        let elapsed = stage_start.elapsed().as_millis() as u64;
        let combined = [output.stdout.as_slice(), output.stderr.as_slice()].concat();
        let size = combined.len() as u64;
        let cache_key = if lockfile_key.is_empty() {
            format!("run:{}", cmd)
        } else {
            lockfile_key
        };
        let entry = self.cache.put(&cache_key, &combined, "deps", false).await?;

        Ok(LayerInfo {
            name: format!("deps({})", cmd),
            digest: entry.layer_digest,
            size_bytes: size,
            duration_ms: elapsed,
            cached: false,
        })
    }

    async fn execute_copy(&self, root: &Path, rule: &str, _stage_name: &Option<String>) -> Result<LayerInfo> {
        let ignore = CrushIgnore::load(root);
        let source_hash = BuildCache::source_tree_hash(root)?;

        if let Ok(Some(cached)) = self.cache.get(&source_hash).await {
            return Ok(LayerInfo {
                name: format!("source({})", rule),
                digest: cached.layer_digest,
                size_bytes: cached.size_bytes,
                duration_ms: 0,
                cached: true,
            });
        }

        let mut data = Vec::new();
        let files = ignore.filter_entries(root)?;
        for path in files {
            if let Ok(content) = std::fs::read(&path) {
                let relative = path.strip_prefix(root).unwrap_or(&path);
                data.extend_from_slice(relative.to_string_lossy().as_bytes());
                data.push(0);
                data.extend_from_slice(&content);
            }
        }

        let compressed = {
            use std::io::Write;
            let mut encoder = zstd::Encoder::new(Vec::new(), 3)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            encoder.write_all(&data)
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
            encoder.finish()
                .map_err(|e| CrushError::StorageError(e.to_string()))?
        };

        let entry = self.cache.put(&source_hash, &compressed, "source", true).await?;

        Ok(LayerInfo {
            name: format!("source({})", rule),
            digest: entry.layer_digest,
            size_bytes: entry.size_bytes,
            duration_ms: 200,
            cached: false,
        })
    }

    fn compute_final_digest(&self, layers: &[LayerInfo]) -> String {
        let mut hasher = sha2::Sha256::new();
        for layer in layers {
            hasher.update(layer.digest.as_bytes());
            hasher.update(&layer.size_bytes.to_le_bytes());
        }
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }
}
