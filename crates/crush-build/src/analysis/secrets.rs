use std::path::Path;
use regex::Regex;
use super::Language;
use super::report::{Finding, Severity, Category};

// (id, pattern, description, entropy_required)
static SECRET_PATTERNS: &[(&str, &str, &str, bool)] = &[
    ("aws_key",       r"AKIA[0-9A-Z]{16}",
                      "AWS Access Key ID", false),
    // Boundary groups keep these from matching inside longer blobs (the
    // regex crate has no lookaround, so the secret itself is capture #1).
    ("aws_secret",    r"(?:^|[^0-9a-zA-Z/+])([0-9a-zA-Z/+]{40})(?:[^0-9a-zA-Z/+=]|$)",
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
    ("db_conn",       r"(?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|rediss?|amqps?)://[^:/\s]+:[^@\s]+@",
                      "Database connection string with embedded credentials", false),
    ("basic_auth",    r"https?://[^:@\s]+:[^@\s]+@",
                      "HTTP Basic Auth credentials in URL", false),
    ("hex_key",       r"(?:^|[^0-9a-fA-F])([0-9a-f]{64})(?:[^0-9a-fA-F]|$)",
                      "Possible hex-encoded key", true),
    ("openai_project_key", r"sk-proj-[A-Za-z0-9_-]{20,}",
                      "OpenAI Project API Key", false),
    ("gitlab_pat",    r"glpat-[0-9a-zA-Z_-]{20,}",
                      "GitLab Personal Access Token", false),
    ("npm_token",     r"npm_[a-zA-Z0-9]{36}",
                      "npm Access Token", false),
    ("vercel_token",  r"vc_(?:token_)?[a-zA-Z0-9]{24,}",
                      "Vercel Access Token", false),
    ("cloudflare_api_token", r"[a-zA-Z0-9_-]{40}",
                      "Cloudflare API Token", true),

    // ── Crush / olpdf / variantrade proprietary tokens ───────────────────────
    ("crush_api_key",     r"\bcrush_sk_[A-Za-z0-9]{31}\b",
                          "Crush API key (internal)", false),
    ("olpdf_auth_token",  r"\bolpdf_auth_[a-f0-9]{21}\b",
                          "olpdf auth token (internal)", false),
    ("variantrade_token", r"\bvrt_live_[A-Za-z0-9\-_]{39}",
                          "variantrade live token (internal)", false),
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

        // Quote types must match: "..." or '...', never "...' across the two.
        let string_re = Regex::new(r#""([^"]{20,})"|'([^']{20,})'"#)
            .unwrap_or_else(|_| Regex::new(r"^$").unwrap());

        Self { patterns, assign_re, string_re }
    }

    pub fn scan(&self, path: &Path, content: &str, _lang: &Language) -> Vec<Finding> {
        if self.is_skip_path(path) { return vec![]; }

        let mut findings = Vec::new();

        for (line_no, line) in content.lines().enumerate() {
            let line_no = line_no + 1;
            if line.contains("crush:ignore-secret") {
                continue;
            }
            // Tiers run most-specific first; the first tier that matches a
            // line owns it, so one leaked credential yields one finding
            // instead of pattern + assignment + entropy triplicates.
            let mut line_matched = false;

            // Tier 1: known secret patterns
            for &(id, desc, ref re, entropy_required) in &self.patterns {
                if let Some(cap) = re.captures(line) {
                    // Boundary-guarded patterns put the secret in group 1.
                    let m = cap.get(1).or_else(|| cap.get(0)).unwrap();
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
                    line_matched = true;
                }
            }

            // Tier 2: sensitive variable name with string literal value
            if !line_matched {
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
                        line_matched = true;
                    }
                }
            }

            // Tier 3: entropy scan on quoted strings (weakest signal — only
            // when nothing more specific already flagged the line)
            if !line_matched {
                for cap in self.string_re.captures_iter(line) {
                    let s = cap.get(1).or_else(|| cap.get(2)).map_or("", |m| m.as_str());
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
    if s.len() <= 20 { return false; }
    // Hex tops out at 4 bits/char (16-symbol alphabet), so a flat 4.5
    // threshold can never fire on hex keys. Scale by alphabet.
    let is_hex = s.bytes().all(|b| b.is_ascii_hexdigit());
    let threshold = if is_hex { 3.3 } else { 4.5 };
    entropy(s) > threshold
}

fn is_placeholder(value: &str) -> bool {
    let v = value.to_lowercase();
    if is_high_entropy(value) { return false; }

    let matches_word = |v: &str, p: &str| {
        v == p 
          || v.starts_with(&format!("{}_", p))
          || v.ends_with(&format!("_{}", p))
          || v.contains(&format!("_{}_", p))
          || v.starts_with(&format!("{}-", p))
          || v.ends_with(&format!("-{}", p))
          || v.contains(&format!("-{}-", p))
          || v.contains(&format!(" {} ", p))
          || v.starts_with(&format!("{} ", p))
          || v.ends_with(&format!(" {}", p))
    };

    PLACEHOLDER_WORDS.iter().any(|&p| {
        if matches!(p, "test" | "fake" | "sample" | "dummy") {
            matches_word(&v, p)
        } else {
            v.contains(p)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scan(content: &str) -> Vec<Finding> {
        SecretsScanner::new().scan(&PathBuf::from("src/config.rs"), content, &Language::Rust)
    }

    #[test]
    fn detects_postgresql_scheme_connection_string() {
        let f = scan(r#"let url = "postgresql://admin:hunter2@db.internal:5432/app";"#);
        assert!(f.iter().any(|x| x.id == "db_conn"), "postgresql:// must be flagged");
    }

    #[test]
    fn detects_mongodb_srv_and_rediss() {
        assert!(scan(r#"u = "mongodb+srv://u:p@cluster0.mongodb.net/db""#).iter().any(|x| x.id == "db_conn"));
        assert!(scan(r#"u = "rediss://u:p@redis.example.com:6380""#).iter().any(|x| x.id == "db_conn"));
    }

    #[test]
    fn detects_hex_encoded_key() {
        // 64 random hex chars — previously unreachable behind the 4.5 entropy gate
        let f = scan(r#"key = "9f8e2c4b6a1d3e5f7a9b0c2d4e6f8a1b3c5d7e9f0a2b4c6d8e0f1a3b5c7d9e1f""#);
        assert!(f.iter().any(|x| x.id == "hex_key"), "high-entropy hex key must be flagged");
    }

    #[test]
    fn low_entropy_hex_not_flagged() {
        let f = scan(r#"zeros = "0000000000000000000000000000000000000000000000000000000000000000""#);
        assert!(!f.iter().any(|x| x.id == "hex_key"));
    }

    #[test]
    fn one_finding_per_leaked_credential_line() {
        // AWS key id in a sensitive assignment: pattern tier owns the line,
        // assignment + entropy tiers must stay quiet.
        let f = scan(r#"api_key = "AKIAIOSFODNN7EXAMPLE""#);
        assert_eq!(f.len(), 1, "expected exactly one finding, got: {:?}",
                   f.iter().map(|x| x.id).collect::<Vec<_>>());
        assert_eq!(f[0].id, "aws_key");
    }

    #[test]
    fn mismatched_quotes_do_not_create_entropy_capture() {
        // " ... ' must not be treated as one quoted string
        let f = scan(r#"println!("step one done: {}', elapsed_AbC123xYz987);"#);
        assert!(!f.iter().any(|x| x.id == "HIGH_ENTROPY_SECRET"));
    }

    #[test]
    fn hardcoded_password_assignment_flagged() {
        let f = scan(r#"password = "Sup3r$ecretValue99""#);
        assert!(f.iter().any(|x| x.id == "HARDCODED_SECRET"));
    }

    #[test]
    fn placeholder_password_not_flagged() {
        let f = scan(r#"password = "your_password_here""#);
        assert!(f.is_empty());
    }

    #[test]
    fn inline_comment_ignores_secret() {
        let f = scan(r#"password = "Sup3r$ecretValue99" // crush:ignore-secret"#);
        assert!(f.is_empty(), "inline comment should suppress findings");
    }

    #[test]
    fn real_secret_with_test_substring_not_ignored() {
        let f = scan(r#"password = "Testing123!""#);
        assert!(!f.is_empty(), "real secret containing 'test' substring must be flagged");
    }

    #[test]
    fn new_token_formats_flagged() {
        assert!(scan(r#"let token = "glpat-A1B2C3D4E5F6G7H8I9J0""#).iter().any(|x| x.id == "gitlab_pat"));
        assert!(scan(r#"let token = "npm_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8""#).iter().any(|x| x.id == "npm_token"));
        assert!(scan(r#"let token = "vc_token_a1b2c3d4e5f6g7h8i9j0k1l2""#).iter().any(|x| x.id == "vercel_token"));
        assert!(scan(r#"let token = "sk-proj-A1B2C3D4E5F6G7H8I9J0K1L2M3N4O5P6Q7R8S9T0""#).iter().any(|x| x.id == "openai_project_key"));
    }

    #[test]
    fn skips_env_example_files() {
        let s = SecretsScanner::new();
        let f = s.scan(&PathBuf::from(".env.example"),
            r#"DB_PASSWORD="postgresql://a:b@c/d""#, &Language::Any);
        assert!(f.is_empty());
    }
}
