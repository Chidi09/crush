use std::path::Path;
use regex::Regex;
use super::Language;
use super::report::{Finding, Severity, Category};

// (id, lang, pattern, description, fix)
static SQLI_PATTERN_DEFS: &[(&str, Language, &str, &str, &str)] = &[
    (
        "SQLI_PYTHON_FSTRING",
        Language::Python,
        r#"(?:execute|query|raw)\s*\(\s*f["'].*\{.*\}.*["']"#,
        "f-string interpolation in SQL query — SQL injection vector",
        "Use parameterised queries: cursor.execute('SELECT * FROM users WHERE id = %s', (user_id,))",
    ),
    (
        "SQLI_JS_TEMPLATE",
        Language::JavaScript,
        r#"(?:query|execute|run)\s*\(\s*`[^`]*\$\{[^}]+\}[^`]*`"#,
        "Template literal interpolation in SQL query — SQL injection vector",
        "Use parameterised queries: db.query('SELECT * FROM users WHERE id = ?', [userId])",
    ),
    (
        "SQLI_PYTHON_CONCAT",
        Language::Python,
        r#"["']SELECT\s.+["']\s*\+\s*\w+"#,
        "String concatenation into SQL query — SQL injection vector",
        "Use parameterised queries or an ORM",
    ),
    (
        "SQLI_JAVA_CONCAT",
        Language::Java,
        r#"["'](?:SELECT|INSERT|UPDATE|DELETE)[^"']*["']\s*\+"#,
        "String concatenation into SQL — classic injection vector",
        "Use PreparedStatement with ? placeholders",
    ),
    (
        "SQLI_RUST_FORMAT",
        Language::Rust,
        r#"format!\s*\(\s*["'](?:SELECT|INSERT|UPDATE|DELETE)[^"']*\{\}"#,
        "format! macro interpolation in SQL query — SQL injection vector",
        "Use sqlx query! macro or parameterised bind()",
    ),
];

static NOSQL_PATTERNS: &[&str] = &[
    r"\.find\(\s*\{[^}]*req\.\s*(?:body|query|params)",
    r"\.findOne\(\s*\{[^}]*req\.\s*(?:body|query|params)",
    r"\.update\(\s*\{[^}]*req\.\s*(?:body|query|params)",
];

// (pattern, description)
static ORM_RAW_PATTERNS: &[(&str, &str)] = &[
    (r"\.raw\(\s*f[\"']",         "Django raw() with f-string — SQL injection vector"),
    (r"\.raw\(\s*[\"'][^\"']*\+", "Django raw() with string concatenation — SQL injection vector"),
    (r"execute\(\s*f[\"']",       "SQLAlchemy execute() with f-string — SQL injection vector"),
    (r"text\(\s*f[\"']",          "SQLAlchemy text() with f-string — SQL injection vector"),
];

static CONN_STRING_FIX: &str =
    "Move database URL to environment variable: DATABASE_URL=... and use std::env::var()";

pub struct SqliScanner {
    patterns: Vec<(&'static str, Language, Regex, &'static str, &'static str)>,
    nosql:    Vec<Regex>,
    orm:      Vec<(Regex, &'static str)>,
    conn:     Regex,
}

impl SqliScanner {
    pub fn new() -> Self {
        let patterns = SQLI_PATTERN_DEFS.iter()
            .filter_map(|&(id, lang, pat, desc, fix)| {
                Regex::new(pat).ok().map(|r| (id, lang, r, desc, fix))
            })
            .collect();

        let nosql = NOSQL_PATTERNS.iter()
            .filter_map(|&p| Regex::new(p).ok())
            .collect();

        let orm = ORM_RAW_PATTERNS.iter()
            .filter_map(|&(p, desc)| Regex::new(p).ok().map(|r| (r, desc)))
            .collect();

        let conn = Regex::new(
            r"(?:postgres|postgresql|mysql|mongodb|redis|mssql)://[A-Za-z0-9][^:]*:[^@\s]+@"
        ).unwrap();

        Self { patterns, nosql, orm, conn }
    }

    pub fn scan(&self, path: &Path, content: &str, lang: &Language) -> Vec<Finding> {
        let mut findings = Vec::new();

        for (line_no, line) in content.lines().enumerate() {
            let line_no = line_no + 1;

            // SQL injection — language-aware patterns
            for &(id, pat_lang, ref re, desc, fix) in &self.patterns {
                if !lang.matches(&pat_lang) { continue; }
                if re.is_match(line) {
                    findings.push(Finding {
                        id,
                        severity: Severity::Critical,
                        category: Category::Injection,
                        file: path.to_path_buf(),
                        line: line_no,
                        col: 0,
                        snippet: line.to_string(),
                        description: desc.to_string(),
                        fix: fix.to_string(),
                        confidence: 0.9,
                    });
                }
            }

            // NoSQL injection — JavaScript/TypeScript only
            if matches!(lang, Language::JavaScript | Language::TypeScript) {
                for re in &self.nosql {
                    if re.is_match(line) {
                        findings.push(Finding {
                            id: "NOSQL_INJECTION",
                            severity: Severity::Critical,
                            category: Category::Injection,
                            file: path.to_path_buf(),
                            line: line_no,
                            col: 0,
                            snippet: line.to_string(),
                            description: "User input passed directly to MongoDB query — NoSQL injection".to_string(),
                            fix: "Sanitise and validate input before use in query. Use $eq operator explicitly.".to_string(),
                            confidence: 0.85,
                        });
                        break;
                    }
                }
            }

            // ORM misuse — Python only
            if *lang == Language::Python {
                for (re, desc) in &self.orm {
                    if re.is_match(line) {
                        findings.push(Finding {
                            id: "ORM_RAW_INJECTION",
                            severity: Severity::Critical,
                            category: Category::Injection,
                            file: path.to_path_buf(),
                            line: line_no,
                            col: 0,
                            snippet: line.to_string(),
                            description: desc.to_string(),
                            fix: "Use ORM parameterised methods: .filter(), .exclude(), .annotate()".to_string(),
                            confidence: 0.9,
                        });
                        break;
                    }
                }
            }

            // Connection string with embedded credentials — all languages
            if self.conn.is_match(line) {
                findings.push(Finding {
                    id: "CONN_STRING_CREDS",
                    severity: Severity::Critical,
                    category: Category::Secret,
                    file: path.to_path_buf(),
                    line: line_no,
                    col: 0,
                    snippet: line.to_string(),
                    description: "Database connection string with embedded credentials".to_string(),
                    fix: CONN_STRING_FIX.to_string(),
                    confidence: 0.95,
                });
            }
        }

        findings
    }
}
