use std::path::Path;
use wasmtime_wasi::{WasiCtxBuilder, WasiCtx, DirPerms, FilePerms};
use crush_types::{Result, CrushError, MountConfig};

pub struct WasiContext;

impl WasiContext {
    pub fn build(
        args: &[String],
        env_vars: &[(String, String)],
        mounts: &[MountConfig],
        preopen_dir: Option<&Path>,
        _memory_limit_mb: u64,
    ) -> Result<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();

        builder.inherit_stdin().inherit_stdout().inherit_stderr();
        builder.args(args);

        for (k, v) in env_vars {
            builder.env(k, v);
        }

        for mount in mounts {
            if mount.host_path.exists() {
                let dir_perms = if mount.read_only {
                    DirPerms::READ
                } else {
                    DirPerms::READ | DirPerms::MUTATE
                };
                builder.preopened_dir(
                    &mount.host_path,
                    mount.container_path.to_string_lossy().as_ref(),
                    dir_perms,
                    FilePerms::all(),
                ).map_err(|e| CrushError::WasmError(format!("preopened_dir failed: {}", e)))?;
            }
        }

        if let Some(dir_path) = preopen_dir {
            builder.preopened_dir(
                dir_path,
                "/",
                DirPerms::READ | DirPerms::MUTATE,
                FilePerms::all(),
            ).map_err(|e| CrushError::WasmError(format!("preopened_dir failed: {}", e)))?;
        }

        Ok(builder.build())
    }
}
