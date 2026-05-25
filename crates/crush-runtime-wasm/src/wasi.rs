use std::path::Path;
use std::sync::Arc;
use wasmtime_wasi::{WasiCtxBuilder, WasiCtx, DirPerms, FilePerms};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};
use cap_std::fs::Dir;
use crush_types::{Result, CrushError, MountConfig};

pub struct WasiContext;

impl WasiContext {
    pub fn build(
        args: &[String],
        env_vars: &[(String, String)],
        mounts: &[MountConfig],
        preopen_dir: Option<&Path>,
        memory_limit_mb: u64,
    ) -> Result<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();

        builder = builder.inherit_stdin().inherit_stdout().inherit_stderr();

        builder = builder.args(args);

        for (k, v) in env_vars {
            builder = builder.env(k, v);
        }

        for mount in mounts {
            if mount.host_path.exists() {
                let dir_result = Dir::open_ambient_dir(&mount.host_path, cap_std::ambient_authority());
                if let Ok(dir) = dir_result {
                    let perms = if mount.read_only {
                        DirPerms::READ
                    } else {
                        DirPerms::READ | DirPerms::WRITE | DirPerms::MUTATE_DIRECTORY
                    };
                    builder = builder.preopened_dir(dir, &mount.container_path.to_string_lossy(), perms);
                }
            }
        }

        if let Some(dir_path) = preopen_dir {
            if let Ok(dir) = Dir::open_ambient_dir(dir_path, cap_std::ambient_authority()) {
                builder = builder.preopened_dir(dir, "/", DirPerms::READ | DirPerms::WRITE);
            }
        }

        let ctx = builder.build()
            .map_err(|e| CrushError::WasmError(format!("WASI context build failed: {}", e)))?;

        Ok(ctx)
    }
}
