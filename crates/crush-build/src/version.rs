use std::path::Path;
use std::fs;

pub struct VersionResolver;

impl VersionResolver {
    pub fn resolve(root: &Path, manifest_version: Option<&str>) -> String {
        let sources = [
            Self::from_nvmrc(root),
            Self::from_node_version(root),
            Self::from_tool_versions(root),
            Self::from_python_version(root),
            Self::from_ruby_version(root),
            Self::from_go_version(root),
            Self::from_java_version(root),
            Self::from_rust_toolchain(root),
        ];

        for source in &sources {
            if let Some(version) = source {
                return version.clone();
            }
        }

        manifest_version.unwrap_or("latest").to_string()
    }

    pub fn from_nvmrc(root: &Path) -> Option<String> {
        let path = root.join(".nvmrc");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        Some(content.trim().trim_start_matches('v').to_string())
    }

    pub fn from_node_version(root: &Path) -> Option<String> {
        let path = root.join(".node-version");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        Some(content.trim().trim_start_matches('v').to_string())
    }

    pub fn from_tool_versions(root: &Path) -> Option<String> {
        let path = root.join(".tool-versions");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "nodejs" | "node" => return Some(parts[1].trim_start_matches('v').to_string()),
                    "python" | "rust" | "golang" | "ruby" | "java" => {
                        return Some(parts[1].trim_start_matches('v').to_string());
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub fn from_python_version(root: &Path) -> Option<String> {
        let path = root.join(".python-version");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        Some(content.trim().to_string())
    }

    pub fn from_ruby_version(root: &Path) -> Option<String> {
        let path = root.join(".ruby-version");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        Some(content.trim().to_string())
    }

    pub fn from_go_version(root: &Path) -> Option<String> {
        let path = root.join(".go-version");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        Some(content.trim().trim_start_matches('v').to_string())
    }

    pub fn from_java_version(root: &Path) -> Option<String> {
        let path = root.join(".java-version");
        if !path.exists() { return None; }
        let content = fs::read_to_string(path).ok()?;
        Some(content.trim().to_string())
    }

    pub fn from_rust_toolchain(root: &Path) -> Option<String> {
        let path = root.join("rust-toolchain.toml");
        if path.exists() {
            let content = fs::read_to_string(path).ok()?;
            if let Ok(parsed) = toml::from_str::<serde_json::Value>(&content) {
                if let Some(channel) = parsed["toolchain"]["channel"].as_str() {
                    return Some(channel.to_string());
                }
            }
        }
        let legacy = root.join("rust-toolchain");
        if legacy.exists() {
            let content = fs::read_to_string(legacy).ok()?;
            return Some(content.trim().to_string());
        }
        None
    }
}
