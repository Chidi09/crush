use std::path::PathBuf;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High     => write!(f, "HIGH    "),
            Severity::Medium   => write!(f, "MEDIUM  "),
            Severity::Low      => write!(f, "LOW     "),
            Severity::Info     => write!(f, "INFO    "),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    Secret,
    Injection,
    Vulnerability,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub id:          &'static str,
    pub severity:    Severity,
    pub category:    Category,
    pub file:        PathBuf,
    pub line:        usize,
    pub col:         usize,
    pub snippet:     String,
    pub description: String,
    pub fix:         String,
    pub confidence:  f32,
}

impl Finding {
    pub fn is_auto_fixable(&self) -> bool {
        matches!(self.id, "DEBUG_IN_PROD" | "WEAK_CRYPTO" | "INSECURE_RANDOM" | "CORS_WILDCARD")
    }
}

pub struct AnalysisReport {
    pub findings:      Vec<Finding>,
    pub scanned_files: usize,
    pub scanned_lines: usize,
    pub duration_ms:   u64,
}

impl AnalysisReport {
    pub fn empty() -> Self {
        Self { findings: Vec::new(), scanned_files: 0, scanned_lines: 0, duration_ms: 0 }
    }

    pub fn display(&self) {
        if self.findings.is_empty() {
            println!("\n  No security findings  ({} files · {}ms)\n",
                self.scanned_files, self.duration_ms);
            return;
        }

        println!("\n{} Security Analysis ({}ms · {} files) {}\n",
            "─".repeat(3),
            self.duration_ms,
            self.scanned_files,
            "─".repeat(30));

        let mut sorted = self.findings.clone();
        sorted.sort_by(|a, b| a.severity.cmp(&b.severity));

        for f in &sorted {
            println!("  {}  {}:{}", f.severity, f.file.display(), f.line);
            println!("            {}", f.description);
            let snip = f.snippet.trim();
            if !snip.is_empty() {
                println!("            {}", snip);
            }
            println!("            → {}\n", f.fix);
        }

        let auto_fixable = self.findings.iter().filter(|f| f.is_auto_fixable()).count();
        let unique_files: HashSet<_> = self.findings.iter().map(|f| &f.file).collect();

        print!("  {} findings · {} files", self.findings.len(), unique_files.len());
        if auto_fixable > 0 {
            print!(" · run `crush scan --fix` to apply {} safe auto-fix{}",
                auto_fixable,
                if auto_fixable == 1 { "" } else { "es" });
        }
        println!("\n");
    }

    pub fn has_blocking_findings(&self, threshold: &Severity) -> bool {
        self.findings.iter().any(|f| &f.severity <= threshold)
    }
}
