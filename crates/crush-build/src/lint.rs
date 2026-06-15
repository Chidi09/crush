//! Cross-OS eject linter.
//!
//! Crush runs natively on Windows (case-insensitive NTFS) but `crush eject`
//! targets a Linux container (case-sensitive ext4). The classic "works on my
//! machine" deploy failure is a relative import whose case doesn't match the
//! file on disk: `import './User'` happily resolves to `user.ts` on Windows,
//! then the Linux build can't find the module.
//!
//! This module scans JS/TS source for relative imports and flags any whose
//! spelling differs from the real filename only by case. It also flags files
//! that collide case-insensitively (two files that can't coexist on NTFS).
//! Pure filesystem + string work — no parser, no toolchain.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One cross-OS portability problem found in the source tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFinding {
    /// Source file containing the import (relative to project root).
    pub file: String,
    /// 1-based line number of the import.
    pub line: usize,
    /// The specifier as written, e.g. `./components/User`.
    pub specifier: String,
    /// What it should be to match disk, e.g. `./components/user`.
    pub suggestion: String,
    /// Human-readable explanation.
    pub message: String,
}

const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", "dist", "build", "out", ".next", ".nuxt",
    ".svelte-kit", ".turbo", "target", ".venv", "venv", "__pycache__",
    ".cache", "coverage", ".angular", ".output", "vendor",
];

const SOURCE_EXTS: &[&str] = &["js", "jsx", "ts", "tsx", "mjs", "cjs", "mts", "cts"];

/// Resolution order Node/bundlers try when an import has no extension.
const RESOLVE_EXTS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts", "json", "vue", "svelte"];

/// Scan `root` for cross-OS import-casing problems. Returns findings sorted by
/// file then line.
pub fn lint_cross_os(root: &Path) -> Vec<LintFinding> {
    let mut files = Vec::new();
    collect_sources(root, &mut files, 0);

    let mut findings = Vec::new();
    for file in &files {
        scan_file_imports(root, file, &mut findings);
    }
    findings.extend(case_collisions(root));
    findings.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    findings.dedup();
    findings
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 24 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if name.starts_with('.') && name != "." || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            collect_sources(&path, out, depth + 1);
        } else if ft.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SOURCE_EXTS.contains(&ext) {
                    out.push(path);
                }
            }
        }
    }
}

fn scan_file_imports(root: &Path, file: &Path, findings: &mut Vec<LintFinding>) {
    let Ok(content) = std::fs::read_to_string(file) else { return };
    let dir = file.parent().unwrap_or(root);
    for (i, line) in content.lines().enumerate() {
        for spec in extract_relative_specifiers(line) {
            if let Some(expected) = mismatched_case(dir, &spec) {
                findings.push(LintFinding {
                    file: rel(root, file),
                    line: i + 1,
                    specifier: spec.clone(),
                    suggestion: expected.clone(),
                    message: format!(
                        "import '{spec}' resolves on Windows but not Linux — on disk it is '{expected}' (case differs)"
                    ),
                });
            }
        }
    }
}

