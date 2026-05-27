use std::path::Path;
use regex::Regex;
use super::Language;
use super::report::{Finding, Severity, Category};

struct VulnPatternDef {
    id:          &'static str,
    languages:   &'static [Language],
    pattern:     &'static str,
    severity:    Severity,
    description: &'static str,
    fix:         &'static str,
}

static VULN_PATTERN_DEFS: &[VulnPatternDef] = &[
    VulnPatternDef {
        id: "PATH_TRAVERSAL",
        languages: &[Language::Python, Language::JavaScript, Language::TypeScript, Language::Go],
        pattern: r"open\s*\(\s*.*\+.*(?:filename|filepath|path|file)\b",
        severity: Severity::Critical,
        description: "User-controlled path in file open — allows reading /etc/passwd and similar",
        fix: "Validate path is within expected directory: path.starts_with(base_dir) after canonicalise()",
    },
    VulnPatternDef {
        id: "CMD_INJECTION",
        languages: &[Language::Python, Language::JavaScript, Language::Ruby],
        pattern: r"(?:os\.system|subprocess\.call|exec|shell_exec)\s*\([^)]*\+",
        severity: Severity::Critical,
        description: "User input concatenated into shell command — command injection",
        fix: "Use subprocess with list args: subprocess.run(['cmd', arg]) — never shell=True with user input",
    },
    VulnPatternDef {
        id: "EVAL_INJECTION",
        languages: &[Language::JavaScript, Language::TypeScript, Language::Python, Language::Ruby, Language::PHP],
        pattern: r"\beval\s*\([^)]*(?:req\.|request\.|params\.|query\.|body\.)",
        severity: Severity::Critical,
        description: "eval() called with user-controlled input — arbitrary code execution",
        fix: "Never use eval() with external input. Use JSON.parse() for data, proper parsers for code.",
    },
    VulnPatternDef {
        id: "JWT_NONE_ALG",
        languages: &[Language::JavaScript, Language::TypeScript, Language::Python],
        pattern: r#"(?:algorithm|algorithms)\s*[=:]\s*["']none["']"#,
        severity: Severity::Critical,
        description: "JWT 'none' algorithm accepted — unsigned tokens bypass authentication",
        fix: "Always specify a strong algorithm explicitly: HS256, RS256, ES256",
    },
    VulnPatternDef {
        id: "SSRF",
        languages: &[Language::Python, Language::JavaScript, Language::TypeScript, Language::Go, Language::Java],
        pattern: r"(?:fetch|axios|requests\.get|http\.get|urllib\.request)\s*\([^)]*(?:req\.|request\.|params\.|query\.|body\.)",
        severity: Severity::High,
        description: "HTTP request to user-controlled URL — Server-Side Request Forgery",
        fix: "Validate URL against allowlist before request. Block 169.254.x.x, 10.x.x.x, 172.16-31.x.x, 192.168.x.x.",
    },
    VulnPatternDef {
        id: "INSECURE_DESERIALIZATION",
        languages: &[Language::Python, Language::Java, Language::Ruby],
        pattern: r"(?:pickle\.loads|yaml\.load\s*\([^)]*Loader\s*=\s*None|Marshal\.load|ObjectInputStream)",
        severity: Severity::High,
        description: "Unsafe deserialization — can lead to remote code execution",
        fix: "Python: yaml.safe_load() instead of yaml.load(). Java: use Jackson with polymorphic type handling disabled.",
    },
    VulnPatternDef {
        id: "HARDCODED_ADMIN",
        languages: &[Language::Any],
        // Require a password/credential context word before the admin/root key so we don't
        // fire on object labels like `role: { admin: "Admin" }` or enum values.
        pattern: r#"(?i)(?:password|passwd|pwd|credential|secret)\s*[=:]\s*["'](?:admin|root|administrator|password|123|pass)[^"']{0,20}["']"#,
        severity: Severity::High,
        description: "Hardcoded admin credentials",
        fix: "Load credentials from environment variables or secrets manager",
    },
    VulnPatternDef {
        id: "INSECURE_RANDOM",
        languages: &[Language::Python, Language::JavaScript, Language::TypeScript, Language::Java],
        pattern: r"(?:Math\.random|random\.random|Random\(\)\.next).*(?:token|key|secret|password|salt|nonce)",
        severity: Severity::High,
        description: "Non-cryptographic RNG used for security-sensitive value",
        fix: "Use crypto.randomBytes() (Node), secrets.token_hex() (Python), SecureRandom (Java)",
    },
    VulnPatternDef {
        id: "WEAK_CRYPTO",
        languages: &[Language::Any],
        pattern: r"\b(?:MD5|SHA1|DES|RC4|ECB)\b",
        severity: Severity::Medium,
        description: "Weak or broken cryptographic algorithm",
        fix: "Use SHA-256+, AES-GCM, or ChaCha20-Poly1305",
    },
    VulnPatternDef {
        id: "CORS_WILDCARD",
        languages: &[Language::JavaScript, Language::TypeScript, Language::Python, Language::Go],
        pattern: r#"(?:Access-Control-Allow-Origin|allow_origins|AllowOrigins)\s*[=:,\(]\s*["']\*["']"#,
        severity: Severity::Medium,
        description: "CORS wildcard origin — credentials may be exposed to any website",
        fix: "Specify exact allowed origins. Never combine wildcard with credentials: true.",
    },
    VulnPatternDef {
        id: "DEBUG_IN_PROD",
        languages: &[Language::Python, Language::JavaScript, Language::TypeScript],
        pattern: r"(?:DEBUG\s*=\s*True|app\.run\([^)]*debug\s*=\s*True)",
        severity: Severity::Medium,
        description: "Debug mode enabled — exposes stack traces and internal state to users",
        fix: "Set DEBUG = False in production. Use environment variable: DEBUG = os.getenv('DEBUG', 'False') == 'True'",
    },
    VulnPatternDef {
        id: "OPEN_REDIRECT",
        languages: &[Language::Python, Language::JavaScript, Language::TypeScript, Language::Ruby],
        pattern: r"redirect\s*\([^)]*(?:req\.|request\.|params\.|query\.)",
        severity: Severity::Medium,
        description: "Redirect to user-controlled URL — open redirect / phishing vector",
        fix: "Validate redirect URL is within your domain before redirecting",
    },
    VulnPatternDef {
        id: "SQL_SLEEP",
        languages: &[Language::Any],
        pattern: r"(?i)SLEEP\s*\(\s*\d+\s*\)|WAITFOR\s+DELAY|pg_sleep",
        severity: Severity::Low,
        description: "SQL time-delay function in source — possible time-based injection test artefact",
        fix: "Verify this is not reachable via user input",
    },
];

pub struct VulnScanner {
    patterns: Vec<(&'static str, &'static [Language], Regex, Severity, &'static str, &'static str)>,
}

impl VulnScanner {
    pub fn new() -> Self {
        let patterns = VULN_PATTERN_DEFS.iter()
            .filter_map(|p| {
                Regex::new(p.pattern).ok().map(|r| {
                    (p.id, p.languages, r, p.severity.clone(), p.description, p.fix)
                })
            })
            .collect();
        Self { patterns }
    }

    pub fn scan(&self, path: &Path, content: &str, lang: &Language) -> Vec<Finding> {
        let mut findings = Vec::new();

        for (line_no, line) in content.lines().enumerate() {
            let line_no = line_no + 1;

            for &(id, languages, ref re, ref severity, description, fix) in &self.patterns {
                if !lang.matches_any(languages) { continue; }
                if re.is_match(line) {
                    findings.push(Finding {
                        id,
                        severity: severity.clone(),
                        category: Category::Vulnerability,
                        file: path.to_path_buf(),
                        line: line_no,
                        col: 0,
                        snippet: line.to_string(),
                        description: description.to_string(),
                        fix: fix.to_string(),
                        confidence: 0.8,
                    });
                }
            }
        }

        findings
    }
}
