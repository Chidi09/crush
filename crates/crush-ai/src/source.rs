use std::path::{Path, PathBuf};
use std::fs;
use crush_types::{Result, CrushError};

#[derive(Debug)]
pub struct SourceContext {
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub context_lines: Vec<SourceLine>,
    pub git_blame: Option<String>,
}

#[derive(Debug)]
pub struct SourceLine {
    pub line_number: usize,
    pub content: String,
    pub is_target: bool,
    pub highlighted: String,
}

pub struct SourceExtractor;

impl SourceExtractor {
    pub fn new() -> Self { Self }

    pub fn extract(file: &Path, line: usize, column: Option<usize>, context: usize) -> Option<SourceContext> {
        if !file.exists() { return None; }
        let content = fs::read_to_string(file).ok()?;
        let lines: Vec<&str> = content.lines().collect();
        if line == 0 || line > lines.len() { return None; }

        let start = line.saturating_sub(context + 1);
        let end = (line + context).min(lines.len());

        let mut context_lines = Vec::new();
        for i in start..end {
            let ln = i + 1;
            let is_target = ln == line;
            let raw = lines.get(i).unwrap_or(&"").to_string();

            let highlighted = if is_target && column.is_some() {
                let col = column.unwrap_or(1).saturating_sub(1);
                let indicator = " ".repeat(col) + "^~~~";
                format!("{}", raw)
            } else {
                raw.clone()
            };

            context_lines.push(SourceLine {
                line_number: ln,
                content: raw,
                is_target,
                highlighted,
            });
        }

        let git_blame = Self::git_blame(file, line);

        Some(SourceContext { file: file.to_path_buf(), line, column, context_lines, git_blame })
    }

    fn git_blame(file: &Path, line: usize) -> Option<String> {
        let output = std::process::Command::new("git")
            .args(["blame", "-L", &format!("{},{}", line, line), "--porcelain", &file.to_string_lossy()])
            .output().ok()?;
        if output.status.success() {
            let out = String::from_utf8_lossy(&output.stdout);
            for line in out.lines() {
                if line.starts_with("author ") {
                    return Some(format!("last change by {}", line.trim_start_matches("author ")));
                }
            }
        }
        None
    }
}
