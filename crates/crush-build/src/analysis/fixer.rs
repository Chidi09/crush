use std::collections::HashMap;
use std::path::{Path, PathBuf};
use regex::Regex;
use super::report::Finding;

// ── Result types ──────────────────────────────────────────────────────────────

pub struct AppliedFix {
    pub file:       PathBuf,
    pub line:       usize,
    pub finding_id: &'static str,
    pub before:     String,
    pub after:      String,
}

pub struct SkippedFix {
    pub file:       PathBuf,
    pub line:       usize,
    pub finding_id: &'static str,
    pub reason:     &'static str,
}

pub struct FixError {
    pub file:    PathBuf,
    pub message: String,
}

pub struct FixResult {
    pub applied:  Vec<AppliedFix>,
    pub skipped:  Vec<SkippedFix>,
    pub errors:   Vec<FixError>,
    pub dry_run:  bool,
}

impl FixResult {
    fn new(dry_run: bool) -> Self {
        Self { applied: Vec::new(), skipped: Vec::new(), errors: Vec::new(), dry_run }
    }

    pub fn display(&self) {
        let label = if self.dry_run { "Would apply" } else { "Applied" };

        if self.applied.is_empty() && self.skipped.is_empty() && self.errors.is_empty() {
            println!("  No auto-fixable findings.");
            return;
        }

        if !self.applied.is_empty() {
            println!("\n  {} {} fix{}:\n",
                label,
                self.applied.len(),
                if self.applied.len() == 1 { "" } else { "es" });
            for fix in &self.applied {
                println!("  ✓  {}:{}  [{}]",
                    fix.file.display(), fix.line, fix.finding_id);
                println!("     - {}", fix.before.trim());
                println!("     + {}\n", fix.after.trim());
            }
        }

        if !self.skipped.is_empty() {
            println!("  {} finding{} skipped (manual fix required):",
                self.skipped.len(),
                if self.skipped.len() == 1 { "" } else { "s" });
            for s in &self.skipped {
                println!("     {}:{}  [{}] — {}",
                    s.file.display(), s.line, s.finding_id, s.reason);
            }
            println!();
        }

        if !self.errors.is_empty() {
            println!("  {} error{}:", self.errors.len(),
                if self.errors.len() == 1 { "" } else { "s" });
            for e in &self.errors {
                println!("     {}: {}", e.file.display(), e.message);
            }
            println!();
        }

        if self.dry_run && !self.applied.is_empty() {
            println!("  Run without --dry-run to apply these fixes.\n");
        }
    }
}

// ── AutoFixer ─────────────────────────────────────────────────────────────────

pub struct AutoFixer;

