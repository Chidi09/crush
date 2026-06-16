use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedRuntime {
    Node,
    Python,
    Java,
    Go,
}

impl SupportedRuntime {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Python => "python",
            Self::Java => "java",
            Self::Go => "go",
        }
    }
}

pub fn detect_version(root: &Path, runtime: SupportedRuntime) -> Option<String> {
    match runtime {
        SupportedRuntime::Node => {
            if let Ok(content) = std::fs::read_to_string(root.join(".nvmrc")) {
                return Some(content.trim().to_string());
            }
            if let Ok(content) = std::fs::read_to_string(root.join(".node-version")) {
                return Some(content.trim().to_string());
            }
            if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(engines) = json.get("engines") {
                        if let Some(node) = engines.get("node") {
                            return Some(node.as_str().unwrap_or_default().to_string());
                        }
                    }
                }
            }
        }
        SupportedRuntime::Python => {
            if let Ok(content) = std::fs::read_to_string(root.join(".python-version")) {
                return Some(content.trim().to_string());
            }
        }
        SupportedRuntime::Java => {
            if let Ok(content) = std::fs::read_to_string(root.join(".sdkmanrc")) {
                for line in content.lines() {
                    if line.starts_with("java=") {
                        return Some(line.replace("java=", "").trim().to_string());
                    }
                }
            }
        }
        SupportedRuntime::Go => {
            if let Ok(content) = std::fs::read_to_string(root.join("go.mod")) {
                for line in content.lines() {
                    if line.starts_with("go ") {
                        return Some(line.replace("go ", "").trim().to_string());
                    }
                }
            }
        }
    }

    if let Ok(content) = std::fs::read_to_string(root.join(".tool-versions")) {
        let prefix = format!("{} ", runtime.as_str());
        for line in content.lines() {
            if line.starts_with(&prefix) {
                return Some(line.replace(&prefix, "").trim().to_string());
            }
        }
    }

    None
}
