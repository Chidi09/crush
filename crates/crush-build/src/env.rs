use std::path::Path;
use std::fs;

pub struct EnvDetector;

impl EnvDetector {
    pub fn scan(root: &Path) -> EnvScanResult {
        let mut required = Vec::new();
        let mut optional = Vec::new();
        let mut secrets = Vec::new();

        for name in [".env.example", ".env.sample", ".env.template", ".env.dist"] {
            let dotenv_path = root.join(name);
            if !dotenv_path.exists() { continue; }
            if let Ok(content) = fs::read_to_string(&dotenv_path) {
                for line in content.lines() {
                    let line = line.trim().trim_start_matches("export ").trim_start();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let key = line.split('=').next().unwrap_or("").trim().to_string();
                    if key.is_empty() {
                        continue;
                    }

                    let upper = key.to_uppercase();
                    let is_secret = upper.contains("SECRET")
                        || upper.contains("PASSWORD")
                        || upper.contains("TOKEN")
                        || upper.contains("KEY")
                        || upper.contains("PASS");

                    // `PORT=3000` ships a usable default → optional.
                    // `DATABASE_URL=` (empty value) must be provided → required.
                    let value = line.split_once('=').map(|(_, v)| v.trim()).unwrap_or("");
                    let has_default = !value.is_empty();

                    if is_secret {
                        if !secrets.contains(&key) { secrets.push(key); }
                    } else if has_default {
                        if !optional.contains(&key) { optional.push(key); }
                    } else if !required.contains(&key) {
                        required.push(key);
                    }
                }
            }
            break; // first example file present wins
        }

        let mut scan_res = EnvScanResult { required, optional, secrets };
        Self::scan_code_for_envs(root, &mut scan_res);
        scan_res.required.sort();
        scan_res.optional.sort();
        scan_res.secrets.sort();
        scan_res
    }

