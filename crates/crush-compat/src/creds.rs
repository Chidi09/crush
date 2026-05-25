use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct DockerCredentialHelper;

impl DockerCredentialHelper {
    pub fn resolve(registry: &str) -> Result<Option<Credential>> {
        let config = Self::load_docker_config()?;

        if let Some(creds) = config.auths.get(registry).or_else(|| config.auths.get("https://index.docker.io/v1/")) {
            if let Some(auth) = &creds.auth {
                let decoded = String::from_utf8(
                    base64_decode(auth).unwrap_or_default()
                ).unwrap_or_default();
                if let Some(pos) = decoded.find(':') {
                    return Ok(Some(Credential {
                        username: decoded[..pos].to_string(),
                        password: decoded[pos + 1..].to_string(),
                        registry: registry.to_string(),
                    }));
                }
            }
        }

        if let Some(helper_name) = config.cred_helpers.get(registry)
            .or_else(|| config.cred_helpers.get("")) {
            return Self::call_helper(helper_name, registry);
        }

        Ok(None)
    }

    pub fn load_docker_config() -> Result<DockerConfig> {
        let paths = vec![
            Self::config_path(),
            PathBuf::from("/etc/docker/config.json"),
        ];

        for path in &paths {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(config) = serde_json::from_str::<DockerConfig>(&content) {
                        return Ok(config);
                    }
                }
            }
        }

        Ok(DockerConfig::default())
    }

    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".docker").join("config.json")
    }

    fn call_helper(helper: &str, registry: &str) -> Result<Option<Credential>> {
        let helper_bin = format!("docker-credential-{}", helper);
        let out = Command::new(&helper_bin)
            .arg("get")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn();

        match out {
            Ok(mut child) => {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(registry.as_bytes()).ok();
                }
                if let Ok(output) = child.wait_with_output() {
                    if output.status.success() {
                        if let Ok(cred) = serde_json::from_slice::<HelperResponse>(&output.stdout) {
                            return Ok(Some(Credential {
                                username: cred.username,
                                password: cred.secret,
                                registry: registry.to_string(),
                            }));
                        }
                    }
                }
                Ok(None)
            }
            Err(_) => {
                if helper == "desktop" || helper == "osxkeychain" || helper == "ecr-login" || helper == "gcr" {
                    eprintln!("Note: {} not found. Install it or configure credentials in ~/.docker/config.json", helper_bin);
                }
                Ok(None)
            }
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Default)]
pub struct DockerConfig {
    #[serde(default)]
    pub auths: HashMap<String, DockerAuthEntry>,
    #[serde(default)]
    pub cred_helpers: HashMap<String, String>,
    #[serde(default)]
    pub creds_store: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DockerAuthEntry {
    pub auth: Option<String>,
    pub identity_token: Option<String>,
    pub registry_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct HelperResponse {
    pub username: String,
    pub secret: String,
}

#[derive(Debug, Clone)]
pub struct Credential {
    pub username: String,
    pub password: String,
    pub registry: String,
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(input).ok()
}
