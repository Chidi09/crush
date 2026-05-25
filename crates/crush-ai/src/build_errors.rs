use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildError {
    pub tool: String,
    pub kind: BuildErrorKind,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub code: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildErrorKind {
    CompileError, DependencyConflict, ConfigError, PermissionError, NetworkTimeout, Other,
}

pub struct BuildErrorParser;

impl BuildErrorParser {
    pub fn new() -> Self { Self }

    pub fn parse(&self, stderr: &str, build_command: &str) -> Vec<BuildError> {
        let tool = self.detect_tool(build_command);

        match tool.as_str() {
            "cargo" => self.parse_cargo(stderr),
            "tsc" => self.parse_typescript(stderr),
            "webpack" => self.parse_bundler(stderr, "webpack"),
            "vite" => self.parse_bundler(stderr, "vite"),
            "esbuild" => self.parse_bundler(stderr, "esbuild"),
            "maven" | "gradle" => self.parse_java_build(stderr),
            "pip" | "npm" | "yarn" | "pnpm" => self.parse_dependency_conflict(stderr, &tool),
            _ => vec![],
        }
    }

    fn detect_tool(&self, cmd: &str) -> String {
        if cmd.contains("cargo") { "cargo" }
        else if cmd.contains("tsc") || cmd.contains("typescript") { "tsc" }
        else if cmd.contains("webpack") { "webpack" }
        else if cmd.contains("vite") { "vite" }
        else if cmd.contains("esbuild") { "esbuild" }
        else if cmd.contains("mvn") || cmd.contains("maven") { "maven" }
        else if cmd.contains("gradle") { "gradle" }
        else if cmd.contains("pip") { "pip" }
        else if cmd.contains("npm") { "npm" }
        else if cmd.contains("yarn") { "yarn" }
        else if cmd.contains("pnpm") { "pnpm" }
        else { "unknown" }.to_string()
    }

    fn parse_cargo(&self, stderr: &str) -> Vec<BuildError> {
        let mut errors = Vec::new();
        for line in stderr.lines() {
            if line.contains("error[") {
                let parts: Vec<&str> = line.splitn(2, "error[").collect();
                let rest = parts.get(1).copied().unwrap_or("error]");
                let code = rest.split(']').next().unwrap_or("").to_string();
                let message = rest.split(']').nth(1).unwrap_or("").trim().to_string();
                errors.push(BuildError {
                    tool: "cargo".into(), kind: BuildErrorKind::CompileError,
                    message, file: None, line: None, column: None,
                    code: Some(code), suggestion: None,
                });
            }
            if let Some(rest) = line.trim().strip_prefix("= help: ") {
                if let Some(last) = errors.last_mut() { last.suggestion = Some(rest.to_string()); }
            }
            if line.contains("--> ") {
                let parts: Vec<&str> = line.split("--> ").nth(1).unwrap_or("").rsplitn(3, ':').collect();
                if let Some(last) = errors.last_mut() {
                    last.file = parts.get(2).map(|s| s.trim().to_string());
                    last.line = parts.get(1).and_then(|s| s.parse().ok());
                    last.column = parts.first().and_then(|s| s.parse().ok());
                }
            }
        }
        errors
    }

    fn parse_typescript(&self, stderr: &str) -> Vec<BuildError> {
        let mut errors = Vec::new();
        for line in stderr.lines() {
            if line.contains("error TS") || line.contains(" TS ") {
                let code = if line.contains("error TS") {
                    let s = line.split("error TS").nth(1).unwrap_or("");
                    s.split(':').next().unwrap_or("").trim().to_string()
                } else { String::new() };
                let parts: Vec<&str> = line.splitn(2, "error").collect();
                let msg = parts.get(1).unwrap_or(&"").trim().to_string();
                errors.push(BuildError {
                    tool: "tsc".into(), kind: BuildErrorKind::CompileError,
                    message: msg, file: None, line: None, column: None,
                    code: Some(code), suggestion: None,
                });
            }
        }
        errors
    }

    fn parse_bundler(&self, stderr: &str, _tool: &str) -> Vec<BuildError> {
        let mut errors = Vec::new();
        for line in stderr.lines() {
            if line.contains("ERROR") || line.contains("Error") {
                errors.push(BuildError {
                    tool: "bundler".into(), kind: BuildErrorKind::CompileError,
                    message: line.trim().to_string(), file: None, line: None, column: None,
                    code: None, suggestion: None,
                });
            }
        }
        errors
    }

    fn parse_java_build(&self, stderr: &str) -> Vec<BuildError> {
        let mut errors = Vec::new();
        for line in stderr.lines() {
            if line.contains("FAILED") || line.contains("BUILD FAILURE") {
                errors.push(BuildError {
                    tool: "java".into(), kind: BuildErrorKind::CompileError,
                    message: line.trim().to_string(), file: None, line: None, column: None,
                    code: None, suggestion: None,
                });
            }
        }
        errors
    }

    fn parse_dependency_conflict(&self, stderr: &str, tool: &str) -> Vec<BuildError> {
        let mut errors = Vec::new();
        for line in stderr.lines() {
            if line.contains("conflict") || line.contains("incompatible") || line.contains("version resolution") {
                errors.push(BuildError {
                    tool: tool.to_string(), kind: BuildErrorKind::DependencyConflict,
                    message: line.trim().to_string(), file: None, line: None, column: None,
                    code: None, suggestion: None,
                });
            }
        }
        errors
    }
}
