use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crush_types::{Result, CrushError};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryAuth {
    pub registry: String,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub identity_token: Option<String>,
    pub token_expiry: u64,
}

#[derive(Default)]
pub struct AuthHandler {
    pub tokens: HashMap<String, RegistryAuth>,
}

impl AuthHandler {
    pub fn new() -> Self {
        Self { tokens: HashMap::new() }
    }

    pub fn save_to_disk(&self, path: &std::path::Path) -> Result<()> {
        let vec: Vec<RegistryAuth> = self.tokens.values().cloned().collect();
        let serialized = serde_json::to_string(&vec)
            .map_err(|e| CrushError::ImageError(format!("Failed to serialize auth: {}", e)))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, serialized)
            .map_err(|e| CrushError::StorageError(format!("Failed to write auth to disk: {}", e)))?;
        Ok(())
    }

    pub fn load_from_disk(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read auth: {}", e)))?;
        let vec: Vec<RegistryAuth> = serde_json::from_str(&content)
            .map_err(|e| CrushError::ImageError(format!("Failed to deserialize auth: {}", e)))?;
        
        let mut tokens = HashMap::new();
        for auth in vec {
            tokens.insert(auth.registry.clone(), auth);
        }
        Ok(Self { tokens })
    }

    pub fn detect_registry_type(registry: &str) -> &str {
        if registry.contains("docker.io") || registry.contains("registry-1.docker.io") {
            "dockerhub"
        } else if registry.contains("ghcr.io") {
            "ghcr"
        } else if registry.contains("azurecr.io") {
            // Must come before the ecr. check: "azurecr.io" contains the substring "ecr."
            "acr"
        } else if registry.contains("ecr.") || registry.contains("amazonaws.com") {
            "ecr"
        } else if registry.contains("gcr.io") || registry.contains("pkg.dev") {
            "gcr"
        } else if registry.contains("harbor") || registry.contains("core.harbor") {
            "harbor"
        } else {
            "selfhosted"
        }
    }

    pub fn get_auth_header(&self, registry: &str) -> Option<String> {
        let auth = self.tokens.get(registry)?;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        if let Some(expiry) = auth.token_expiry.checked_sub(now) {
            if expiry < 60 {
                return None;
            }
        }

        if let Some(ref token) = auth.token {
            return Some(format!("Bearer {}", token));
        }

        if let (Some(ref user), Some(ref pass)) = (&auth.username, &auth.password) {
            let encoded = BASE64.encode(format!("{}:{}", user, pass));
            return Some(format!("Basic {}", encoded));
        }

        None
    }

    pub fn store_token(&mut self, registry: &str, token: String) {
        self.store_token_with_expiry(registry, token, 3600);
    }

    pub fn store_token_with_expiry(&mut self, registry: &str, token: String, expires_in: u64) {
        let expiry = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + expires_in;
        self.tokens.insert(registry.to_string(), RegistryAuth {
            registry: registry.to_string(),
            token: Some(token),
            username: None,
            password: None,
            identity_token: None,
            token_expiry: expiry,
        });
    }

    pub async fn authenticate_dockerhub(&mut self, client: &reqwest::Client, image: &str) -> Result<String> {
        let url = format!("https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull", image);
        let resp = client.get(&url).send().await
            .map_err(|e| CrushError::ImageError(format!("Docker Hub auth failed: {}", e)))?;
        let json: serde_json::Value = resp.json().await
            .map_err(|e| CrushError::ImageError(format!("Auth response parse failed: {}", e)))?;
        let token = json["token"].as_str().unwrap_or("").to_string();
        Ok(token)
    }

    pub async fn authenticate_ghcr(&mut self, token: &str) -> Result<String> {
        Ok(format!("Bearer {}", token))
    }

    pub async fn authenticate_ecr(&mut self, _region: &str) -> Result<String> {
        Err(CrushError::ImageError("ECR auth requires AWS credentials. Use `aws ecr get-login-password` and pipe to `crush registry login`.".to_string()))
    }

    pub async fn authenticate_gcr(&mut self) -> Result<String> {
        Err(CrushError::ImageError("GCR auth requires gcloud ADC. Run `gcloud auth configure-docker` first.".to_string()))
    }

    pub async fn authenticate_acr(&mut self, registry: &str) -> Result<String> {
        let exchange_url = format!("https://{}/oauth2/exchange", registry);
        let _ = exchange_url;
        Err(CrushError::ImageError("ACR auth requires Azure Managed Identity or `az acr login`. Run `az acr login --name <registry>` first.".to_string()))
    }

    pub async fn authenticate_basic(&mut self, registry: &str, username: &str, password: &str) -> Result<String> {
        let encoded = BASE64.encode(format!("{}:{}", username, password));
        let auth = RegistryAuth {
            registry: registry.to_string(),
            token: None,
            username: Some(username.to_string()),
            password: Some(password.to_string()),
            identity_token: None,
            token_expiry: u64::MAX,
        };
        self.tokens.insert(registry.to_string(), auth);
        Ok(format!("Basic {}", encoded))
    }

    pub fn load_docker_config(&mut self, config_path: &std::path::Path) -> Result<()> {
        if !config_path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read config: {}", e)))?;
        let config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| CrushError::ImageError(format!("Invalid docker config: {}", e)))?;

        if let Some(auths) = config["auths"].as_object() {
            for (registry, creds) in auths {
                if let Some(auth_val) = creds["auth"].as_str() {
                    if !auth_val.is_empty() {
                        let decoded = String::from_utf8(
                            BASE64.decode(auth_val).unwrap_or_default()
                        ).unwrap_or_default();
                        if let Some(pos) = decoded.find(':') {
                            let user = &decoded[..pos];
                            let pass = &decoded[pos + 1..];
                            self.tokens.insert(registry.clone(), RegistryAuth {
                                registry: registry.clone(),
                                token: None,
                                username: Some(user.to_string()),
                                password: Some(pass.to_string()),
                                identity_token: None,
                                token_expiry: u64::MAX,
                            });
                        }
                    }
                }
                if let Some(token) = creds["identitytoken"].as_str() {
                    self.tokens.insert(registry.clone(), RegistryAuth {
                        registry: registry.clone(),
                        token: Some(token.to_string()),
                        username: None,
                        password: None,
                        identity_token: Some(token.to_string()),
                        token_expiry: u64::MAX,
                    });
                }
                if let Some(registry_token) = creds["registrytoken"].as_str() {
                    self.tokens.insert(registry.clone(), RegistryAuth {
                        registry: registry.clone(),
                        token: Some(registry_token.to_string()),
                        username: None,
                        password: None,
                        identity_token: None,
                        token_expiry: u64::MAX,
                    });
                }
            }
        }

        if let Some(creds_store) = config["credHelpers"].as_object() {
            for (_registry, _helper) in creds_store {
                // docker-credential-* helpers would be dispatched here
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_registry_type ──────────────────────────────────────────────────

    #[test]
    fn detect_dockerhub_variants() {
        assert_eq!(AuthHandler::detect_registry_type("docker.io"), "dockerhub");
        assert_eq!(AuthHandler::detect_registry_type("registry-1.docker.io"), "dockerhub");
    }

    #[test]
    fn detect_ghcr() {
        assert_eq!(AuthHandler::detect_registry_type("ghcr.io"), "ghcr");
    }

    #[test]
    fn detect_ecr() {
        assert_eq!(AuthHandler::detect_registry_type("123456789.dkr.ecr.us-east-1.amazonaws.com"), "ecr");
    }

    #[test]
    fn detect_gcr() {
        assert_eq!(AuthHandler::detect_registry_type("gcr.io"), "gcr");
        assert_eq!(AuthHandler::detect_registry_type("us-docker.pkg.dev"), "gcr");
    }

    #[test]
    fn detect_acr() {
        assert_eq!(AuthHandler::detect_registry_type("myregistry.azurecr.io"), "acr");
    }

    #[test]
    fn detect_selfhosted() {
        assert_eq!(AuthHandler::detect_registry_type("registry.example.com"), "selfhosted");
        assert_eq!(AuthHandler::detect_registry_type("localhost:5000"), "selfhosted");
    }

    // ── store_token / get_auth_header ─────────────────────────────────────────

    #[test]
    fn token_stored_and_retrieved_as_bearer() {
        let mut handler = AuthHandler::new();
        handler.store_token("registry-1.docker.io", "mytoken123".to_string());
        let header = handler.get_auth_header("registry-1.docker.io");
        assert_eq!(header, Some("Bearer mytoken123".to_string()));
    }

    #[test]
    fn missing_registry_returns_none() {
        let handler = AuthHandler::new();
        assert!(handler.get_auth_header("registry-1.docker.io").is_none());
    }

    #[test]
    fn basic_auth_credentials_encoded_correctly() {
        let mut handler = AuthHandler::new();
        // Simulate basic credentials stored via load_docker_config
        handler.tokens.insert("registry.example.com".to_string(), RegistryAuth {
            registry: "registry.example.com".to_string(),
            token: None,
            username: Some("alice".to_string()),
            password: Some("secret".to_string()),
            identity_token: None,
            token_expiry: u64::MAX,
        });
        let header = handler.get_auth_header("registry.example.com").unwrap();
        assert!(header.starts_with("Basic "), "expected Basic auth: {header}");
        // Decode and verify
        let encoded = header.trim_start_matches("Basic ");
        let decoded = String::from_utf8(
            base64::engine::general_purpose::STANDARD.decode(encoded).unwrap()
        ).unwrap();
        assert_eq!(decoded, "alice:secret");
    }

    // ── load_docker_config ────────────────────────────────────────────────────

    #[test]
    fn load_docker_config_parses_auth_field() {
        use base64::Engine as _;
        let encoded_creds = base64::engine::general_purpose::STANDARD.encode("bob:password123");
        let config_json = format!(r#"{{
            "auths": {{
                "ghcr.io": {{
                    "auth": "{}"
                }}
            }}
        }}"#, encoded_creds);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), config_json).unwrap();

        let mut handler = AuthHandler::new();
        handler.load_docker_config(tmp.path()).unwrap();

        let header = handler.get_auth_header("ghcr.io").unwrap();
        assert!(header.starts_with("Basic "), "expected Basic from docker config: {header}");
    }

    #[test]
    fn load_docker_config_nonexistent_path_is_ok() {
        let mut handler = AuthHandler::new();
        let result = handler.load_docker_config(std::path::Path::new("/nonexistent/config.json"));
        assert!(result.is_ok());
    }

    #[test]
    fn load_docker_config_invalid_json_returns_error() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "not valid json {{{{").unwrap();
        let mut handler = AuthHandler::new();
        let result = handler.load_docker_config(tmp.path());
        assert!(result.is_err());
    }
}
