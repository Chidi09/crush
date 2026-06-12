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
            #[cfg(unix)]
            let mut file = {
                use std::os::unix::fs::OpenOptionsExt;
                std::fs::OpenOptions::new()
                    .create(true).write(true).mode(0o600)
                    .open(&secret_path)
                    .map_err(|e| CrushError::StorageError(format!("Failed to create secret file: {}", e)))?
            };
            #[cfg(not(unix))]
            let mut file = std::fs::OpenOptions::new()
                .create(true).write(true)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_from_args_parses_key_value() {
        let mut s = BuildSecrets::new();
        s.register_from_args(&["TOKEN=abc123".to_string(), "KEY=value".to_string()]).unwrap();
        assert!(s.has_secrets());
    }

    #[test]
    fn register_from_args_skips_entries_without_equals() {
        let mut s = BuildSecrets::new();
        s.register_from_args(&["NOEQUALS".to_string()]).unwrap();
        assert!(!s.has_secrets(), "entry without '=' should be skipped");
    }

    #[test]
    fn register_from_args_value_can_contain_equals() {
        let mut s = BuildSecrets::new();
        // Value like a base64 string may contain '='
        s.register_from_args(&["CERT=abc=def=".to_string()]).unwrap();
        assert!(s.has_secrets());
    }

    #[test]
    fn has_secrets_false_when_empty() {
        let s = BuildSecrets::new();
        assert!(!s.has_secrets());
    }

    #[test]
    fn register_from_env_var() {
        std::env::set_var("CRUSH_TEST_SECRET_XYZ", "supersecret");
        let secret = crate::parser::CrushfileSecret {
            id: "mysecret".to_string(),
            src: None,
            env: Some("CRUSH_TEST_SECRET_XYZ".to_string()),
        };
        let mut s = BuildSecrets::new();
        s.register(&secret).unwrap();
        assert!(s.has_secrets());
        std::env::remove_var("CRUSH_TEST_SECRET_XYZ");
    }

    #[test]
    fn register_from_env_var_missing_returns_error() {
        std::env::remove_var("CRUSH_TEST_DEFINITELY_NOT_SET");
        let secret = crate::parser::CrushfileSecret {
            id: "missing".to_string(),
            src: None,
            env: Some("CRUSH_TEST_DEFINITELY_NOT_SET".to_string()),
        };
        let mut s = BuildSecrets::new();
        let result = s.register(&secret);
        assert!(result.is_err());
    }

    #[test]
    fn register_from_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"file-secret-data").unwrap();
        let secret = crate::parser::CrushfileSecret {
            id: "filesecret".to_string(),
            src: Some(tmp.path().to_string_lossy().to_string()),
            env: None,
        };
        let mut s = BuildSecrets::new();
        s.register(&secret).unwrap();
        assert!(s.has_secrets());
    }

    #[test]
    fn register_with_neither_src_nor_env_returns_error() {
        let secret = crate::parser::CrushfileSecret {
            id: "bad".to_string(),
            src: None,
            env: None,
        };
        let mut s = BuildSecrets::new();
        let result = s.register(&secret);
        assert!(result.is_err());
    }

    #[test]
    fn mount_secrets_creates_files_in_build_root() {
        let build_root = tempfile::TempDir::new().unwrap();
        let mut s = BuildSecrets::new();
        s.register_from_args(&["MYTOKEN=hunter2".to_string()]).unwrap();

        let mounted = s.mount_secrets(build_root.path()).unwrap();
        assert_eq!(mounted.len(), 1);
        assert!(mounted[0].exists(), "secret file should exist after mount");

        let content = std::fs::read(&mounted[0]).unwrap();
        assert_eq!(content, b"hunter2");
    }

    #[test]
    fn cleanup_removes_secrets_dir() {
        let build_root = tempfile::TempDir::new().unwrap();
        let mut s = BuildSecrets::new();
        s.register_from_args(&["FOO=bar".to_string()]).unwrap();
        s.mount_secrets(build_root.path()).unwrap();

        s.cleanup(build_root.path());
        let secrets_dir = build_root.path().join("run").join("secrets");
        assert!(!secrets_dir.exists(), "secrets dir should be cleaned up");
    }
}
