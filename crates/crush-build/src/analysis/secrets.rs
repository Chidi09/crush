use std::path::Path;
use regex::Regex;
use super::Language;
use super::report::{Finding, Severity, Category};

// (id, pattern, description, entropy_required)
static SECRET_PATTERNS: &[(&str, &str, &str, bool)] = &[
    ("aws_key",       r"AKIA[0-9A-Z]{16}",
                      "AWS Access Key ID", false),
    ("aws_secret",    r"[0-9a-zA-Z/+]{40}",
                      "AWS Secret Key", true),
    ("github_token",  r"gh[pousr]_[A-Za-z0-9]{36}",
                      "GitHub Token", false),
    ("github_fine",   r"github_pat_[A-Za-z0-9]{82}",
                      "GitHub Fine-grained PAT", false),
    ("stripe_live",   r"sk_live_[0-9a-zA-Z]{24}",
                      "Stripe Live Secret Key", false),
    ("stripe_test",   r"sk_test_[0-9a-zA-Z]{24}",
                      "Stripe Test Key", false),
    ("jwt",           r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
                      "JWT Token", false),
    ("private_key",   r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
                      "Private Key", false),
    ("anthropic_key", r"sk-ant-[A-Za-z0-9]{40,}",
                      "Anthropic API Key", false),
    ("openai_key",    r"sk-[A-Za-z0-9]{48}",
                      "OpenAI API Key", false),
    ("sendgrid",      r"SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}",
                      "SendGrid API Key", false),
    ("twilio",        r"SK[0-9a-fA-F]{32}",
                      "Twilio Secret Key", false),
    ("slack_token",   r"xox[baprs]-[0-9A-Za-z]{10,48}",
                      "Slack Token", false),
    ("gcp_sa",        r#""type":\s*"service_account""#,
                      "GCP Service Account JSON", false),
    ("db_conn",       r"(?:postgres|mysql|mongodb|redis)://[^:]+:[^@\s]+@",
                      "Database connection string with embedded credentials", false),
    ("basic_auth",    r"https?://[^:@\s]+:[^@\s]+@",
                      "HTTP Basic Auth credentials in URL", false),
    ("hex_key",       r"[0-9a-f]{64}",
                      "Possible hex-encoded key", true),
];

static SENSITIVE_VAR_NAMES: &[&str] = &[
    "password", "passwd", "pwd", "secret", "api_key", "apikey",
    "token", "auth_token", "access_token", "private_key", "credential",
    "database_url", "db_password", "db_pass", "connection_string",
    "encryption_key", "signing_key", "master_key", "client_secret",
];

static PLACEHOLDER_WORDS: &[&str] = &[
    "example", "placeholder", "replace", "your_", "changeme",
    "todo", "fixme", "xxx", "dummy", "test", "fake", "sample",
    "insert", "here", "your-", "<", ">",
];

static SKIP_SUFFIXES: &[&str] = &[
    ".env.example", ".env.sample", ".env.test",
    ".test.", ".spec.", "_test.go", "_test.rs",
];

static SKIP_NAMES: &[&str] = &[
    "CHANGELOG", "LICENSE", "COPYING", "AUTHORS",
];

pub struct SecretsScanner {
    // (id, description, compiled regex, entropy_required)
    patterns:  Vec<(&'static str, &'static str, Regex, bool)>,
    assign_re: Regex,
    string_re: Regex,
}

