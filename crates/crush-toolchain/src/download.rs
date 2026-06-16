use crate::SupportedRuntime;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn cache_dir_for(runtime: SupportedRuntime, version: &str) -> Result<PathBuf> {
    let mut dir = dirs::home_dir().context("No home dir")?;
    dir.push(".crush");
    dir.push("runtimes");
    dir.push(runtime.as_str());
    dir.push(version);
    Ok(dir)
}

pub async fn fetch_toolchain(runtime: SupportedRuntime, version: &str) -> Result<PathBuf> {
    let dir = cache_dir_for(runtime, version)?;
    if dir.exists() {
        return Ok(dir); // cache hit
    }

    // Mock fetching process. In a real implementation:
    // - Node: download from nodejs.org/dist
    // - Python: uv python install
    // - Java: adoptium
    // - Go: go.dev/dl
    
    std::fs::create_dir_all(&dir)?;
    let bin_dir = dir.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    
    Ok(dir)
}
