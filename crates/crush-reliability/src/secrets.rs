use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use zeroize::Zeroize;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSpec {
    pub id: String,
    pub source: SecretSource,
    pub destination: SecretDestination,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretSource {
    Env { key: String },
    File { path: PathBuf },
    Vault { path: String, field: String, engine: VaultEngine },
    AwsSsm { name: String, with_decryption: bool },
    AwsSecretsManager { secret_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VaultEngine {
    KvV1,
    KvV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretDestination {
    Env { var_name: String },
    File { path: PathBuf, tmpfs: bool },
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SecretValue {
    inner: Vec<u8>,
}

impl SecretValue {
    pub fn new(data: Vec<u8>) -> Self { Self { inner: data } }
    pub fn as_bytes(&self) -> &[u8] { &self.inner }
    pub fn as_str(&self) -> &str { std::str::from_utf8(&self.inner).unwrap_or("") }
}

pub struct SecretManager {
    vault_addr: Option<String>,
    vault_token: Option<String>,
    secrets_dir: PathBuf,
}

impl SecretManager {
    pub fn new(secrets_dir: PathBuf) -> Self {
        Self { vault_addr: None, vault_token: None, secrets_dir }
    }

    pub fn with_vault(mut self, addr: String, token: String) -> Self {
        self.vault_addr = Some(addr);
        self.vault_token = Some(token);
        self
    }

    pub async fn resolve(&self, spec: &SecretSpec) -> Result<SecretValue> {
        match &spec.source {
            SecretSource::Env { key } => {
                let val = std::env::var(key)
                    .map_err(|_| CrushError::StorageError(format!("Env var '{}' not set", key)))?;
                Ok(SecretValue::new(val.into_bytes()))
            }
            SecretSource::File { path } => {
                let data = tokio::fs::read(path).await
                    .map_err(|e| CrushError::StorageError(format!("Secret file error: {}", e)))?;
                Ok(SecretValue::new(data))
            }
            SecretSource::Vault { path, field, engine: _ } => {
                self.read_vault(path, field).await
            }
            SecretSource::AwsSsm { name, with_decryption } => {
                self.read_ssm(name, *with_decryption).await
            }
            SecretSource::AwsSecretsManager { secret_id } => {
                self.read_secrets_manager(secret_id).await
            }
        }
    }

    pub async fn mount(&self, spec: &SecretSpec, value: &SecretValue) -> Result<PathBuf> {
        match &spec.destination {
            SecretDestination::Env { var_name } => {
                if let Ok(val) = std::str::from_utf8(value.as_bytes()) {
                    std::env::set_var(var_name, val);
                }
                Ok(PathBuf::new())
            }
            SecretDestination::File { path, tmpfs } => {
                let dir = if *tmpfs {
                    self.secrets_dir.clone()
                } else {
                    path.parent().unwrap_or(&self.secrets_dir).to_path_buf()
                };
                tokio::fs::create_dir_all(&dir).await
                    .map_err(|e| CrushError::StorageError(e.to_string()))?;

                tokio::fs::write(path, value.as_bytes()).await
                    .map_err(|e| CrushError::StorageError(e.to_string()))?;

                // Set permissions to 0400
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o400);
                    std::fs::set_permissions(path, perms).ok();
                }

                Ok(path.clone())
            }
        }
    }

    async fn read_vault(&self, path: &str, field: &str) -> Result<SecretValue> {
        let addr = self.vault_addr.as_ref()
            .ok_or_else(|| CrushError::StorageError("Vault address not configured".to_string()))?;
        let token = self.vault_token.as_ref()
            .ok_or_else(|| CrushError::StorageError("Vault token not configured".to_string()))?;

        let url = format!("{}/v1/{}", addr.trim_end_matches('/'), path);
        let client = reqwest::Client::new();
        let resp = client.get(&url)
            .header("X-Vault-Token", token)
            .send().await
            .map_err(|e| CrushError::StorageError(format!("Vault request failed: {}", e)))?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| CrushError::StorageError(format!("Vault response parse: {}", e)))?;

        let value = json["data"]["data"][field].as_str()
            .or_else(|| json["data"][field].as_str())
            .ok_or_else(|| CrushError::StorageError(format!("Field '{}' not found in Vault path '{}'", field, path)))?;

        Ok(SecretValue::new(value.as_bytes().to_vec()))
    }

    async fn read_ssm(&self, name: &str, with_decryption: bool) -> Result<SecretValue> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "Name": name,
            "WithDecryption": with_decryption,
        });
        let resp = client.post("http://localhost:2773/systemsmanager/parameters/get/")
            .header("X-Aws-Parameter-Secrets-Token", "")
            .json(&body)
            .send().await
            .map_err(|e| CrushError::StorageError(format!("SSM request failed: {}", e)))?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| CrushError::StorageError(format!("SSM response: {}", e)))?;
        let value = json["Parameter"]["Value"].as_str()
            .ok_or_else(|| CrushError::StorageError("SSM parameter value not found".to_string()))?;

        Ok(SecretValue::new(value.as_bytes().to_vec()))
    }

    async fn read_secrets_manager(&self, secret_id: &str) -> Result<SecretValue> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({ "SecretId": secret_id });
        let resp = client.post("http://localhost:2773/secretsmanager/get/")
            .json(&body).send().await
            .map_err(|e| CrushError::StorageError(format!("Secrets Manager request: {}", e)))?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| CrushError::StorageError(format!("Secrets Manager response: {}", e)))?;
        let value = json["SecretString"].as_str()
            .ok_or_else(|| CrushError::StorageError("Secret string not found".to_string()))?;

        Ok(SecretValue::new(value.as_bytes().to_vec()))
    }
}