impl SecretsScanner {
    pub fn new() -> Self {
        let patterns = SECRET_PATTERNS.iter()
            .filter_map(|&(id, pat, desc, entropy)| {
                Regex::new(pat).ok().map(|r| (id, desc, r, entropy))
            })
            .collect();

        let sensitive_names = SENSITIVE_VAR_NAMES.join("|");
        let assign_pattern = format!(
            r#"(?i)(?:^|[^a-zA-Z])({names})\s*[=:]\s*["']([^"']{{4,}})["']"#,
            names = sensitive_names
        );
        let assign_re = Regex::new(&assign_pattern)
            .unwrap_or_else(|_| Regex::new(r"^$").unwrap());

        let string_re = Regex::new(r#"["']([^"']{20,})["']"#)
            .unwrap_or_else(|_| Regex::new(r"^$").unwrap());

        Self { patterns, assign_re, string_re }
    }

    pub fn scan(&self, path: &Path, content: &str, _lang: &Language) -> Vec<Finding> {
        if self.is_skip_path(path) { return vec![]; }

        let mut findings = Vec::new();

        for (line_no, line) in content.lines().enumerate() {
            let line_no = line_no + 1;

            // Tier 1: entropy scan on quoted strings
            for cap in self.string_re.captures_iter(line) {
                let s = &cap[1];
                if is_high_entropy(s) {
                    findings.push(Finding {
                        id: "HIGH_ENTROPY_SECRET",
                        severity: Severity::High,
                        category: Category::Secret,
                        file: path.to_path_buf(),
                        line: line_no,
                        col: 0,
                        snippet: line.to_string(),
                        description: "High-entropy string — possible hardcoded secret".to_string(),
                        fix: "Move to environment variable or secrets manager".to_string(),
                        confidence: 0.7,
                    });
                    break; // one finding per line for entropy
                }
            }

            // Tier 2: known secret patterns
            for &(id, desc, ref re, entropy_required) in &self.patterns {
                if let Some(m) = re.find(line) {
                    if entropy_required && !is_high_entropy(m.as_str()) {
                        continue;
                    }
                    findings.push(Finding {
                        id,
                        severity: if id.contains("test") || id.contains("sample") {
                            Severity::Medium
                        } else {
                            Severity::Critical
                        },
                        category: Category::Secret,
                        file: path.to_path_buf(),
                        line: line_no,
                        col: m.start(),
                        snippet: line.to_string(),
                        description: desc.to_string(),
                        fix: "Move to environment variable: use std::env::var(\"SECRET_NAME\") or a secrets manager".to_string(),
                        confidence: if entropy_required { 0.75 } else { 0.95 },
                    });
                }
            }

            // Tier 3: sensitive variable name with string literal value
            if let Some(cap) = self.assign_re.captures(line) {
                let var_name = cap.get(1).map_or("", |m| m.as_str());
                let value    = cap.get(2).map_or("", |m| m.as_str());
                if !is_placeholder(value) {
                    findings.push(Finding {
                        id: "HARDCODED_SECRET",
                        severity: Severity::Critical,
                        category: Category::Secret,
                        file: path.to_path_buf(),
                        line: line_no,
                        col: 0,
                        snippet: line.to_string(),
                        description: format!("Hardcoded value assigned to sensitive variable '{}'", var_name),
                        fix: "Use environment variable or secrets manager instead of hardcoded literal".to_string(),
                        confidence: 0.85,
                    });
                }
            }
        }

        findings
    }

    fn is_skip_path(&self, path: &Path) -> bool {
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        let path_str = path.to_string_lossy().replace('\\', "/");

        if SKIP_NAMES.contains(&filename.as_ref()) { return true; }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "md" | "txt" | "lock" | "sum" | "png" | "jpg" | "gif") {
                return true;
            }
        }

        for s in SKIP_SUFFIXES {
            if path_str.contains(s) || filename.contains(s) { return true; }
        }

        false
    }
}

fn entropy(s: &str) -> f64 {
    let mut freq = [0usize; 256];
    for b in s.bytes() { freq[b as usize] += 1; }
    let len = s.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| { let p = c as f64 / len; -p * p.log2() })
        .sum()
}

fn is_high_entropy(s: &str) -> bool {
    s.len() > 20 && entropy(s) > 4.5
}

fn is_placeholder(value: &str) -> bool {
    let v = value.to_lowercase();
    PLACEHOLDER_WORDS.iter().any(|&p| v.contains(p))
}
