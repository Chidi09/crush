use serde::{Serialize, Deserialize};
use regex::Regex;
use crate::ai_client::DiagnosisResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflinePattern {
    pub language: String,
    pub pattern: String,
    pub root_cause: String,
    pub fix_description: String,
}

#[derive(Debug, Clone)]
pub struct OfflinePatterns {
    pub patterns: Vec<OfflinePattern>,
}

impl OfflinePatterns {
    pub fn new() -> Self {
        let raw_json = include_str!("patterns.json");
        let patterns: Vec<OfflinePattern> = serde_json::from_str(raw_json).unwrap_or_default();
        Self { patterns }
    }

    pub fn match_stderr(&self, stderr: &str) -> Option<DiagnosisResult> {
        for pat in &self.patterns {
            if let Ok(re) = Regex::new(&pat.pattern) {
                if re.is_match(stderr) {
                    return Some(DiagnosisResult {
                        root_cause: pat.root_cause.clone(),
                        fix_description: pat.fix_description.clone(),
                        proposed_patch: None,
                        auto_fixable: false,
                        confidence: 0.8,
                    });
                }
            }
        }
        None
    }
}
