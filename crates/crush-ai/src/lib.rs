pub mod parsers;
pub mod build_errors;
pub mod source;
pub mod diagnose;
pub mod autofix;
pub mod history;

use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError};

pub use parsers::{TraceParser, ParsedTrace, StackFrame};
pub use build_errors::{BuildErrorParser, BuildError, BuildErrorKind};
pub use source::{SourceExtractor, SourceContext, SourceLine};
pub use diagnose::{DiagnosticEngine, AiDiagnosis};
pub use autofix::AutoFixApplicator;
pub use history::{ErrorHistory, ErrorRecord};

pub struct AiEngine {
    pub trace_parser: TraceParser,
    pub build_parser: BuildErrorParser,
    pub diagnostic: DiagnosticEngine,
    pub fix_applicator: AutoFixApplicator,
    pub history: ErrorHistory,
}

impl AiEngine {
    pub fn new(api_key: Option<String>, data_dir: PathBuf) -> Self {
        Self {
            trace_parser: TraceParser::new(),
            build_parser: BuildErrorParser::new(),
            diagnostic: DiagnosticEngine::new(api_key),
            fix_applicator: AutoFixApplicator::new(),
            history: ErrorHistory::new(&data_dir),
        }
    }

    pub async fn diagnose_stderr(
        &self,
        stderr: &str,
        build_command: Option<&str>,
        project_root: Option<&Path>,
    ) -> Result<FullDiagnosis> {
        // 1. Run ALL runtime parsers, scored. Highest confidence wins.
        if let Some(trace) = self.trace_parser.parse(stderr) {
            let source_ctx = if !trace.file.is_empty() && trace.line > 0 {
                let file_path = project_root.map(|r| r.join(&trace.file))
                    .unwrap_or_else(|| PathBuf::from(&trace.file));
                SourceExtractor::extract(&file_path, trace.line, trace.column, 8)
            } else {
                None
            };

            let source_str = source_ctx.as_ref().map(|ctx| {
                ctx.context_lines.iter()
                    .map(|l| format!("{:>4} {}", l.line_number, l.content))
                    .collect::<Vec<_>>()
                    .join("\n")
            });

            let diagnosis = self.diagnostic.diagnose(&trace, source_str.as_deref()).await?;
            let record = self.history.log(&trace.exception_type, &trace.language, &trace.file, trace.line);

            return Ok(FullDiagnosis {
                trace: Some(trace),
                build_errors: vec![],
                diagnosis: Some(diagnosis),
                source_context: source_ctx,
                error_record: Some(record),
            });
        }

        // 2. Try build error parsing
        if let Some(cmd) = build_command {
            let errors = self.build_parser.parse(stderr, cmd);
            if !errors.is_empty() {
                return Ok(FullDiagnosis {
                    trace: None,
                    build_errors: errors,
                    diagnosis: None,
                    source_context: None,
                    error_record: None,
                });
            }
        }

        // 3. Generic fallback
        Ok(FullDiagnosis {
            trace: None,
            build_errors: vec![],
            diagnosis: None,
            source_context: None,
            error_record: None,
        })
    }
}

#[derive(Debug)]
pub struct FullDiagnosis {
    pub trace: Option<ParsedTrace>,
    pub build_errors: Vec<BuildError>,
    pub diagnosis: Option<AiDiagnosis>,
    pub source_context: Option<SourceContext>,
    pub error_record: Option<ErrorRecord>,
}
