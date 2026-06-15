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
        // Normalize Rust's arch names to OCI/Docker convention
        let arch = match std::env::consts::ARCH {
            "x86_64"  => "amd64",
            "aarch64" => "arm64",
            "arm"     => "arm",
            other     => other,
        };
        Self {
            os: std::env::consts::OS.to_string(),
            architecture: arch.to_string(),
            variant: None,
            os_version: None,
            os_features: Vec::new(),
            features: Vec::new(),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() >= 2 {
            let os = parts[0].to_string();
            let arch = match parts[1] {
                "amd64" | "x86_64" => "amd64",
                "arm64" | "aarch64" => "arm64",
                other => other,
            };
            let variant = parts.get(2).map(|s| s.to_string());
            Some(Self {
                os,
                architecture: arch.to_string(),
                variant,
                os_version: None,
                os_features: Vec::new(),
                features: Vec::new(),
            })
        } else {
            None
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
        Self::resolve_for_host(manifests, &host).ok_or_else(|| CrushError::ImageError(format!(
            "No matching platform manifest for {}/{} (also tried linux/{})",
            host.os, host.architecture, host.architecture
        )))
    }

    /// Pick the best manifest entry for `host`, falling back to `linux/<arch>`
    /// when the host isn't Linux. On Windows/macOS most service images ship only
    /// linux manifests, and the linux/amd64 image is exactly what the user wants
    /// (crush exports it / runs it under Linux execution), so a missing native
    /// manifest shouldn't be a hard error. Pure over inputs for testability.
    fn resolve_for_host<'a>(
        manifests: &'a [serde_json::Value],
        host: &Platform,
    ) -> Option<&'a serde_json::Value> {
        let mut targets = vec![host.clone()];
        if host.os != "linux" {
            targets.push(Platform {
                os: "linux".to_string(),
                architecture: host.architecture.clone(),
                variant: None,
                os_version: None,
                os_features: Vec::new(),
                features: Vec::new(),
            });
        }
        for target in &targets {
            let mut best: Option<&serde_json::Value> = None;
            let mut best_score = -1i32;
            for entry in manifests.iter() {
                let plat = Platform::from_manifest_entry(entry);
                if !target.matches(&plat) {
                    continue;
                }
                let score = if plat.architecture == target.architecture { 2 } else { 1 };
                if score > best_score {
                    best_score = score;
                    best = Some(entry);
                }
            }
            if best.is_some() {
                return best;
            }
        }
        None
    }

    pub fn select_platform_digest(manifest_list: &serde_json::Value) -> Result<String> {
        let entry = Self::resolve_manifest(manifest_list)?;
        entry["digest"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| CrushError::ImageError("Missing digest in manifest list entry".to_string()))
    }

    pub fn resolve_manifest_with_platform<'a>(
        manifest_list: &'a serde_json::Value,
        preferred_platform: &Platform,
    ) -> Result<&'a serde_json::Value> {
        let manifests = manifest_list["manifests"].as_array()
            .ok_or_else(|| CrushError::ImageError("Invalid manifest list: no manifests array".to_string()))?;

        for entry in manifests {
            let plat = Platform::from_manifest_entry(entry);
            if preferred_platform.os == plat.os && preferred_platform.architecture == plat.architecture {
                if preferred_platform.variant.is_none() || preferred_platform.variant == plat.variant {
                    return Ok(entry);
                }
            }
        }

        Err(CrushError::ImageError(format!(
            "No matching platform manifest for preferred platform {}/{}",
            preferred_platform.os, preferred_platform.architecture
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win() -> Platform {
        Platform { os: "windows".into(), architecture: "amd64".into(), variant: None,
            os_version: None, os_features: vec![], features: vec![] }
    }
    fn linux() -> Platform {
        Platform { os: "linux".into(), architecture: "amd64".into(), variant: None,
            os_version: None, os_features: vec![], features: vec![] }
    }

    // A linux-only image (FlareSolverr-style) must resolve on a Windows host via
    // the linux/amd64 fallback instead of erroring.
    fn linux_only_manifests() -> serde_json::Value {
        serde_json::json!([
            { "digest": "sha256:aaa", "platform": { "os": "linux", "architecture": "amd64" } },
            { "digest": "sha256:bbb", "platform": { "os": "linux", "architecture": "arm64" } }
        ])
    }

    #[test]
    fn windows_host_falls_back_to_linux_amd64() {
        let m = linux_only_manifests();
        let arr = m.as_array().unwrap();
        let entry = MultiArchResolver::resolve_for_host(arr, &win()).expect("should fall back to linux");
        assert_eq!(entry["digest"], "sha256:aaa");
    }

    #[test]
    fn linux_host_picks_native_linux() {
        let m = linux_only_manifests();
        let arr = m.as_array().unwrap();
        let entry = MultiArchResolver::resolve_for_host(arr, &linux()).unwrap();
        assert_eq!(entry["digest"], "sha256:aaa");
    }

    #[test]
    fn windows_host_prefers_real_windows_manifest_when_present() {
        let m = serde_json::json!([
            { "digest": "sha256:lin", "platform": { "os": "linux", "architecture": "amd64" } },
            { "digest": "sha256:winamd", "platform": { "os": "windows", "architecture": "amd64" } }
        ]);
        let arr = m.as_array().unwrap();
        let entry = MultiArchResolver::resolve_for_host(arr, &win()).unwrap();
        assert_eq!(entry["digest"], "sha256:winamd", "native windows manifest wins over linux fallback");
    }

    #[test]
    fn no_matching_arch_returns_none() {
        let m = serde_json::json!([
            { "digest": "sha256:arm", "platform": { "os": "linux", "architecture": "arm64" } }
        ]);
        let arr = m.as_array().unwrap();
        // windows/amd64 host: no windows, and linux fallback only has arm64 → none
        assert!(MultiArchResolver::resolve_for_host(arr, &win()).is_none());
    }
}
