use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crushfile {
    pub version: Option<String>,
    pub project: Option<CrushfileProject>,
    pub build: Option<CrushfileBuild>,
    pub stages: Option<Vec<CrushfileStage>>,
    pub env: Option<HashMap<String, String>>,
    pub secrets: Option<Vec<CrushfileSecret>>,
    pub platform: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushfileProject {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub project_type: Option<String>,
    pub runtime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushfileBuild {
    pub command: Option<String>,
    pub entry: Option<String>,
    pub port: Option<u16>,
    pub base: Option<String>,
    pub workdir: Option<String>,
    pub healthcheck: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushfileStage {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub stage_type: String,
    pub command: Option<String>,
    pub rule: Option<String>,
    pub from: Option<String>,
    pub image: Option<String>,
    pub target: Option<String>,
    pub platforms: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushfileSecret {
    pub id: String,
    pub src: Option<String>,
    pub env: Option<String>,
}

pub struct CrushfileParser;

impl CrushfileParser {
    pub fn parse(path: &Path) -> Result<Crushfile> {
        if !path.exists() {
            return Err(CrushError::ImageError(format!("Crushfile not found: {:?}", path)));
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read Crushfile: {}", e)))?;

        let interpolated = Self::interpolate_env(&content);

        let crushfile: Crushfile = toml::from_str(&interpolated)
            .map_err(|e| {
                let msg = format!("Crushfile parse error: {}\n  File: {:?}", e, path);
                CrushError::ImageError(msg)
            })?;

        Self::validate(&crushfile, path)?;

        Ok(crushfile)
    }

    pub fn parse_str(content: &str) -> Result<Crushfile> {
        let interpolated = Self::interpolate_env(content);
        let crushfile: Crushfile = toml::from_str(&interpolated)
            .map_err(|e| CrushError::ImageError(format!("Crushfile parse error: {}", e)))?;
        Ok(crushfile)
    }

    fn validate(crushfile: &Crushfile, path: &Path) -> Result<()> {
        if let Some(ref stages) = crushfile.stages {
            for (i, stage) in stages.iter().enumerate() {
                if stage.stage_type != "run" && stage.stage_type != "copy"
                    && stage.stage_type != "base" && stage.stage_type != "from"
                    && stage.stage_type != "config" {
                    return Err(CrushError::ImageError(format!(
                        "{}:{}: Unknown stage type '{}'. Expected: base, run, copy, from, config",
                        path.display(), i + 1, stage.stage_type
                    )));
                }
            }
        }
        Ok(())
    }

    fn interpolate_env(content: &str) -> String {
        let re = regex::Regex::new(r"\$\{([^}:]+)(?::-(.*?))?\}").unwrap();
        re.replace_all(content, |caps: &regex::Captures| {
            let var = &caps[1];
            let default = caps.get(2).map(|m| m.as_str());
            std::env::var(var).ok()
                .or_else(|| default.map(|s| s.to_string()))
                .unwrap_or_else(|| format!("${{{}}}", var))
        }).to_string()
    }
}
