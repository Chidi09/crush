pub mod parsers;
pub mod source;
pub mod ai_client;
pub mod offline;
pub mod history;

use std::path::{Path, PathBuf};
use anyhow::Result;

pub use parsers::{StackTrace, ParsedFrame, BuildError, BuildErrorKind};
pub use source::SourceContext;
pub use ai_client::DiagnosisResult;
pub use history::ErrorHistory;

#[derive(Debug, Clone)]
pub struct FullDiagnosis {
    pub trace: Option<StackTrace>,
    pub build_errors: Vec<BuildError>,
    pub diagnosis: Option<DiagnosisResult>,
    pub source_context: Option<SourceContext>,
}

#[derive(Debug, Clone)]
pub struct AiEngine {
    pub api_key: Option<String>,
    pub data_dir: PathBuf,
}

impl AiEngine {
    pub fn new(api_key: Option<String>, data_dir: PathBuf) -> Self {
        Self { api_key, data_dir }
    }

    pub async fn diagnose_stderr(
        &self,
        stderr: &str,
        source_root: Option<&Path>,
        project_root: Option<&Path>,
    ) -> Result<FullDiagnosis> {
        let trace = parsers::parse(stderr);

        let mut source_context = None;
        if let Some(ref t) = trace {
            if !t.file.is_empty() && t.line > 0 {
                let file_path = if let Some(root) = source_root {
                    root.join(&t.file)
                } else if let Some(root) = project_root {
                    root.join(&t.file)
                } else {
                    PathBuf::from(&t.file)
                };

                let col = t.stack_frames.first().and_then(|f| f.column);
                source_context = source::extract_context_with_column(&file_path.to_string_lossy(), t.line, col, 8);
            }
        }

        let build_errors = parsers::parse_build_errors(stderr);

        let offline = offline::OfflinePatterns::new();
        let mut diagnosis = offline.match_stderr(stderr);

        if diagnosis.is_none() {
            if let Some(ref key) = self.api_key {
                let client = ai_client::AiClient::new(key.clone());
                let error_json = if let Some(ref t) = trace {
                    serde_json::to_string(t).unwrap_or_else(|_| stderr.to_string())
                } else {
                    stderr.to_string()
                };
                let source_str = source_context.as_ref().map(|ctx| ctx.format());
                let req = ai_client::DiagnosisRequest {
                    error_json,
                    source_context: source_str,
                    project_summary: None,
                };
                if let Ok(res) = client.diagnose(req).await {
                    diagnosis = Some(res);
                }
            }
        }

        let history = history::ErrorHistory::new(self.data_dir.clone());
        if let Some(ref t) = trace {
            let event = history::ErrorEvent {
                id: String::new(),
                timestamp: chrono::Utc::now(),
                language: t.language.clone(),
                exception_type: t.exception_type.clone(),
                file: t.file.clone(),
                line: t.line,
                resolved: false,
            };
            history.record(event);
        }

        Ok(FullDiagnosis {
            trace,
            build_errors,
            diagnosis,
            source_context,
        })
    }
}