/// Pull relative import/require/dynamic-import specifiers out of a line.
/// Deliberately simple: matches `from '...'`, `require('...')`, `import('...')`
/// where the target starts with `./` or `../`.
fn extract_relative_specifiers(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Strip line comments to avoid false hits.
    let code = line.split("//").next().unwrap_or(line);
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\'' || c == '"' || c == '`' {
            // read the quoted string
            let quote = c;
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] as char != quote {
                j += 1;
            }
            if j <= bytes.len() {
                let s = &code[start..j.min(code.len())];
                if s.starts_with("./") || s.starts_with("../") {
                    // Only treat as an import if preceded by from/import/require.
                    let before = code[..i].trim_end();
                    if before.ends_with("from")
                        || before.ends_with("import")
                        || before.ends_with("require(")
                        || before.ends_with("import(")
                    {
                        out.push(s.to_string());
                    }
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// If the specifier resolves to a real file/dir whose on-disk casing differs
/// from how it was written, return the correctly-cased specifier. `None` when
/// it matches exactly, can't be resolved, or matches a directory index.
fn mismatched_case(import_dir: &Path, spec: &str) -> Option<String> {
    // Split off any trailing extension the author wrote so we compare segments.
    let segments: Vec<&str> = spec.split('/').collect();
    let mut cur = import_dir.to_path_buf();
    let mut rebuilt: Vec<String> = Vec::new();
    let mut any_mismatch = false;

    for (idx, seg) in segments.iter().enumerate() {
        match *seg {
            "." => { rebuilt.push(".".into()); continue; }
            ".." => { cur.pop(); rebuilt.push("..".into()); continue; }
            "" => continue,
            _ => {}
        }
        let is_last = idx == segments.len() - 1;
        // Find the matching directory entry, comparing case-insensitively.
        let entries = list_dir(&cur)?;
        if is_last {
            // The last segment may need an extension appended to resolve.
            if let Some((real, mismatch)) = resolve_leaf(&entries, seg) {
                any_mismatch |= mismatch;
                rebuilt.push(real);
                // We don't descend further.
                break;
            } else {
                return None; // unresolved — not our concern (could be a package alias)
            }
        } else {
            let (real, mismatch) = match_entry(&entries, seg)?;
            any_mismatch |= mismatch;
            cur.push(&real);
            rebuilt.push(real);
        }
    }

    if any_mismatch {
        Some(rebuilt.join("/"))
    } else {
        None
    }
}

/// (entry names in `dir`) or None if unreadable.
fn list_dir(dir: &Path) -> Option<Vec<String>> {
    let rd = std::fs::read_dir(dir).ok()?;
    Some(rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
}

/// Match a path segment against directory entries. Returns (real_name, mismatch).
/// `None` when there's no case-insensitive match at all.
fn match_entry(entries: &[String], seg: &str) -> Option<(String, bool)> {
    // Exact first.
    if entries.iter().any(|e| e == seg) {
        return Some((seg.to_string(), false));
    }
    // Case-insensitive.
    let lower = seg.to_lowercase();
    entries
        .iter()
        .find(|e| e.to_lowercase() == lower)
        .map(|e| (e.clone(), true))
}

/// Resolve the final segment, which may be written without an extension and may
/// point at a file or a directory (index file). Returns (corrected_leaf, mismatch).
fn resolve_leaf(entries: &[String], seg: &str) -> Option<(String, bool)> {
    // 1) Direct file/dir match (author wrote the extension, or it's a dir).
    if let Some(hit) = match_entry(entries, seg) {
        return Some(hit);
    }
    // 2) Author omitted the extension: try seg + .<ext>.
    let lower = seg.to_lowercase();
    for ext in RESOLVE_EXTS {
        let want = format!("{lower}.{ext}");
        if let Some(real) = entries.iter().find(|e| e.to_lowercase() == want) {
            // The corrected specifier keeps the author's (extensionless) style
            // but fixes the stem's casing.
            let stem = real.rsplit_once('.').map(|(s, _)| s).unwrap_or(real);
            let mismatch = stem != seg;
            return Some((stem.to_string(), mismatch));
        }
    }
    None
}

/// Files in the same directory that differ only by case — illegal to check out
/// together on a case-insensitive filesystem, and a hazard the other way too.
fn case_collisions(root: &Path) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut seen: HashMap<String, String> = HashMap::new();
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                stack.push(entry.path());
            }
            let key = name.to_lowercase();
            if let Some(prev) = seen.get(&key) {
                if prev != &name {
                    findings.push(LintFinding {
                        file: rel(root, &dir.join(&name)),
                        line: 0,
                        specifier: name.clone(),
                        suggestion: prev.clone(),
                        message: format!(
                            "'{name}' and '{prev}' differ only in case — they collide on case-insensitive filesystems"
                        ),
                    });
                }
            } else {
                seen.insert(key, name);
            }
        }
    }
    findings
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    #[test]
    fn flags_case_mismatched_relative_import() {
        let d = tempfile::TempDir::new().unwrap();
        write(d.path(), "src/user.ts", "export const x = 1;");
        write(d.path(), "src/app.ts", "import { x } from './User';\n");
        let f = lint_cross_os(d.path());
        assert_eq!(f.len(), 1, "expected one finding, got {f:?}");
        assert_eq!(f[0].specifier, "./User");
        assert_eq!(f[0].suggestion, "./user");
    }

    #[test]
    fn accepts_correctly_cased_import() {
        let d = tempfile::TempDir::new().unwrap();
        write(d.path(), "src/user.ts", "export const x = 1;");
        write(d.path(), "src/app.ts", "import { x } from './user';\n");
        assert!(lint_cross_os(d.path()).is_empty());
    }

    #[test]
    fn flags_mismatched_subdirectory() {
        let d = tempfile::TempDir::new().unwrap();
        write(d.path(), "src/components/Button.tsx", "export default 1;");
        write(d.path(), "src/app.tsx", "import B from './Components/Button';\n");
        let f = lint_cross_os(d.path());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].suggestion, "./components/Button");
    }

    #[test]
    fn ignores_bare_package_imports() {
        let d = tempfile::TempDir::new().unwrap();
        write(d.path(), "src/app.ts", "import React from 'react';\nimport x from '@scope/pkg';\n");
        assert!(lint_cross_os(d.path()).is_empty());
    }

    #[test]
    fn ignores_commented_imports() {
        let d = tempfile::TempDir::new().unwrap();
        write(d.path(), "src/user.ts", "export const x = 1;");
        write(d.path(), "src/app.ts", "// import { x } from './User';\n");
        assert!(lint_cross_os(d.path()).is_empty());
    }

    #[test]
    fn handles_require_and_dynamic_import() {
        let d = tempfile::TempDir::new().unwrap();
        write(d.path(), "src/helper.js", "module.exports = 1;");
        write(d.path(), "src/a.js", "const h = require('./Helper');\n");
        write(d.path(), "src/b.js", "const h = import('./Helper');\n");
        let f = lint_cross_os(d.path());
        assert_eq!(f.len(), 2, "got {f:?}");
    }
}
