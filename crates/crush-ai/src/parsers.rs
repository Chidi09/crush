use crush_types::{Result, CrushError};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTrace {
    pub language: String,
    pub exception_type: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: Option<usize>,
    pub stack_frames: Vec<StackFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub function: Option<String>,
    pub file: String,
    pub line: usize,
    pub column: Option<usize>,
    pub raw: String,
}

pub struct TraceParser;

impl TraceParser {
    pub fn new() -> Self { Self }

    pub fn parse(&self, stderr: &str) -> Option<ParsedTrace> {
        let candidates: Vec<(ParsedTrace, f32)> = [
            self.parse_node(stderr),
            self.parse_python(stderr),
            self.parse_rust(stderr),
            self.parse_go(stderr),
            self.parse_java(stderr),
            self.parse_dotnet(stderr),
            self.parse_ruby(stderr),
            self.parse_php(stderr),
            self.parse_elixir(stderr),
            self.parse_c_cpp(stderr),
        ].into_iter().flatten().map(|t| {
            let score = score_parsed_trace(&t, stderr);
            (t, score)
        }).collect();

        candidates.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(trace, _score)| trace)
            .or_else(|| self.parse_generic(stderr))
    }

    pub fn parse_node(&self, stderr: &str) -> Option<ParsedTrace> {
        let lines: Vec<&str> = stderr.lines().collect();
        let error_line = lines.iter().find(|l| l.contains("Error:") || l.contains("TypeError") || l.contains("ReferenceError") || l.contains("SyntaxError") || l.contains("RangeError"))?;
        let exception_type = Self::extract_first_word(error_line);
        let message = error_line.trim().to_string();

        let mut frames = Vec::new();
        let mut file = String::new();
        let mut line = 0usize;
        let mut column = None;

        for l in &lines {
            let l = l.trim();
            if let Some(frame) = l.strip_prefix("at ") {
                let (func, f, ln, col) = self.parse_v8_frame(frame);
                if file.is_empty() { file = f.clone(); line = ln; column = col; }
                frames.push(StackFrame { function: func, file: f, line: ln, column: col, raw: l.to_string() });
            }
        }

        if file.is_empty() { return None; }
        Some(ParsedTrace { language: "Node.js (TypeScript)".into(), exception_type, message, file, line, column, stack_frames: frames })
    }

    pub fn parse_python(&self, stderr: &str) -> Option<ParsedTrace> {
        let lines: Vec<&str> = stderr.lines().collect();
        let trace_start = lines.iter().position(|l| l.trim() == "Traceback (most recent call last):")?;

        let error_line = lines.iter().rev().find(|l| l.contains("Error:") || l.contains("Exception:"))?;
        let colon_pos = error_line.find(':')?;
        let exception_type = error_line[..colon_pos].trim().to_string();
        let message = error_line[colon_pos + 1..].trim().to_string();

        let mut frames = Vec::new();
        let mut file = String::new();
        let mut line = 0;

        for l in &lines[trace_start..] {
            let l = l.trim();
            if let Some(rest) = l.strip_prefix("File \"") {
                if let Some(end) = rest.find('"') {
                    let f = rest[..end].to_string();
                    let rest = rest[end + 1..].trim();
                    if let Some(ln_str) = rest.strip_prefix("line ") {
                        if let Ok(ln) = ln_str.split(',').next().unwrap_or("0").trim().parse::<usize>() {
                            if file.is_empty() { file = f.clone(); line = ln; }
                            let code = lines.iter().skip_while(|x| x.trim() != l).nth(1).map(|s| s.trim().to_string());
                            frames.push(StackFrame {
                                function: None, file: f, line: ln, column: None,
                                raw: code.unwrap_or_default(),
                            });
                        }
                    }
                }
            }
        }

        Some(ParsedTrace { language: "Python".into(), exception_type, message, file, line, column: None, stack_frames: frames })
    }

    pub fn parse_rust(&self, stderr: &str) -> Option<ParsedTrace> {
        let lines: Vec<&str> = stderr.lines().collect();
        if lines.iter().any(|l| l.contains("thread '") && l.contains("panicked")) {
            let panic_line = lines.iter().find(|l| l.contains("panicked"))?;
            let message = panic_line.trim().to_string();
            let mut frames = Vec::new();
            let mut file = String::new();
            let mut line = 0;

            for l in &lines {
                let l = l.trim();
                if let Some(rest) = l.strip_prefix("at ") {
                    let parts: Vec<&str> = rest.rsplitn(3, ':').collect();
                    if parts.len() >= 2 {
                        let ln: usize = parts[0].parse().unwrap_or(0);
                        let col: Option<usize> = parts.get(1).and_then(|c| c.parse().ok());
                        let f = if parts.len() >= 3 { parts[2..].join(":") } else { parts[1..].join(":") };
                        if file.is_empty() { file = f.clone(); line = ln; }
                        frames.push(StackFrame { function: None, file: f, line: ln, column: col, raw: l.to_string() });
                    }
                }
            }
            if file.is_empty() { return None; }
            return Some(ParsedTrace { language: "Rust".into(), exception_type: "panic".into(), message, file, line, column: None, stack_frames: frames });
        }

        if stderr.contains("error[") || stderr.contains("error:") {
            let err_line = lines.iter().find(|l| l.contains("error["))?;
            let ex_type = Self::extract_first_word(err_line);
            let message = lines.iter().skip_while(|l| !l.contains("error[")).take(5).cloned().collect::<Vec<_>>().join("\n");
            let mut frames = Vec::new();
            for l in &lines {
                if l.contains("--> ") {
                    let parts: Vec<&str> = l.split("--> ").nth(1).unwrap_or("").rsplitn(3, ':').collect();
                    if parts.len() >= 2 {
                        let f = parts[2..].join(":").trim().to_string();
                        let ln: usize = parts[1].parse().unwrap_or(0);
                        let col: Option<usize> = parts.first().and_then(|c| c.parse().ok());
                        if f.contains('/') || f.contains('\\') {
                            frames.push(StackFrame { function: None, file: f, line: ln, column: col, raw: l.to_string() });
                        }
                    }
                }
            }
            let file = frames.first().map(|f| f.file.clone()).unwrap_or_default();
            let line = frames.first().map(|f| f.line).unwrap_or(0);
            return Some(ParsedTrace { language: "Rust".into(), exception_type: ex_type, message, file, line, column: None, stack_frames: frames });
        }

        None
    }

    pub fn parse_go(&self, stderr: &str) -> Option<ParsedTrace> {
        if !stderr.contains("goroutine ") || !stderr.contains("panic") { return None; }
        let lines: Vec<&str> = stderr.lines().collect();
        let error_line = lines.iter().find(|l| l.contains("panic:"))?;
        let message = error_line.trim().to_string();
        let mut frames = Vec::new();
        let mut file = String::new();
        let mut line = 0;
        for l in &lines {
            let l = l.trim();
            let parts: Vec<&str> = l.splitn(3, ':').collect();
            if l.contains(".go:") && parts.len() >= 2 {
                if let Ok(ln) = parts[1].trim().parse::<usize>() {
                    if file.is_empty() { file = parts[0].trim().to_string(); line = ln; }
                    frames.push(StackFrame { function: None, file: parts[0].trim().to_string(), line: ln, column: None, raw: l.to_string() });
                }
            }
        }
        Some(ParsedTrace { language: "Go".into(), exception_type: "panic".into(), message, file, line, column: None, stack_frames: frames })
    }

    pub fn parse_java(&self, stderr: &str) -> Option<ParsedTrace> {
        let lines: Vec<&str> = stderr.lines().collect();
        let error_line = lines.iter().find(|l| l.contains("Exception"))?;
        let colon_pos = error_line.find(':').unwrap_or(error_line.len());
        let exception_type = error_line[..colon_pos].trim().to_string();
        let message = error_line[colon_pos..].trim().to_string();
        let mut frames = Vec::new();
        let mut file = String::new();
        let mut line = 0;
        for l in &lines {
            let l = l.trim();
            if let Some(rest) = l.strip_prefix("at ") {
                let paren = rest.rfind('(')?;
                let loc = &rest[paren + 1..rest.len() - 1];
                let parts: Vec<&str> = loc.rsplitn(3, ':').collect();
                if parts.len() >= 2 {
                    let f = parts[2..].join(":");
                    if let Ok(ln) = parts[1].parse::<usize>() {
                        if file.is_empty() { file = f.clone(); line = ln; }
                        frames.push(StackFrame { function: None, file: f, line: ln, column: None, raw: l.to_string() });
                    }
                }
            }
        }
        Some(ParsedTrace { language: "Java".into(), exception_type, message, file, line, column: None, stack_frames: frames })
    }

    fn parse_v8_frame(&self, frame: &str) -> (Option<String>, String, usize, Option<usize>) {
        let (func, loc) = if let Some(paren) = frame.rfind('(') {
            (frame[..paren].trim().to_string(), frame[paren + 1..frame.len() - 1].trim())
        } else {
            (String::new(), frame.trim())
        };
        let parts: Vec<&str> = loc.rsplitn(4, ':').collect();
        let col: Option<usize> = parts.first().and_then(|c| c.parse().ok());
        let line: usize = parts.get(1).and_then(|c| c.parse().ok()).unwrap_or(0);
        let file = if parts.len() >= 3 { parts[2..].join(":") } else { loc.to_string() };
        (if func.is_empty() { None } else { Some(func) }, file, line, col)
    }

    fn parse_generic(&self, stderr: &str) -> Option<ParsedTrace> {
        let lines: Vec<&str> = stderr.lines().collect();
        let first_error = lines.iter().find(|l| l.contains("error") || l.contains("Error") || l.contains("ERROR"))?;
        Some(ParsedTrace {
            language: "Unknown".into(),
            exception_type: "error".into(),
            message: first_error.trim().to_string(),
            file: String::new(), line: 0, column: None,
            stack_frames: vec![],
        })
    }

    // Stub parsers for remaining languages
    pub fn parse_dotnet(&self, stderr: &str) -> Option<ParsedTrace> {
        if !stderr.contains("Exception") || !stderr.contains("at ") { return None; }
        self.parse_java(stderr).map(|mut t| { t.language = ".NET".into(); t })
    }

    pub fn parse_ruby(&self, stderr: &str) -> Option<ParsedTrace> {
        let lines: Vec<&str> = stderr.lines().collect();
        let err = lines.iter().find(|l| l.contains("Error") || l.contains("Exception"))?;
        let (ex_type, msg) = err.split_once(':').unwrap_or((err.trim(), ""));
        let mut frames = Vec::new();
        for l in &lines {
            if let Some(rest) = l.trim().strip_prefix("from ") {
                if let Some(pos) = rest.rfind(':') {
                    let f = rest[..pos].trim().to_string();
                    if let Ok(ln) = rest[pos + 1..].trim().parse::<usize>() {
                        frames.push(StackFrame { function: None, file: f, line: ln, column: None, raw: l.to_string() });
                    }
                }
            }
        }
        let first = frames.first().cloned().unwrap_or(StackFrame { function: None, file: String::new(), line: 0, column: None, raw: String::new() });
        Some(ParsedTrace { language: "Ruby".into(), exception_type: ex_type.to_string(), message: msg.to_string(), file: first.file, line: first.line, column: None, stack_frames: frames })
    }

    pub fn parse_php(&self, stderr: &str) -> Option<ParsedTrace> {
        if !stderr.contains("Fatal error") && !stderr.contains("Stack trace") { return None; }
        let lines: Vec<&str> = stderr.lines().collect();
        let err = lines.iter().find(|l| l.contains("Fatal error") || l.contains("Error"))?;
        let msg = err.trim().to_string();
        let mut frames = Vec::new();
        for l in &lines {
            if l.contains('#') && l.contains("called in") {
                if let Some(pos) = l.rfind("in ") {
                    let rest = &l[pos + 3..];
                    if let Some(end) = rest.rfind(':') {
                        if let Ok(ln) = rest[end + 1..].trim().parse::<usize>() {
                            frames.push(StackFrame { function: None, file: rest[..end].to_string(), line: ln, column: None, raw: l.to_string() });
                        }
                    }
                }
            }
        }
        Some(ParsedTrace { language: "PHP".into(), exception_type: "Fatal error".into(), message: msg, file: frames.first().map(|f| f.file.clone()).unwrap_or_default(), line: frames.first().map(|f| f.line).unwrap_or(0), column: None, stack_frames: frames })
    }

    pub fn parse_elixir(&self, stderr: &str) -> Option<ParsedTrace> {
        if !stderr.contains("** (") { return None; }
        let lines: Vec<&str> = stderr.lines().collect();
        let err = lines.iter().find(|l| l.contains("** ("))?;
        let msg = err.trim().to_string();
        let mut frames = Vec::new();
        for l in &lines {
            if l.contains(".ex:") || l.contains(".exs:") {
                let parts: Vec<&str> = l.rsplitn(4, ':').collect();
                if parts.len() >= 2 {
                    if let Ok(ln) = parts[1].parse::<usize>() {
                        let f = parts[2..].join(":").trim().to_string();
                        frames.push(StackFrame { function: None, file: f, line: ln, column: None, raw: l.to_string() });
                    }
                }
            }
        }
        Some(ParsedTrace { language: "Elixir".into(), exception_type: "FunctionClauseError".into(), message: msg, file: frames.first().map(|f| f.file.clone()).unwrap_or_default(), line: frames.first().map(|f| f.line).unwrap_or(0), column: None, stack_frames: frames })
    }

    pub fn parse_c_cpp(&self, stderr: &str) -> Option<ParsedTrace> {
        if stderr.contains("AddressSanitizer") || stderr.contains("Segmentation fault") || stderr.contains("SIGSEGV") {
            Some(ParsedTrace { language: "C/C++".into(), exception_type: "Segfault".into(), message: stderr.lines().find(|l| l.contains("ERROR") || l.contains("Segfault")).unwrap_or("SEGFAULT").to_string(), file: String::new(), line: 0, column: None, stack_frames: vec![] })
        } else { None }
    }

    fn extract_first_word(s: &str) -> String {
        let s = s.trim();
        if let Some(colon) = s.find(':') { s[..colon].trim().to_string() }
        else { s.split_whitespace().next().unwrap_or("Error").to_string() }
    }
}

