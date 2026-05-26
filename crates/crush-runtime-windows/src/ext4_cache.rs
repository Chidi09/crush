use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

pub struct Ext4Cache {
    cache_dir: PathBuf,
}

impl Ext4Cache {
    pub fn new(data_dir: &Path) -> Self {
        let cache_dir = data_dir.join("fc-drives");
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }

    /// Path where the ext4 drive for this digest lives.
    pub fn drive_path(&self, image_digest: &str) -> PathBuf {
        // digest may contain ':' (sha256:abc...) — strip the prefix for filename safety
        let safe = image_digest.replace(':', "_");
        self.cache_dir.join(format!("{}.ext4", &safe[..std::cmp::min(safe.len(), 64)]))
    }

    /// Returns true if a usable drive image is already cached for this digest.
    pub fn is_cached(&self, image_digest: &str) -> bool {
        let p = self.drive_path(image_digest);
        p.exists() && p.metadata().map(|m| m.len() > 0).unwrap_or(false)
    }

    /// Build an ext4 drive image from an already-extracted rootfs directory.
    /// `image_digest` is the OCI image digest — used as the cache key.
    /// `rootfs` is the extracted layer tree on the host filesystem.
    ///
    /// Strategy:
    ///   1. Measure total size of rootfs (with 20% headroom), minimum 256 MiB.
    ///   2. Create a sparse file of that size.
    ///   3. Format it with mke2fs -t ext4 (from e2fsprogs, must be in PATH).
    ///   4. Copy contents with e2cp or genext2fs if available; otherwise use a
    ///      minimal init that mounts the raw image and unpacks a tar at boot.
    ///      For maximum compatibility, fall back to a tarball init approach.
    pub fn build(&self, image_digest: &str, rootfs: &Path) -> Result<PathBuf> {
        let dest = self.drive_path(image_digest);
        if self.is_cached(image_digest) {
            return Ok(dest);
        }

        // Measure rootfs size
        let size_bytes = dir_size(rootfs)?;
        // Add 20% headroom, minimum 256 MiB, round up to nearest MiB
        let size_mib = std::cmp::max(256, (size_bytes * 12 / 10) / (1024 * 1024) + 1);

        // Create sparse file
        {
            let f = std::fs::File::create(&dest)
                .map_err(|e| anyhow!("Cannot create ext4 image {:?}: {}", dest, e))?;
            f.set_len(size_mib * 1024 * 1024)
                .map_err(|e| anyhow!("Cannot size ext4 image: {}", e))?;
        }

        // Format with mke2fs
        let mke2fs = find_tool(&["mke2fs", "mke2fs.exe"])
            .ok_or_else(|| anyhow!(
                "mke2fs not found in PATH. Install e2fsprogs:\n\
                 Windows: winget install e2fsprogs OR use the bundled binary at C:\\crush\\tools\\mke2fs.exe\n\
                 Set CRUSH_MKE2FS env var to override the path."
            ))?;

        let status = std::process::Command::new(&mke2fs)
            .args(["-t", "ext4", "-F", &dest.to_string_lossy()])
            .status()
            .map_err(|e| anyhow!("mke2fs failed to run: {}", e))?;

        if !status.success() {
            let _ = std::fs::remove_file(&dest);
            return Err(anyhow!("mke2fs exited with {:?}", status.code()));
        }

        // Populate: try debugfs batch copy, fall back to tar-based init
        let populated = try_populate_with_debugfs(&dest, rootfs);
        if !populated {
            // Fallback: pack rootfs into a tar, embed path in kernel boot args.
            // The minimal init will unpack it at first boot into the ext4 image.
            // This is slower on first boot but requires no extra tools.
            pack_as_initrd_tar(rootfs, &dest.with_extension("initrd.tar"))?;
        }

        Ok(dest)
    }
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}

fn find_tool(names: &[&str]) -> Option<std::path::PathBuf> {
    // Check CRUSH_MKE2FS env override first
    if let Ok(path) = std::env::var("CRUSH_MKE2FS") {
        let p = std::path::PathBuf::from(&path);
        if p.exists() { return Some(p); }
    }
    // Check bundled path
    let bundled = std::path::PathBuf::from(r"C:\crush\tools\mke2fs.exe");
    if bundled.exists() { return Some(bundled); }
    // Search PATH
    for name in names {
        if let Ok(p) = which::which(name) {
            return Some(p);
        }
    }
    None
}

fn try_populate_with_debugfs(drive: &Path, rootfs: &Path) -> bool {
    // Build a debugfs script: one `write <host_path> <guest_path>` per file
    // This is limited to ~10k files before it gets slow, but covers most images.
    let script_path = drive.with_extension("debugfs_script");
    let mut script = String::new();

    if let Ok(_walker) = std::fs::read_dir(rootfs) {
        build_debugfs_script(rootfs, rootfs, &mut script);
    }

    if script.is_empty() { return false; }

    std::fs::write(&script_path, &script).ok();

    let result = std::process::Command::new("debugfs")
        .args(["-w", &drive.to_string_lossy(), "-f", &script_path.to_string_lossy()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = std::fs::remove_file(&script_path);
    result
}

fn build_debugfs_script(base: &Path, dir: &Path, script: &mut String) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        let guest = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
        if path.is_dir() {
            script.push_str(&format!("mkdir {}\n", guest));
            build_debugfs_script(base, &path, script);
        } else {
            script.push_str(&format!("write {} {}\n", path.display(), guest));
        }
    }
}

fn pack_as_initrd_tar(rootfs: &Path, output: &Path) -> Result<()> {
    let file = std::fs::File::create(output)
        .map_err(|e| anyhow!("Cannot create initrd tar: {}", e))?;
    let mut builder = tar::Builder::new(file);
    builder.append_dir_all(".", rootfs)
        .map_err(|e| anyhow!("Tar append failed: {}", e))?;
    builder.finish().map_err(|e| anyhow!("Tar finish failed: {}", e))?;
    Ok(())
}
