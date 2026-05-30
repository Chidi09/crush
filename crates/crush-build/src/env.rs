use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalService {
    pub name: String,
    pub kind: String,
    pub source_var: String,
}

pub fn scan_external_services(root: &Path) -> Vec<ExternalService> {
    let mut services = Vec::new();
    let env_files = [".env", ".env.local", ".env.development", ".env.production", ".env.staging", ".env.example"];
    for file in &env_files {
        let path = root.join(file);
        if !path.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let mut parts = line.split('=');
                let key = parts.next().unwrap_or("").trim();
                let value = parts.next().unwrap_or("").trim();
                if key.is_empty() {
                    continue;
                }
                
                let key_upper = key.to_uppercase();
                if key_upper == "SUPABASE_URL" || key_upper.starts_with("NEXT_PUBLIC_SUPABASE_") {
                    if !services.iter().any(|s: &ExternalService| s.name == "Supabase") {
                        services.push(ExternalService {
                            name: "Supabase".to_string(),
                            kind: "hosted".to_string(),
                            source_var: key.to_string(),
                        });
                    }
                } else if key_upper.starts_with("UPSTASH_") {
                    if !services.iter().any(|s: &ExternalService| s.name == "Upstash") {
                        services.push(ExternalService {
                            name: "Upstash".to_string(),
                            kind: "hosted".to_string(),
                            source_var: key.to_string(),
                        });
                    }
                } else if key_upper.contains("FIREBASE") {
                    if !services.iter().any(|s: &ExternalService| s.name == "Firebase") {
                        services.push(ExternalService {
                            name: "Firebase".to_string(),
                            kind: "hosted".to_string(),
                            source_var: key.to_string(),
                        });
                    }
                } else if key_upper.starts_with("CLERK_") || key_upper.starts_with("NEXT_PUBLIC_CLERK_") {
                    if !services.iter().any(|s: &ExternalService| s.name == "Clerk") {
                        services.push(ExternalService {
                            name: "Clerk".to_string(),
                            kind: "hosted".to_string(),
                            source_var: key.to_string(),
                        });
                    }
                } else if key_upper.starts_with("AUTH0_") {
                    if !services.iter().any(|s: &ExternalService| s.name == "Auth0") {
                        services.push(ExternalService {
                            name: "Auth0".to_string(),
                            kind: "hosted".to_string(),
                            source_var: key.to_string(),
                        });
                    }
                } else if key_upper == "DATABASE_URL" || key_upper == "MONGODB_URI" || key_upper == "REDIS_URL" || key_upper == "MYSQL_URL" {
                    let val_lower = value.to_lowercase();
                    let (name, kind) = if val_lower.starts_with("postgres") || val_lower.starts_with("postgresql") {
                        ("PostgreSQL".to_string(), "external".to_string())
                    } else if val_lower.starts_with("mongodb") {
                        ("MongoDB".to_string(), "external".to_string())
                    } else if val_lower.starts_with("redis") {
                        ("Redis".to_string(), "external".to_string())
                    } else if val_lower.starts_with("mysql") {
                        ("MySQL".to_string(), "external".to_string())
                    } else {
                        continue;
                    };
                    
                    if !services.iter().any(|s: &ExternalService| s.name == name) {
                        services.push(ExternalService {
                            name,
                            kind,
                            source_var: key.to_string(),
                        });
                    }
                }
            }
        }
    }
    services
}

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
