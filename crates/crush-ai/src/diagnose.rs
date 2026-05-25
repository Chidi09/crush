use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};
use crate::parsers::ParsedTrace;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDiagnosis {
    pub root_cause: String,
    pub fix_description: String,
    pub proposed_patch: Option<String>,
    pub confidence: f32,
    pub auto_fixable: bool,
}

pub struct DiagnosticEngine {
    api_key: Option<String>,
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<u64, AiDiagnosis>>>,
    offline_db: OfflineDiagnosisDb,
}

impl DiagnosticEngine {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .user_agent("crush-ai/0.1.0")
                .build()
                .expect("Failed to build HTTP client"),
            cache: Arc::new(Mutex::new(HashMap::new())),
            offline_db: OfflineDiagnosisDb::new(),
        }
    }

    pub async fn diagnose(&self, trace: &ParsedTrace, source_context: Option<&str>) -> Result<AiDiagnosis> {
        let cache_key = self.cache_key(trace);

        {
            let cache = self.cache.lock().await;
            if let Some(diag) = cache.get(&cache_key) {
                return Ok(diag.clone());
            }
        }

        if let Some(offline) = self.offline_db.lookup(trace) {
            let mut cache = self.cache.lock().await;
            cache.insert(cache_key, offline.clone());
            return Ok(offline);
        }

        if let Some(ref key) = self.api_key {
            if let Ok(diag) = self.call_anthropic_api(key, trace, source_context).await {
                let mut cache = self.cache.lock().await;
                cache.insert(cache_key, diag.clone());
                return Ok(diag);
            }
        }

        let fallback = self.fallback_diagnose(trace);
        let mut cache = self.cache.lock().await;
        cache.insert(cache_key, fallback.clone());
        Ok(fallback)
    }

    fn cache_key(&self, trace: &ParsedTrace) -> u64 {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(trace.exception_type.as_bytes());
        hasher.update(trace.file.as_bytes());
        hasher.update(trace.line.to_string().as_bytes());
        hasher.update(trace.message.as_bytes());
        let hash = hasher.finalize();
        u64::from_le_bytes([hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]])
    }

    async fn call_anthropic_api(&self, api_key: &str, trace: &ParsedTrace, source_ctx: Option<&str>) -> Result<AiDiagnosis> {
        let prompt = format!(
            "Analyze this error trace and provide a diagnosis.\nLanguage: {}\nException: {}\nMessage: {}\nFile: {}:{}\nStack:\n{}\n\nSource context:\n{}\n\nRespond with ONLY a JSON object: {{\"root_cause\":\"...\",\"fix\":\"...\",\"patch\":\"...\",\"confidence\":0.0,\"auto_fixable\":true}}",
            trace.language, trace.exception_type, trace.message, trace.file, trace.line,
            trace.stack_frames.iter().map(|f| f.raw.as_str()).collect::<Vec<_>>().join("\n"),
            source_ctx.unwrap_or("(not available)")
        );

        let body = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": "You are a senior debugging engineer. Analyze errors and return JSON diagnosis only.",
            "messages": [{"role": "user", "content": prompt}]
        });

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send().await
            .map_err(|e| CrushError::AiError(format!("API call failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(CrushError::AiError(format!("API returned HTTP {}", resp.status())));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| CrushError::AiError(format!("API parse error: {}", e)))?;

        if let Some(content) = json["content"][0]["text"].as_str() {
            if let Some(json_start) = content.find('{') {
                if let Some(json_end) = content.rfind('}') {
                    let json_str = &content[json_start..=json_end];
                    if let Ok(diag) = serde_json::from_str::<AiDiagnosis>(json_str) {
                        return Ok(diag);
                    }
                }
            }
        }

        Err(CrushError::AiError("Failed to parse diagnosis from API response".to_string()))
    }

    fn fallback_diagnose(&self, trace: &ParsedTrace) -> AiDiagnosis {
        let (root_cause, confidence) = match trace.exception_type.as_str() {
            "TypeError" => (format!("TypeError at {}:{}: null/undefined value accessed as object", trace.file, trace.line), 0.85),
            "ReferenceError" => (format!("ReferenceError: variable not defined in scope at {}", trace.file), 0.80),
            "SyntaxError" => (format!("Syntax error in {} — check for missing brackets, commas, or quotes", trace.file), 0.90),
            "ModuleNotFoundError" | "ImportError" => (format!("Missing Python dependency — install the required package"), 0.88),
            "ZeroDivisionError" => (format!("Division by zero at {}:{} — add a divisor check before division", trace.file, trace.line), 0.95),
            "panic" => (format!("Rust panic at {}:{} — unwrap on None/Err result", trace.file, trace.line), 0.70),
            "Segfault" => ("Segmentation fault — likely null pointer dereference or buffer overflow".to_string(), 0.60),
            "OutOfMemoryError" | "OOM" => ("Out of memory — container exceeded memory limit".to_string(), 0.90),
            _ => (format!("Error in {} at line {}: {}", trace.file, trace.line, trace.message), 0.60),
        };

        let fix = match trace.exception_type.as_str() {
            "ModuleNotFoundError" | "ImportError" => {
                "Run `pip install <module>` to install the missing dependency".to_string()
            }
            "ZeroDivisionError" => {
                "Add a check before division: `if divisor != 0:`".to_string()
            }
            _ => format!("Check line {} in {} for the issue. Add input validation and null checks.", trace.line, trace.file),
        };

        AiDiagnosis {
            root_cause, fix_description: fix, proposed_patch: None,
            confidence, auto_fixable: confidence > 0.8,
        }
    }
}

struct OfflineDiagnosisDb {
    patterns: Vec<OfflinePattern>,
}

struct OfflinePattern {
    exception_prefix: String,
    message_contains: Vec<String>,
    diagnosis: AiDiagnosis,
}

impl OfflineDiagnosisDb {
    fn new() -> Self {
        Self { patterns: Self::build_common_patterns() }
    }

    fn lookup(&self, trace: &ParsedTrace) -> Option<AiDiagnosis> {
        for pattern in &self.patterns {
            if trace.exception_type.starts_with(&pattern.exception_prefix) {
                if pattern.message_contains.is_empty() {
                    return Some(pattern.diagnosis.clone());
                }
                for needle in &pattern.message_contains {
                    if trace.message.contains(needle) {
                        return Some(pattern.diagnosis.clone());
                    }
                }
            }
        }
        None
    }

    fn build_common_patterns() -> Vec<OfflinePattern> {
        vec![
            // ── Python: Module & Import ──────────────────────────────
            OfflinePattern {
                exception_prefix: "ModuleNotFoundError".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Python module not installed — the package is missing from the current environment".into(),
                    fix_description: "Activate the correct virtualenv, then run `pip install <module>` and add to requirements.txt".into(),
                    proposed_patch: None, confidence: 0.92, auto_fixable: true,
                },
            },
            OfflinePattern {
                exception_prefix: "ImportError".into(), message_contains: vec!["cannot import name".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Python import failed — circular import or name doesn't exist in the target module".into(),
                    fix_description: "Check for circular imports (A imports B imports A). Verify the name exists in the target module version.".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "ImportError".into(), message_contains: vec!["No module named".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Python dependency missing — the module is referenced in code but not installed".into(),
                    fix_description: "Run `pip install <module>` or add it to requirements.txt and re-run `pip install -r requirements.txt`".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: true,
                },
            },

            // ── Python: Runtime ───────────────────────────────────────
            OfflinePattern {
                exception_prefix: "ZeroDivisionError".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Division by zero — divisor evaluated to 0 at runtime".into(),
                    fix_description: "Add a guard: `if divisor != 0:` before the division, or use `try/except ZeroDivisionError`".into(),
                    proposed_patch: None, confidence: 0.95, auto_fixable: true,
                },
            },
            OfflinePattern {
                exception_prefix: "KeyError".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Dictionary key not found — accessing a dict with a key that doesn't exist".into(),
                    fix_description: "Use `.get(key, default)` instead of `[key]` access, or wrap in `try/except KeyError`".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "AttributeError".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Object has no attribute — likely a NoneType or unexpected type returned".into(),
                    fix_description: "Check what the object actually is (`print(type(obj))`) before accessing attributes. Add null checks.".into(),
                    proposed_patch: None, confidence: 0.85, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "TypeError".into(), message_contains: vec!["not enough arguments".to_string(), "missing".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Function call with wrong number of arguments — signature mismatch".into(),
                    fix_description: "Check the function definition — number of parameters doesn't match the call site.".into(),
                    proposed_patch: None, confidence: 0.87, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "TypeError".into(), message_contains: vec!["object is not iterable".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Expected an iterable (list, tuple) but got None or a scalar".into(),
                    fix_description: "Check that the variable is not None before iterating. Ensure the function returns a list.".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },

            // ── Node.js / TypeScript: Syntax & Type ───────────────────
            OfflinePattern {
                exception_prefix: "SyntaxError".into(), message_contains: vec!["Unexpected token".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "JavaScript/TypeScript parse error — missing comma, bracket, or parenthesis".into(),
                    fix_description: "Check the line and surrounding context for a missing `,`, `}`, `]`, or `)`".into(),
                    proposed_patch: None, confidence: 0.93, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "TypeError".into(), message_contains: vec!["is not a function".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Value is not callable — a variable is undefined or the wrong type when called as a function".into(),
                    fix_description: "Check the import: is the export a function or an object? Verify ESM vs CJS module interop.".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "TypeError".into(), message_contains: vec!["Cannot read properties".to_string(), "Cannot read property".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Accessing a property on `undefined` or `null` — the object was not initialized or returned empty".into(),
                    fix_description: "Add optional chaining: `obj?.prop?.nestedProp` or a default: `const val = obj ?? fallback`".into(),
                    proposed_patch: None, confidence: 0.92, auto_fixable: true,
                },
            },
            OfflinePattern {
                exception_prefix: "ReferenceError".into(), message_contains: vec!["is not defined".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Variable referenced before declaration — typo, missing import, or scope issue".into(),
                    fix_description: "Check the variable name for typos. Ensure it's imported/declared in the current scope.".into(),
                    proposed_patch: None, confidence: 0.91, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "TypeError".into(), message_contains: vec!["undefined is not".to_string(), "null is not".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Variable is `undefined` or `null` when an object was expected — async data not yet loaded".into(),
                    fix_description: "Check that async/await completes before accessing data. Add null guard: `if (!data) return;`".into(),
                    proposed_patch: None, confidence: 0.89, auto_fixable: false,
                },
            },

            // ── Rust: Compile errors (most common) ────────────────────
            OfflinePattern {
                exception_prefix: "error[E0308]".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Type mismatch — expected one type but got another (most common Rust compile error)".into(),
                    fix_description: "Check the expected type vs the actual type. Add an explicit `.into()` or `.as_str()`, or change the variable type.".into(),
                    proposed_patch: None, confidence: 0.94, auto_fixable: true,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0382]".into(), message_contains: vec!["borrow of moved value".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Use after move — value was moved to another owner, then accessed again (ownership error)".into(),
                    fix_description: "Clone the value before moving: `.clone()`. Or restructure to use a reference `&` instead of moving.".into(),
                    proposed_patch: None, confidence: 0.93, auto_fixable: true,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0502]".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Cannot borrow as mutable because also borrowed as immutable — borrow checker conflict".into(),
                    fix_description: "Restructure to avoid simultaneous mutable and immutable references. Use `RefCell` or split borrows.".into(),
                    proposed_patch: None, confidence: 0.91, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0597]".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Borrowed value does not live long enough — dangling reference".into(),
                    fix_description: "The referenced value goes out of scope before the reference. Own the value with `.to_owned()` or adjust lifetimes.".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0277]".into(), message_contains: vec!["Sized".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Generic type parameter doesn't implement a required trait (often `Sized`) — trait bound missing".into(),
                    fix_description: "Add `where T: TraitName` bound to the generic parameter, or use `dyn Trait` with a Box.".into(),
                    proposed_patch: None, confidence: 0.87, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0432]".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Import path is wrong — the module or crate doesn't exist at that path".into(),
                    fix_description: "Check the crate name in Cargo.toml `[dependencies]` and the module path. Use `cargo check` to verify.".into(),
                    proposed_patch: None, confidence: 0.92, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0046]".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Trait method not implemented — a required trait method is missing from the impl block".into(),
                    fix_description: "Add the missing method(s) to the `impl TraitName for Type` block. Check the trait definition for all required methods.".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0106]".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Missing lifetime specifier — the compiler cannot infer lifetimes for a reference".into(),
                    fix_description: "Add explicit lifetime annotations: `fn foo<'a>(x: &'a str) -> &'a str`".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "error[E0308]".into(), message_contains: vec!["mismatched types".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "match arms have incompatible types — each arm must return the same type".into(),
                    fix_description: "Ensure all match arms return the same type. Convert early returns or add `Ok()`/`Err()` wrappers.".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },

            // ── Infrastructure: Databases & services ─────────────────
            OfflinePattern {
                exception_prefix: "ECONNREFUSED".into(), message_contains: vec![":5432".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "PostgreSQL connection refused — the database server is not running or not reachable on port 5432".into(),
                    fix_description: "Start PostgreSQL: `sudo systemctl start postgresql` or `pg_ctl start`.".into(),
                    proposed_patch: None, confidence: 0.95, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "ECONNREFUSED".into(), message_contains: vec![":6379".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "Redis connection refused — Redis server is not running or not reachable on port 6379".into(),
                    fix_description: "Start Redis: `redis-server` or `sudo systemctl start redis`".into(),
                    proposed_patch: None, confidence: 0.95, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "ECONNREFUSED".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Connection refused — the target service is not running on the specified port".into(),
                    fix_description: "Ensure the target service is started and listening. Check the port number. Verify firewall rules.".into(),
                    proposed_patch: None, confidence: 0.92, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "FATAL".into(), message_contains: vec!["role".to_string(), "does not exist".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "PostgreSQL authentication failure — the database user/role does not exist".into(),
                    fix_description: "Create the role: `CREATE ROLE <name> WITH LOGIN PASSWORD '<pass>'`. Or set the correct `PGUSER` environment variable.".into(),
                    proposed_patch: None, confidence: 0.93, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "FATAL".into(), message_contains: vec!["database".to_string(), "does not exist".to_string()],
                diagnosis: AiDiagnosis {
                    root_cause: "PostgreSQL database does not exist — the application is trying to connect to a missing database".into(),
                    fix_description: "Create the database: `CREATE DATABASE <name>`. Check the `DATABASE_URL` / `PGDATABASE` env var.".into(),
                    proposed_patch: None, confidence: 0.93, auto_fixable: false,
                },
            },

            // ── Generic system errors ─────────────────────────────────
            OfflinePattern {
                exception_prefix: "ENOENT".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "File not found — the specified path does not exist on the filesystem".into(),
                    fix_description: "Check the file path. Create parent directories if needed. Verify the file was generated by a previous step.".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "EACCES".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Permission denied — insufficient access rights to the file or directory".into(),
                    fix_description: "Check file permissions with `ls -la`. Use `chmod 755` for directories, `chmod 644` for files.".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "EISDIR".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Expected a file but a directory was found — path points to a directory instead of a file".into(),
                    fix_description: "Check the path — append the filename to the directory path.".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "ENOTDIR".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Expected a directory but a file was found — path points to a file instead of a directory".into(),
                    fix_description: "Check the path — it should be a directory but points to an existing file.".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "ENOSPC".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "No space left on device — disk is full".into(),
                    fix_description: "Free up space: `df -h` to check, remove unused Docker images, clear apt cache, truncate logs.".into(),
                    proposed_patch: None, confidence: 0.92, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "ETIMEDOUT".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Network operation timed out — the remote server did not respond within the timeout period".into(),
                    fix_description: "Check network connectivity. Increase timeout setting. Verify DNS resolution. Check firewall rules.".into(),
                    proposed_patch: None, confidence: 0.85, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "EHOSTUNREACH".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Host unreachable — no network route to the target host".into(),
                    fix_description: "Check if the host is online (`ping <host>`). Verify network configuration and routing.".into(),
                    proposed_patch: None, confidence: 0.87, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "EPERM".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Operation not permitted — the operation requires higher privileges".into(),
                    fix_description: "Run with `sudo` or adjust capabilities. Check that the process has the required Linux capabilities.".into(),
                    proposed_patch: None, confidence: 0.85, auto_fixable: false,
                },
            },

            // ── Docker / container specific ───────────────────────────
            OfflinePattern {
                exception_prefix: "Cannot connect to the Docker daemon".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Docker daemon is not running or the socket is not accessible".into(),
                    fix_description: "Start Docker: `sudo systemctl start docker`. Check socket: `ls -la /var/run/docker.sock`.".into(),
                    proposed_patch: None, confidence: 0.95, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "port is already allocated".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Port conflict — the host port is already bound by another process or container".into(),
                    fix_description: "Use a different port or stop the conflicting container: `lsof -i :<port>` to find the process.".into(),
                    proposed_patch: None, confidence: 0.93, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "no space left on device".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Docker layer storage full — disk space exhausted by container images and layers".into(),
                    fix_description: "Clean up: `docker system prune -a`. Check `/var/lib/docker` disk usage.".into(),
                    proposed_patch: None, confidence: 0.92, auto_fixable: false,
                },
            },

            // ── OOM / resource exhaustion ─────────────────────────────
            OfflinePattern {
                exception_prefix: "OutOfMemoryError".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Java process ran out of heap memory — the application requires more memory than allocated".into(),
                    fix_description: "Increase heap: `-Xmx512m` or set container memory limit higher. Check for memory leaks.".into(),
                    proposed_patch: None, confidence: 0.90, auto_fixable: false,
                },
            },
            OfflinePattern {
                exception_prefix: "Killed".into(), message_contains: vec![],
                diagnosis: AiDiagnosis {
                    root_cause: "Process was killed by the OOM killer — system ran out of available memory".into(),
                    fix_description: "Check `dmesg | grep oom` for details. Increase memory limit or reduce the process memory footprint.".into(),
                    proposed_patch: None, confidence: 0.88, auto_fixable: false,
                },
            },
        ]
    }
}
