pub mod report;
pub mod secrets;
pub mod sqli;
pub mod vulns;
pub mod fixer;

use std::path::{Path, PathBuf};
use rayon::prelude::*;
use report::AnalysisReport;
use secrets::SecretsScanner;
use sqli::SqliScanner;
use vulns::VulnScanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    Ruby,
    PHP,
    Any,
    Unknown,
}

impl Language {
    pub fn detect(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("py")              => Language::Python,
            Some("js" | "mjs" | "cjs") => Language::JavaScript,
            Some("ts" | "tsx")      => Language::TypeScript,
            Some("rs")              => Language::Rust,
            Some("go")              => Language::Go,
            Some("java" | "kt" | "kts") => Language::Java,
            Some("rb")              => Language::Ruby,
            Some("php")             => Language::PHP,
            _                       => Language::Unknown,
        }
    }

    // Returns true if self satisfies the pattern lang (Any matches everything).
    pub fn matches(&self, pattern: &Language) -> bool {
        *pattern == Language::Any || self == pattern
            // TypeScript inherits JavaScript patterns
            || (*self == Language::TypeScript && *pattern == Language::JavaScript)
    }

    pub fn matches_any(&self, patterns: &[Language]) -> bool {
        patterns.iter().any(|p| self.matches(p))
    }
}

static SKIP_DIRS: &[&str] = &[
    "node_modules", "vendor", "target", ".git",
    "fixtures", "testdata", "__mocks__", "__pycache__",
    ".cache", "dist", "build", "out",
];

static SKIP_EXTENSIONS: &[&str] = &[
    "md", "txt", "lock", "sum", "png", "jpg", "jpeg", "gif", "svg",
    "woff", "woff2", "ttf", "eot", "ico", "pdf", "zip", "tar", "gz",
];

pub struct AnalysisConfig {
    pub enabled:    bool,
    pub block_on:   Option<report::Severity>,
    pub skip_paths: Vec<String>,
    pub ignore_ids: Vec<String>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            enabled:    true,
            block_on:   Some(report::Severity::Critical),
            skip_paths: Vec::new(),
            ignore_ids: Vec::new(),
        }
    }
}

pub struct AnalysisEngine {
    pub secrets: SecretsScanner,
    pub sqli:    SqliScanner,
    pub vulns:   VulnScanner,
    pub config:  AnalysisConfig,
}

impl AnalysisEngine {
    pub fn new() -> Self {
        Self {
            secrets: SecretsScanner::new(),
            sqli:    SqliScanner::new(),
            vulns:   VulnScanner::new(),
            config:  AnalysisConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AnalysisConfig) -> Self {
        self.config = config;
        self
    }

    pub fn scan_sync(&self, project_root: &Path) -> AnalysisReport {
        let start = std::time::Instant::now();
        let files = self.collect_files(project_root);
        let scanned_files = files.len();

        let results: Vec<(Vec<report::Finding>, usize)> = files
            .par_iter()
            .map(|file| {
                let content = std::fs::read_to_string(file).unwrap_or_default();
                let lines = content.lines().count();
                let lang = Language::detect(file);
                let mut f = Vec::new();
                f.extend(self.secrets.scan(file, &content, &lang));
                f.extend(self.sqli.scan(file, &content, &lang));
                f.extend(self.vulns.scan(file, &content, &lang));
                // Drop findings for ignored IDs
                f.retain(|finding| !self.config.ignore_ids.iter().any(|id| id == finding.id));
                (f, lines)
            })
            .collect();

        let mut findings = Vec::new();
        let mut scanned_lines = 0usize;
        for (f, l) in results {
            findings.extend(f);
            scanned_lines += l;
        }

        AnalysisReport {
            findings,
            scanned_files,
            scanned_lines,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    fn collect_files(&self, root: &Path) -> Vec<PathBuf> {
        walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !SKIP_DIRS.contains(&name.as_ref())
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                let path = e.path();
                if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
                    if SKIP_EXTENSIONS.contains(&ext) { return false; }
                }
                let rel = path.strip_prefix(root).unwrap_or(path)
                    .to_string_lossy().replace('\\', "/");
                // Belt-and-suspenders: exclude any file inside a skipped dir
                // even if filter_entry didn't prune it (e.g. symlinks on Windows).
                if SKIP_DIRS.iter().any(|skip| {
                    rel.starts_with(&format!("{}/", skip))
                        || rel.contains(&format!("/{}/", skip))
                }) {
                    return false;
                }
                !self.config.skip_paths.iter().any(|p| rel.contains(p.as_str()))
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }
}

/// Runs the analysis engine concurrently with the build pipeline.
/// Uses `spawn_blocking` so rayon's CPU parallelism doesn't starve the tokio runtime.
pub async fn scan_async(project_root: PathBuf) -> AnalysisReport {
    tokio::task::spawn_blocking(move || {
        AnalysisEngine::new().scan_sync(&project_root)
    })
    .await
    .unwrap_or_else(|_| AnalysisReport::empty())
}