impl AutoFixer {
    pub fn apply(&self, findings: &[Finding], dry_run: bool) -> std::io::Result<FixResult> {
        let mut result = FixResult::new(dry_run);

        // Group auto-fixable findings by file.
        let mut by_file: HashMap<&PathBuf, Vec<&Finding>> = HashMap::new();
        for f in findings.iter().filter(|f| f.is_auto_fixable()) {
            by_file.entry(&f.file).or_default().push(f);
        }

        for (path, file_findings) in &by_file {
            match self.fix_file(path, file_findings, dry_run) {
                Ok((applied, skipped)) => {
                    result.applied.extend(applied);
                    result.skipped.extend(skipped);
                }
                Err(e) => {
                    result.errors.push(FixError {
                        file: (*path).clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(result)
    }

    fn fix_file(
        &self,
        path: &Path,
        findings: &[&Finding],
        dry_run: bool,
    ) -> std::io::Result<(Vec<AppliedFix>, Vec<SkippedFix>)> {
        let original = std::fs::read_to_string(path)?;
        let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
        let mut applied  = Vec::new();
        let mut skipped  = Vec::new();

        for finding in findings {
            let idx = finding.line.saturating_sub(1);
            if idx >= lines.len() { continue; }

            let before = lines[idx].clone();
            match apply_fix_to_line(&before, finding.id) {
                Some(after) if after != before => {
                    lines[idx] = after.clone();
                    applied.push(AppliedFix {
                        file:       path.to_path_buf(),
                        line:       finding.line,
                        finding_id: finding.id,
                        before,
                        after,
                    });
                }
                Some(_) => {
                    skipped.push(SkippedFix {
                        file:       path.to_path_buf(),
                        line:       finding.line,
                        finding_id: finding.id,
                        reason:     "Transformation produced no change",
                    });
                }
                None => {
                    skipped.push(SkippedFix {
                        file:       path.to_path_buf(),
                        line:       finding.line,
                        finding_id: finding.id,
                        reason:     "No applicable transformation",
                    });
                }
            }
        }

        if !dry_run && !applied.is_empty() {
            // Restore trailing newline if the original had one.
            let mut content = lines.join("\n");
            if original.ends_with('\n') { content.push('\n'); }
            std::fs::write(path, content)?;
        }

        Ok((applied, skipped))
    }
}

// ── Per-ID fix functions ──────────────────────────────────────────────────────

fn apply_fix_to_line(line: &str, id: &str) -> Option<String> {
    match id {
        "DEBUG_IN_PROD"   => fix_debug_in_prod(line),
        "WEAK_CRYPTO"     => fix_weak_crypto(line),
        "INSECURE_RANDOM" => fix_insecure_random(line),
        "CORS_WILDCARD"   => fix_cors_wildcard(line),
        _                 => None,
    }
}

// DEBUG = True  →  DEBUG = False
// debug=True inside app.run(...)  →  debug=False
fn fix_debug_in_prod(line: &str) -> Option<String> {
    // Match both DEBUG = True (config var) and debug=True (kwarg).
    // Capture everything up to `True`, then substitute `False`.
    let re = Regex::new(r"(?i)(debug\s*=\s*)True").ok()?;
    if !re.is_match(line) { return None; }
    Some(re.replace_all(line, "${1}False").into_owned())
}

// MD5 → SHA256, SHA1 → SHA256, DES → AES, RC4 → ChaCha20, ECB → GCM
// Also lowercase variants that appear in function-call names (hashlib.md5 etc.)
fn fix_weak_crypto(line: &str) -> Option<String> {
    // Ordered: longest/most-specific first to avoid partial matches.
    static PAIRS: &[(&str, &str)] = &[
        (r"\bSHA-1\b",   "SHA-256"),
        (r"\bSHA1\b",    "SHA256"),
        (r"\bMD5\b",     "SHA256"),
        (r"\bDES\b",     "AES"),
        (r"\bRC4\b",     "ChaCha20"),
        (r"\bECB\b",     "GCM"),
        // lowercase (hashlib.md5, hashlib.sha1, etc.)
        (r"\bsha1\b",    "sha256"),
        (r"\bmd5\b",     "sha256"),
        (r"\bsha-1\b",   "sha-256"),
    ];

    let mut result = line.to_string();
    let mut changed = false;
    for &(pattern, replacement) in PAIRS {
        if let Ok(re) = Regex::new(pattern) {
            let next = re.replace_all(&result, replacement).into_owned();
            if next != result { changed = true; result = next; }
        }
    }
    if changed { Some(result) } else { None }
}

// Math.random()    →  crypto.randomInt(2**32) / 2**32   (JS — same 0..1 range, CSPRNG)
// random.random()  →  secrets.SystemRandom().random()   (Python — drop-in CSPRNG)
// new Random()     →  new SecureRandom()                 (Java)
fn fix_insecure_random(line: &str) -> Option<String> {
    static PAIRS: &[(&str, &str)] = &[
        (r"\bMath\.random\(\)",  "crypto.randomInt(2**32) / 2**32"),
        (r"\brandom\.random\(\)", "secrets.SystemRandom().random()"),
        (r"\bnew\s+Random\(\)",  "new SecureRandom()"),
    ];

    let mut result = line.to_string();
    let mut changed = false;
    for &(pattern, replacement) in PAIRS {
        if let Ok(re) = Regex::new(pattern) {
            let next = re.replace_all(&result, replacement).into_owned();
            if next != result { changed = true; result = next; }
        }
    }
    if changed { Some(result) } else { None }
}

// "*" or '*' as a CORS origin value  →  "YOUR_ALLOWED_ORIGIN" / 'YOUR_ALLOWED_ORIGIN'
// Uses simple string replace; safe because the finding already confirmed this line
// contains a CORS wildcard assignment, so "*" here is the origin, not a glob.
fn fix_cors_wildcard(line: &str) -> Option<String> {
    // Only touch the line if it really looks like a CORS wildcard.
    if !line.contains('*') { return None; }

    let result = line
        .replace("\"*\"", "\"YOUR_ALLOWED_ORIGIN\"")
        .replace("'*'",   "'YOUR_ALLOWED_ORIGIN'");

    if result != line { Some(result) } else { None }
}
