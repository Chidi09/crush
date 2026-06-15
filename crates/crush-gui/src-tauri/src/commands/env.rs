use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

#[tauri::command]
pub async fn read_env(project_path: String) -> Result<Vec<EnvVar>, String> {
    let root = PathBuf::from(&project_path);
    let env_path = root.join(".env");
    
    let mut vars = Vec::new();
    
    if env_path.exists() {
        let content = fs::read_to_string(&env_path).map_err(|e| e.to_string())?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_string();
                let value = v.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                let upper = key.to_uppercase();
                let is_secret = upper.contains("SECRET")
                    || upper.contains("PASSWORD")
                    || upper.contains("TOKEN")
                    || upper.contains("KEY")
                    || upper.contains("PASS")
                    || upper.contains("URL")
                    || upper.contains("URI");
                vars.push(EnvVar { key, value, is_secret });
            }
        }
    }
    
    Ok(vars)
}

#[tauri::command]
pub async fn write_env(project_path: String, env: Vec<EnvVar>) -> Result<(), String> {
    let root = PathBuf::from(&project_path);
    let env_path = root.join(".env");
    
    let mut lines = Vec::new();
    let mut existing_keys = std::collections::HashSet::new();
    
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    lines.push(line.to_string());
                    continue;
                }
                if let Some((k, _)) = trimmed.split_once('=') {
                    let key = k.trim();
                    if let Some(new_var) = env.iter().find(|v| v.key == key) {
                        lines.push(format!("{}={}", new_var.key, new_var.value));
                        existing_keys.insert(key.to_string());
                    } else {
                        // Key removed in UI
                    }
                } else {
                    lines.push(line.to_string());
                }
            }
        }
    }
    
    for var in env {
        if !existing_keys.contains(&var.key) {
            lines.push(format!("{}={}", var.key, var.value));
        }
    }
    
    fs::write(&env_path, lines.join("\n")).map_err(|e| e.to_string())?;
    
    Ok(())
}
