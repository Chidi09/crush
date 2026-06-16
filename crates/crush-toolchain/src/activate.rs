use crate::{SupportedRuntime, fetch_toolchain};
use anyhow::Result;
use std::path::PathBuf;

pub async fn activate_toolchain(runtime: SupportedRuntime, version: &str) -> Result<PathBuf> {
    let dir = fetch_toolchain(runtime, version).await?;
    let bin_dir = dir.join("bin");
    Ok(bin_dir)
}
