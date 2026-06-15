//! Lockfile-content-hash based freshness tracking (R1.3).
//!
//! Persists `{ lockfile-relpath: { sha256, installed_at } }` in
//! `.crush/deps-state.json` under the workspace root. Freshness = sha256
//! of the current lockfile matches the stored sha256 AND the deps directory
//! exists. The old mtime comparison is kept only as a fast-path *negative*
//! (if the deps dir is missing → not fresh); it is never used as a positive
//! freshness signal.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockfileState {
    pub sha256: String,
    pub installed_at: u64,
}

pub type DepsState = HashMap<String, LockfileState>;

/// Walk upwards from `start` to find a directory containing `.crush/` or `.git/`.
pub fn find_workspace_root(start: &Path) -> PathBuf {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join(".crush").is_dir() || cur.join(".git").is_dir() {
            return cur;
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => return start.to_path_buf(),
        }
    }
}

fn deps_state_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".crush").join("deps-state.json")
}

/// Load the persisted deps state. Returns empty map on any error.
pub fn load_deps_state(workspace_root: &Path) -> DepsState {
    fs::read_to_string(deps_state_path(workspace_root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Persist the deps state atomically (write temp file + rename).
pub fn save_deps_state(workspace_root: &Path, state: &DepsState) -> std::io::Result<()> {
    let path = deps_state_path(workspace_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// SHA-256 of the file at `path`. Returns `None` on read error.
pub fn file_sha256(path: &Path) -> Option<String> {
    use sha2::{Sha256, Digest};
    let data = fs::read(path).ok()?;
    let mut h = Sha256::new();
    h.update(&data);
    Some(hex::encode(h.finalize()))
}

/// All recognised lockfile names we track.
pub const LOCKFILE_NAMES: &[&str] = &[
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "poetry.lock",
    "requirements.txt",
    "uv.lock",
    "Cargo.lock",
    "go.sum",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "composer.lock",
    "mix.lock",
];

/// Return all lockfiles that actually exist inside `dir`.
pub fn lockfiles_in(dir: &Path) -> Vec<PathBuf> {
    LOCKFILE_NAMES.iter()
        .map(|n| dir.join(n))
        .filter(|p| p.is_file())
        .collect()
}

/// Determine whether the installed dependencies for `dir` are fresh.
///
/// **Fresh** means:
///   1. The language-appropriate deps directory exists (fast-path negative).
///   2. At least one lockfile in `dir` has an entry in `.crush/deps-state.json`
///      whose stored SHA-256 matches the *current* file contents.
pub fn check_deps_fresh(dir: &Path, language: &str) -> bool {
    let lang = language.split(' ').next().unwrap_or("").to_lowercase();

    // Fast-path negative: if the deps directory doesn't exist, never fresh.
    let deps_dir_missing = match lang.as_str() {
        "node" | "typescript" | "bun" | "deno" => !dir.join("node_modules").exists(),
        "python" => !dir.join(".venv").exists() && !dir.join("venv").exists(),
        "php" => !dir.join("vendor").exists(),
        "elixir" => !dir.join("deps").exists(),
        "rust" => !dir.join("target").exists(),
        _ => false, // no specific dir to check; proceed to hash check
    };
    if deps_dir_missing {
        return false;
    }

    let locks = lockfiles_in(dir);
    if locks.is_empty() {
        return false;
    }

    let root = find_workspace_root(dir);
    let state = load_deps_state(&root);

    for lock in &locks {
        let rel = lock.strip_prefix(&root).unwrap_or(lock);
        let rel_str = rel.to_string_lossy().replace('\\', "/"); // normalise on Windows
        if let Some(stored) = state.get(rel_str.as_str()) {
            if let Some(cur) = file_sha256(lock) {
                if stored.sha256 == cur {
                    return true; // found a matching lockfile → fresh
                }
            }
        }
    }
    false
}

/// Record the SHA-256 of every lockfile in `dir` into the workspace deps-state.
/// Call this immediately after a successful `npm install` / `pip install` / etc.
pub fn record_install(dir: &Path) {
    let root = find_workspace_root(dir);
    let mut state = load_deps_state(&root);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut dirty = false;
    for lock in lockfiles_in(dir) {
        if let Some(sha) = file_sha256(&lock) {
            let rel = lock.strip_prefix(&root).unwrap_or(&lock);
            let key = rel.to_string_lossy().replace('\\', "/");
            state.insert(key, LockfileState { sha256: sha, installed_at: now });
            dirty = true;
        }
    }
    if dirty {
        let _ = save_deps_state(&root, &state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn git_root(dir: &Path) {
        fs::create_dir_all(dir.join(".git")).unwrap();
    }

    #[test]
    fn fresh_after_record_then_not_fresh_on_change() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        git_root(root);

        let lock = root.join("package-lock.json");
        fs::write(&lock, b"v1-content").unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();

        // Before recording — not fresh.
        assert!(!check_deps_fresh(root, "node"));

        record_install(root);
        // After recording — fresh.
        assert!(check_deps_fresh(root, "node"));

        // Mutate the lockfile → stale.
        fs::write(&lock, b"v2-content").unwrap();
        assert!(!check_deps_fresh(root, "node"));
    }

    #[test]
    fn missing_deps_dir_is_not_fresh() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        git_root(root);
        fs::write(root.join("package-lock.json"), b"content").unwrap();
        record_install(root);
        // node_modules is absent → not fresh despite recorded hash.
        assert!(!check_deps_fresh(root, "node"));
    }

    #[test]
    fn file_sha256_is_deterministic() {
        let tmp = TempDir::new().unwrap();
        let f = tmp.path().join("lock");
        fs::write(&f, b"hello").unwrap();
        let h1 = file_sha256(&f).unwrap();
        let h2 = file_sha256(&f).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // hex SHA-256 = 64 chars
    }

    #[test]
    fn mtime_cannot_fake_freshness() {
        // Even if node_modules has a NEWER mtime, we rely only on hash.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        git_root(root);
        let lock = root.join("package-lock.json");
        fs::write(&lock, b"content").unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();

        // Do NOT call record_install → no stored hash.
        // node_modules exists, lockfile exists — but hash is unrecorded.
        // Result must be not-fresh, proving mtime is not being used as positive signal.
        assert!(!check_deps_fresh(root, "node"));
    }
}
