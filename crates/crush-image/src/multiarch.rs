use crush_types::{Result, CrushError};

#[derive(Debug, Clone)]
pub struct Platform {
    pub os: String,
    pub architecture: String,
    pub variant: Option<String>,
    pub os_version: Option<String>,
    pub os_features: Vec<String>,
    pub features: Vec<String>,
}

impl Platform {
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            variant: None,
            os_version: None,
            os_features: Vec::new(),
            features: Vec::new(),
        }
    }

    pub fn matches(&self, other: &Platform) -> bool {
        if self.os != other.os {
            return false;
        }
        if self.architecture != other.architecture {
            if self.architecture == "aarch64" && other.architecture == "arm64" {
            } else if self.architecture == "x86_64" && other.architecture == "amd64" {
            } else {
                return false;
            }
        }
        if let Some(ref my_var) = self.variant {
            if let Some(ref other_var) = other.variant {
                if my_var != other_var {
                    return false;
                }
            }
        }
        true
    }

    pub fn from_manifest_entry(entry: &serde_json::Value) -> Self {
        Self {
            os: entry["platform"]["os"].as_str().unwrap_or("linux").to_string(),
            architecture: entry["platform"]["architecture"].as_str().unwrap_or("amd64").to_string(),
            variant: entry["platform"]["variant"].as_str().map(|s| s.to_string()),
            os_version: entry["platform"]["os.version"].as_str().map(|s| s.to_string()),
            os_features: entry["platform"]["os.features"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            features: entry["platform"]["features"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
        }
    }
}

pub struct MultiArchResolver;

impl MultiArchResolver {
    pub fn resolve_manifest<'a>(manifest_list: &'a serde_json::Value) -> Result<&'a serde_json::Value> {
        let manifests = manifest_list["manifests"].as_array()
            .ok_or_else(|| CrushError::ImageError("Invalid manifest list: no manifests array".to_string()))?;

        let host = Platform::current();

        let mut best: Option<(usize, &serde_json::Value)> = None;
        let mut best_score = -1i32;

        for (i, entry) in manifests.iter().enumerate() {
            let plat = Platform::from_manifest_entry(entry);
            if plat.os != host.os {
                continue;
            }
            if !plat.architecture.contains(&host.architecture)
                && !host.architecture.contains(&plat.architecture)
            {
                continue;
            }
            let score = if plat.architecture == host.architecture { 2 } else { 1 };
            if plat.variant.is_some() {
                // prefer exact variant match
            }
            if score > best_score {
                best_score = score;
                best = Some((i, entry));
            }
        }

        if let Some((_, entry)) = best {
            let digest = entry["digest"].as_str()
                .ok_or_else(|| CrushError::ImageError("Manifest entry missing digest".to_string()))?;
            // The caller uses this digest to fetch the platform-specific manifest
            return Ok(entry);
        }

        Err(CrushError::ImageError(format!(
            "No matching platform manifest for {}/{}",
            host.os, host.architecture
        )))
    }

    pub fn select_platform_digest(manifest_list: &serde_json::Value) -> Result<String> {
        let entry = Self::resolve_manifest(manifest_list)?;
        entry["digest"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CrushError::ImageError("Missing digest in manifest list entry".to_string()))
    }
}
