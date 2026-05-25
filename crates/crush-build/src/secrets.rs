use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crush_types::{Result, CrushError};
use crate::parser::CrushfileSecret;

pub struct BuildSecrets {
    secrets: HashMap<String, BuildSecret>,
    mount_dir: PathBuf,
}

struct BuildSecret {
    value: Vec<u8>,
    from_env: bool,
}

impl BuildSecrets {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
            mount_dir: PathBuf::from("/run/secrets"),
        }
    }

    pub fn register(&mut self, secret: &CrushfileSecret) -> Result<()> {
        let value = if let Some(ref src) = secret.src {
            std::fs::read(Path::new(src))
                .map_err(|e| CrushError::StorageError(format!(
                    "Failed to read secret file '{}': {}", src, e
                )))?
        } else if let Some(ref env_var) = secret.env {
            std::env::var(env_var)
                .map(|v| v.into_bytes())
                .map_err(|_| CrushError::ImageError(format!(
                    "Environment variable '{}' not set for secret", env_var
                )))?
        } else {
            return Err(CrushError::ImageError(format!(
                "Secret '{}' has no src or env field", secret.id
            )));
        };

        self.secrets.insert(secret.id.clone(), BuildSecret {
            value,
            from_env: secret.env.is_some(),
        });
        Ok(())
    }

    pub fn register_from_args(&mut self, args: &[String]) -> Result<()> {
        for arg in args {
            if let Some(eq) = arg.find('=') {
                let key = &arg[..eq];
                let val = &arg[eq + 1..];
                self.secrets.insert(key.to_string(), BuildSecret {
                    value: val.as_bytes().to_vec(),
                    from_env: false,
                });
            }
        }
        Ok(())
    }

    pub fn mount_secrets(&self, build_root: &Path) -> Result<Vec<PathBuf>> {
        let mut mounted = Vec::new();
        let secrets_dir = build_root.join(
            self.mount_dir.strip_prefix("/").unwrap_or(&self.mount_dir)
        );
        std::fs::create_dir_all(&secrets_dir)
            .map_err(|e| CrushError::StorageError(format!("Failed to create secrets dir: {}", e)))?;

        for (id, secret) in &self.secrets {
            let secret_path = secrets_dir.join(id);
            // ⚠ FIX: Use 0o600 for secrets to prevent world-readable build artifacts
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .mode(0o600)
                .open(&secret_path)
                .map_err(|e| CrushError::StorageError(format!("Failed to create secret file: {}", e)))?;
            use std::io::Write;
            file.write_all(&secret.value)
                .map_err(|e| CrushError::StorageError(format!("Failed to write secret: {}", e)))?;
            mounted.push(secret_path);
        }

        Ok(mounted)
    }

    pub fn cleanup(&self, build_root: &Path) {
        let secrets_dir = build_root.join(
            self.mount_dir.strip_prefix("/").unwrap_or(&self.mount_dir)
        );
        if secrets_dir.exists() {
            let _ = std::fs::remove_dir_all(&secrets_dir);
        }
    }

    pub fn has_secrets(&self) -> bool {
        !self.secrets.is_empty()
    }
}
