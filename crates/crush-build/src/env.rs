use std::path::Path;
use std::fs;

pub struct EnvDetector;

impl EnvDetector {
    pub fn scan(root: &Path) -> EnvScanResult {
        let mut required = Vec::new();
        let mut optional = Vec::new();
        let mut secrets = Vec::new();

        let dotenv_path = root.join(".env.example");
        if dotenv_path.exists() {
            if let Ok(content) = fs::read_to_string(&dotenv_path) {
                for line in content.lines() {
                    let line = line.trim();
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

                    let has_default = line.contains('=') && line.trim_end().ends_with('=');

                    if is_secret {
                        secrets.push(key);
                    } else if has_default {
                        optional.push(key);
                    } else {
                        required.push(key);
                    }
                }
            }
        }

        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(content) = fs::read_to_string(&gitignore_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line == ".env" || line == "*.env" || line == ".env.*" {
                        if required.is_empty() && optional.is_empty() {
                            required.push("DATABASE_URL".to_string());
                            optional.push("PORT".to_string());
                        }
                        break;
                    }
                }
            }
        }

        EnvScanResult { required, optional, secrets }
    }
}

pub struct EnvScanResult {
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub secrets: Vec<String>,
}