fn score_parsed_trace(trace: &ParsedTrace, raw_stderr: &str) -> f32 {
    let mut score = 0.0f32;

    // Each valid stack frame is strong evidence
    score += trace.stack_frames.len() as f32 * 2.0;

    // A real file path with extension is strong signal
    let has_ext = trace.file.contains('.') && trace.file.len() > 5;
    if has_ext { score += 3.0; }

    // Line number must be > 0
    if trace.line > 0 { score += 2.0; }

    // Matched exception type present in raw stderr
    if !trace.exception_type.is_empty() && raw_stderr.contains(&trace.exception_type) { score += 2.0; }

    // Language-specific confidence boosters
    match trace.language.as_str() {
        "Rust" => {
            if raw_stderr.contains("panicked at") { score += 5.0; }
            if raw_stderr.contains("error[") { score += 4.0; }
            if raw_stderr.contains("--> ") { score += 2.0; }
        }
        "Python" => {
            if raw_stderr.contains("Traceback") { score += 5.0; }
            if raw_stderr.contains("File \"") { score += 3.0; }
        }
        "Node.js (TypeScript)" | "Node.js" => {
            if raw_stderr.contains("at ") { score += 3.0; }
            if trace.column.is_some() { score += 1.0; }
        }
        "Go" => {
            if raw_stderr.contains("goroutine ") { score += 5.0; }
        }
        "Java" => {
            if raw_stderr.contains("Exception") { score += 3.0; }
            if trace.language == "Java" && raw_stderr.contains(".java:") { score += 2.0; }
        }
        _ => {}
    }

    // Penalize generic/no-match
    if trace.exception_type == "error" && trace.file.is_empty() && trace.stack_frames.is_empty() {
        score -= 5.0;
    }

    // Favourably weight first real file path over 'unknown.rs' or default names
    if trace.file.contains('/') || trace.file.contains('\\') { score += 2.0; }

    // Multiple frames means real runtime crash vs build error
    if trace.stack_frames.len() >= 3 { score += 3.0; }

    // Unambiguous language signature detection
    let lang_signals = match trace.language.as_str() {
        "Python" => if raw_stderr.contains("Traceback (most recent call last)") { 4.0 } else { 0.0 },
        "Go" => if raw_stderr.contains("created by ") { 3.0 } else { 0.0 },
        "Rust" => if raw_stderr.contains("note: run with `RUST_BACKTRACE=1`") { 3.0 } else { 0.0 },
        _ => 0.0,
    };
    score += lang_signals;

    score.max(0.0)
}
