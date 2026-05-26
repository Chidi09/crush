use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone)]
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
    tokens: HashMap<String, RegistryAuth>,
}

impl AuthHandler {
    pub fn new() -> Self {
        Self { tokens: HashMap::new() }
    }

    pub fn detect_registry_type(registry: &str) -> &str {
        if registry.contains("docker.io") || registry.contains("registry-1.docker.io") {
            "dockerhub"
        } else if registry.contains("ghcr.io") {
            "ghcr"
        } else if registry.contains("ecr.") || registry.contains("amazonaws.com") {
            "ecr"
        } else if registry.contains("gcr.io") || registry.contains("pkg.dev") {
            "gcr"
        } else if registry.contains("azurecr.io") {
            "acr"
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
            let encoded = base64::encode(format!("{}:{}", user, pass));
            return Some(format!("Basic {}", encoded));
        }

        None
    }

    pub fn store_token(&mut self, registry: &str, token: String) {
        let expiry = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + 3600;
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
        let encoded = base64::encode(format!("{}:{}", username, password));
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
                            base64::decode(auth_val).unwrap_or_default()
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