    fn scan_code_for_envs(root: &Path, result: &mut EnvScanResult) {
        let mut paths_to_scan = Vec::new();
        Self::collect_source_files(root, &mut paths_to_scan, 0);

        let process_env_re = regex::Regex::new(r"\bprocess\.env\.([A-Za-z0-9_]+)\b").unwrap();
        let os_environ_re = regex::Regex::new(r#"\bos\.environ(?:\[|(?:\.get\())\s*['"]([A-Za-z0-9_]+)['"]"#).unwrap();
        let os_getenv_re = regex::Regex::new(r#"\bos\.getenv\(\s*['"]([A-Za-z0-9_]+)['"]"#).unwrap();
        let std_env_re = regex::Regex::new(r#"\bstd::env::var\(\s*['"]([A-Za-z0-9_]+)['"]"#).unwrap();
        let system_getenv_re = regex::Regex::new(r#"\bSystem\.getenv\(\s*['"]([A-Za-z0-9_]+)['"]"#).unwrap();
        let dotnet_env_re = regex::Regex::new(r#"\bEnvironment\.GetEnvironmentVariable\(\s*['"]([A-Za-z0-9_]+)['"]"#).unwrap();
        let go_getenv_re = regex::Regex::new(r#"\bos\.Getenv\(\s*['"]([A-Za-z0-9_]+)['"]"#).unwrap();

        let common_ignores = [
            "NODE_ENV", "PORT", "HOST", "PATH", "PWD", "HOME", "USER", "SHELL", "LANG", "DISPLAY"
        ];

        for path in paths_to_scan {
            if let Ok(content) = fs::read_to_string(&path) {
                let regexes = [
                    &process_env_re,
                    &os_environ_re,
                    &os_getenv_re,
                    &std_env_re,
                    &system_getenv_re,
                    &dotnet_env_re,
                    &go_getenv_re,
                ];

                for re in &regexes {
                    for caps in re.captures_iter(&content) {
                        let key = caps[1].to_string();
                        if common_ignores.contains(&key.as_str()) {
                            continue;
                        }
                        let upper = key.to_uppercase();
                        let is_secret = upper.contains("SECRET")
                            || upper.contains("PASSWORD")
                            || upper.contains("TOKEN")
                            || upper.contains("KEY")
                            || upper.contains("PASS");

                        if is_secret {
                            if !result.secrets.contains(&key) {
                                result.secrets.push(key);
                            }
                        } else {
                            if !result.required.contains(&key) && !result.optional.contains(&key) && !result.secrets.contains(&key) {
                                result.required.push(key);
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_source_files(dir: &Path, result: &mut Vec<std::path::PathBuf>, depth: usize) {
        if depth > 10 { return; }
        let read_dir = match fs::read_dir(dir) {
            Ok(d) => d,
            Err(_) => return,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if name.starts_with('.') && name != "." && name != ".." { continue; }
                if matches!(name.as_ref(),
                    "node_modules" | "target" | "dist" | "build" | "venv" | ".venv" |
                    "__pycache__" | "obj" | "bin" | ".gradle" | "vendor" | "deps" |
                    "_build" | "out" | ".git" | ".cache"
                ) {
                    continue;
                }
                Self::collect_source_files(&path, result, depth + 1);
            } else if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "js" | "ts" | "jsx" | "tsx" | "py" | "rs" | "java" | "cs" | "go" | "rb" | "php") {
                        result.push(path);
                    }
                }
            }
        }
    }
}

pub struct EnvScanResult {
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub secrets: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_with(name: &str, content: &str) -> EnvScanResult {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(dir.path().join(name), content).unwrap();
        EnvDetector::scan(dir.path())
    }

    #[test]
    fn value_with_default_is_optional_empty_is_required() {
        let r = scan_with(".env.example", "PORT=3000\nDATABASE_URL=\n");
        assert_eq!(r.optional, vec!["PORT"], "a shipped default means optional");
        assert_eq!(r.required, vec!["DATABASE_URL"], "empty value means user must provide it");
    }

    #[test]
    fn secret_like_names_are_classified_secret() {
        let r = scan_with(".env.example", "API_KEY=\nSESSION_SECRET=abc\nSMTP_PASSWORD=\n");
        assert_eq!(r.secrets, vec!["API_KEY", "SESSION_SECRET", "SMTP_PASSWORD"]);
        assert!(r.required.is_empty() && r.optional.is_empty());
    }

    #[test]
    fn reads_env_sample_and_template_variants() {
        for name in [".env.sample", ".env.template", ".env.dist"] {
            let r = scan_with(name, "REDIS_HOST=\n");
            assert_eq!(r.required, vec!["REDIS_HOST"], "should read {}", name);
        }
    }

    #[test]
    fn export_prefix_is_stripped() {
        let r = scan_with(".env.example", "export NODE_ENV=production\nexport APP_URL=\n");
        assert_eq!(r.optional, vec!["NODE_ENV"]);
        assert_eq!(r.required, vec!["APP_URL"]);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let r = scan_with(".env.example", "# comment\n\nHOST=\n");
        assert_eq!(r.required, vec!["HOST"]);
    }

    #[test]
    fn scans_code_for_env_vars() {
        let dir = tempfile::TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        
        fs::write(src_dir.join("app.js"), "const db = process.env.DATABASE_URL;\nconst port = process.env.PORT;\nconst apiKey = process.env.API_KEY;").unwrap();
        fs::write(src_dir.join("main.py"), "import os\nsecret = os.environ['STRIPE_SECRET']\nval = os.getenv('CONFIG_PATH')").unwrap();
        fs::write(src_dir.join("lib.rs"), "let api_token = std::env::var(\"API_TOKEN\");").unwrap();
        
        let r = EnvDetector::scan(dir.path());
        assert_eq!(r.required, vec!["CONFIG_PATH", "DATABASE_URL"]);
        assert_eq!(r.secrets, vec!["API_KEY", "API_TOKEN", "STRIPE_SECRET"]);
    }
}
