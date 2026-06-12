use serde::{Serialize, Deserialize};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFrame {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
    pub function: String,
    pub module: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTrace {
    pub language: String,
    pub exception_type: String,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub stack_frames: Vec<ParsedFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildErrorKind {
    TypeScript,
    Rust,
    Python,
    Go,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildError {
    pub kind: BuildErrorKind,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub code: Option<String>,
}

pub fn detect_language(stderr: &str) -> &str {
    if stderr.contains("Traceback (most recent call last)") {
        "Python"
    } else if stderr.contains("at Object.<anonymous>") || stderr.contains("at ") && (stderr.contains("node:") || stderr.contains("js")) {
        "Node"
    } else if stderr.contains("thread 'main' panicked") || stderr.contains("stack backtrace:") {
        "Rust"
    } else if stderr.contains("goroutine ") && stderr.contains("[running]:") {
        "Go"
    } else if stderr.contains("Exception in thread") {
        "Java"
    } else {
        "Generic"
    }
}

pub fn parse_nodejs(stderr: &str) -> Option<StackTrace> {
    let lines: Vec<&str> = stderr.lines().collect();
    if lines.is_empty() { return None; }

    // First line typically contains error type and message: TypeError: ...
    let first_line = lines[0].trim();
    let (exception_type, message) = if let Some(pos) = first_line.find(':') {
        (first_line[..pos].trim().to_string(), first_line[pos + 1..].trim().to_string())
    } else {
        ("Error".to_string(), first_line.to_string())
    };

    let re = Regex::new(r"at\s+(?P<fn>[^\s(]+)?\s*(?:\((?P<file>.+?):(?P<line>\d+):(?P<col>\d+)\)|(?P<file2>[^\s]+?):(?P<line2>\d+):(?P<col2>\d+))").ok()?;
    let mut frames = Vec::new();

    for line in &lines {
        if let Some(caps) = re.captures(line) {
            let function = caps.name("fn").map(|m| m.as_str()).unwrap_or("<anonymous>").to_string();
            let file = caps.name("file").or_else(|| caps.name("file2")).map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: u32 = caps.name("line").or_else(|| caps.name("line2")).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let col_num: Option<u32> = caps.name("col").or_else(|| caps.name("col2")).and_then(|m| m.as_str().parse().ok());

            frames.push(ParsedFrame {
                file,
                line: line_num,
                column: col_num,
                function,
                module: None,
            });
        }
    }

    if frames.is_empty() { return None; }

    let primary_file = frames[0].file.clone();
    let primary_line = frames[0].line;

    Some(StackTrace {
        language: "Node".to_string(),
        exception_type,
        message,
        file: primary_file,
        line: primary_line,
        stack_frames: frames,
    })
}

pub fn parse_python(stderr: &str) -> Option<StackTrace> {
    if !stderr.contains("Traceback (most recent call last):") { return None; }
    let lines: Vec<&str> = stderr.lines().collect();

    // Exception class is on the last non-empty line: IndexError: list index out of range
    let last_line = lines.iter().rev().find(|l| !l.trim().is_empty())?.trim();
    let (exception_type, message) = if let Some(pos) = last_line.find(':') {
        (last_line[..pos].trim().to_string(), last_line[pos + 1..].trim().to_string())
    } else {
        ("Exception".to_string(), last_line.to_string())
    };

    let re = Regex::new(r#"File "(?P<file>[^"]+)", line (?P<line>\d+), in (?P<fn>\S+)"#).ok()?;
    let mut frames = Vec::new();

    for line in &lines {
        if let Some(caps) = re.captures(line) {
            let file = caps.name("file").map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: u32 = caps.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let function = caps.name("fn").map(|m| m.as_str().to_string()).unwrap_or_default();

            frames.push(ParsedFrame {
                file,
                line: line_num,
                column: None,
                function,
                module: None,
            });
        }
    }

    if frames.is_empty() { return None; }

    // Python traces put the most recent call last, so the primary crash site is the last frame!
    let primary = frames.last()?.clone();

    Some(StackTrace {
        language: "Python".to_string(),
        exception_type,
        message,
        file: primary.file,
        line: primary.line,
        stack_frames: frames,
    })
}

pub fn parse_rust_panic(stderr: &str) -> Option<StackTrace> {
    if !stderr.contains("thread 'main' panicked at") { return None; }
    let lines: Vec<&str> = stderr.lines().collect();

    // Extract message from panic line
    let panic_line = lines.iter().find(|l| l.contains("panicked at"))?;
    let message = panic_line.trim().to_string();

    // Look for file and line in the panic location (e.g. src/main.rs:10:15)
    let re_loc = Regex::new(r"(?P<file>[^:\s]+):(?P<line>\d+):(?P<col>\d+)").ok()?;
    let (p_file, p_line) = if let Some(caps) = re_loc.captures(panic_line) {
        let f = caps.name("file").map(|m| m.as_str().to_string()).unwrap_or_default();
        let l: u32 = caps.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        (f, l)
    } else {
        (String::new(), 0)
    };

    // Extract frames from backtrace: format "   0: pkg::func\n             at src/file.rs:line"
    let mut frames = Vec::new();
    let re_frame_idx = Regex::new(r"^\s*\d+:\s+(?P<fn>.+)").ok()?;
    let re_frame_loc = Regex::new(r"^\s*at\s+(?P<file>[^:]+):(?P<line>\d+)").ok()?;

    let mut current_fn = String::new();
    for line in &lines {
        if let Some(caps) = re_frame_idx.captures(line) {
            current_fn = caps.name("fn").map(|m| m.as_str().to_string()).unwrap_or_default();
        } else if let Some(caps) = re_frame_loc.captures(line) {
            let file = caps.name("file").map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: u32 = caps.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            frames.push(ParsedFrame {
                file,
                line: line_num,
                column: None,
                function: current_fn.clone(),
                module: None,
            });
        }
    }

    let file = if !p_file.is_empty() { p_file } else { frames.first().map(|f| f.file.clone()).unwrap_or_default() };
    let line = if p_line > 0 { p_line } else { frames.first().map(|f| f.line).unwrap_or(0) };

    Some(StackTrace {
        language: "Rust".to_string(),
        exception_type: "Panic".to_string(),
        message,
        file,
        line,
        stack_frames: frames,
    })
}

pub fn parse_rust_compile(stderr: &str) -> Vec<BuildError> {
    // Attempt to parse JSON message format if available
    let mut errors = Vec::new();
    for line in stderr.lines() {
        if line.starts_with('{') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if val.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                    if let Some(msg) = val.get("message") {
                        if msg.get("level").and_then(|l| l.as_str()) == Some("error") {
                            let message = msg.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
                            let code = msg.get("code").and_then(|c| c.get("code")).and_then(|c| c.as_str()).map(|c| c.to_string());
                            let mut file = None;
                            let mut line_num = None;
                            let mut col_num = None;

                            if let Some(spans) = msg.get("spans").and_then(|s| s.as_array()) {
                                if let Some(span) = spans.iter().find(|s| s.get("is_primary").and_then(|b| b.as_bool()) == Some(true)) {
                                    file = span.get("file_name").and_then(|f| f.as_str()).map(|f| f.to_string());
                                    line_num = span.get("line_start").and_then(|l| l.as_u64()).map(|l| l as u32);
                                    col_num = span.get("column_start").and_then(|c| c.as_u64()).map(|c| c as u32);
                                }
                            }

                            errors.push(BuildError {
                                kind: BuildErrorKind::Rust,
                                message,
                                file,
                                line: line_num,
                                column: col_num,
                                code,
                            });
                        }
                    }
                }
            }
        }
    }

    if !errors.is_empty() { return errors; }

    // Fallback to text parser: error[E####]: message\n --> file:line:col
    let re_head = Regex::new(r"error(?:\[(?P<code>E\d+)\])?:\s*(?P<msg>.+)").ok().unwrap();
    let re_loc = Regex::new(r"\s*-->\s*(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+)").ok().unwrap();

    let lines: Vec<&str> = stderr.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(caps) = re_head.captures(lines[i]) {
            let code = caps.name("code").map(|m| m.as_str().to_string());
            let message = caps.name("msg").map(|m| m.as_str().to_string()).unwrap_or_default();
            let mut file = None;
            let mut line = None;
            let mut column = None;

            if i + 1 < lines.len() {
                if let Some(loc_caps) = re_loc.captures(lines[i + 1]) {
                    file = loc_caps.name("file").map(|m| m.as_str().to_string());
                    line = loc_caps.name("line").and_then(|m| m.as_str().parse().ok());
                    column = loc_caps.name("column").or_else(|| loc_caps.name("col")).and_then(|m| m.as_str().parse().ok());
                }
            }

            errors.push(BuildError {
                kind: BuildErrorKind::Rust,
                message,
                file,
                line,
                column,
                code,
            });
        }
        i += 1;
    }

    errors
}

pub fn parse_go(stderr: &str) -> Option<StackTrace> {
    if !stderr.contains("goroutine ") { return None; }
    let lines: Vec<&str> = stderr.lines().collect();

    let panic_line = lines.iter().find(|l| l.contains("panic:"))?;
    let message = panic_line.trim().to_string();

    let mut frames = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.contains("goroutine ") && line.contains("[running]:") {
            i += 1;
            while i < lines.len() {
                let frame_line = lines[i].trim();
                if frame_line.is_empty() || frame_line.contains("goroutine ") {
                    break;
                }
                //pkg.Func(...)
                let function = frame_line.to_string();
                i += 1;
                if i < lines.len() {
                    let loc_line = lines[i].trim();
                    //\tfile.go:N +0x...
                    let re_loc = Regex::new(r"(?P<file>[^:]+):(?P<line>\d+)").ok().unwrap();
                    if let Some(caps) = re_loc.captures(loc_line) {
                        let file = caps.name("file").map(|m| m.as_str().to_string()).unwrap_or_default();
                        let line_num: u32 = caps.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                        frames.push(ParsedFrame {
                            file,
                            line: line_num,
                            column: None,
                            function: function.clone(),
                            module: None,
                        });
                    }
                }
                i += 1;
            }
        }
        i += 1;
    }

    if frames.is_empty() { return None; }

    let primary = frames[0].clone();

    Some(StackTrace {
        language: "Go".to_string(),
        exception_type: "Panic".to_string(),
        message,
        file: primary.file,
        line: primary.line,
        stack_frames: frames,
    })
}

pub fn parse_java(stderr: &str) -> Option<StackTrace> {
    if !stderr.contains("Exception in thread") { return None; }
    let lines: Vec<&str> = stderr.lines().collect();

    let first_line = lines.iter().find(|l| l.contains("Exception in thread"))?;
    // Exception in thread "main" pkg.ExceptionType: msg
    let re_head = Regex::new(r#"Exception in thread "[^"]+"\s+(?P<ex>\S+):\s*(?P<msg>.+)"#).ok()?;
    let (exception_type, message) = if let Some(caps) = re_head.captures(first_line) {
        (caps.name("ex").map(|m| m.as_str().to_string()).unwrap_or_default(), caps.name("msg").map(|m| m.as_str().to_string()).unwrap_or_default())
    } else {
        ("Exception".to_string(), first_line.to_string())
    };

    let re_frame = Regex::new(r"^\s*at\s+(?P<fn>[^(]+)\((?P<file>[^:]+):(?P<line>\d+)\)").ok()?;
    let mut frames = Vec::new();

    for line in &lines {
        if let Some(caps) = re_frame.captures(line) {
            let function = caps.name("fn").map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            let file = caps.name("file").map(|m| m.as_str().to_string()).unwrap_or_default();
            let line_num: u32 = caps.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);

            frames.push(ParsedFrame {
                file,
                line: line_num,
                column: None,
                function,
                module: None,
            });
        }
    }

    if frames.is_empty() { return None; }

    let primary = frames[0].clone();

    Some(StackTrace {
        language: "Java".to_string(),
        exception_type,
        message,
        file: primary.file,
        line: primary.line,
        stack_frames: frames,
    })
}

pub fn parse(stderr: &str) -> Option<StackTrace> {
    match detect_language(stderr) {
        "Node" => parse_nodejs(stderr),
        "Python" => parse_python(stderr),
        "Rust" => parse_rust_panic(stderr),
        "Go" => parse_go(stderr),
        "Java" => parse_java(stderr),
        _ => None,
    }
}

pub fn parse_build_errors(stderr: &str) -> Vec<BuildError> {
    let mut errors = parse_rust_compile(stderr);
    if !errors.is_empty() { return errors; }

    // Parse TypeScript compilation errors (tsc): "src/file.ts(10,15): error TS2304: Cannot find name 'x'."
    let re_ts = Regex::new(r"(?P<file>[^(]+)\((?P<line>\d+),(?P<col>\d+)\):\s*error\s+(?P<code>TS\d+):\s*(?P<msg>.+)").ok().unwrap();
    // Parse Go build errors: "./main.go:10:15: undefined: x"
    let re_go = Regex::new(r"(?P<file>[^:]+):(?P<line>\d+):(?P<col>\d+):\s*(?P<msg>.+)").ok().unwrap();

    for line in stderr.lines() {
        if let Some(caps) = re_ts.captures(line) {
            errors.push(BuildError {
                kind: BuildErrorKind::TypeScript,
                message: caps.name("msg").map(|m| m.as_str().to_string()).unwrap_or_default(),
                file: caps.name("file").map(|m| m.as_str().trim().to_string()),
                line: caps.name("line").and_then(|m| m.as_str().parse().ok()),
                column: caps.name("col").and_then(|m| m.as_str().parse().ok()),
                code: caps.name("code").map(|m| m.as_str().to_string()),
            });
        } else if let Some(caps) = re_go.captures(line) {
            errors.push(BuildError {
                kind: BuildErrorKind::Go,
                message: caps.name("msg").map(|m| m.as_str().to_string()).unwrap_or_default(),
                file: caps.name("file").map(|m| m.as_str().trim().to_string()),
                line: caps.name("line").and_then(|m| m.as_str().parse().ok()),
                column: caps.name("col").and_then(|m| m.as_str().parse().ok()),
                code: None,
            });
        }
    }

    if errors.is_empty() {
        // Generic fallback error
        if stderr.contains("error") || stderr.contains("failed") {
            errors.push(BuildError {
                kind: BuildErrorKind::Generic,
                message: stderr.trim().to_string(),
                file: None,
                line: None,
                column: None,
                code: None,
            });
        }
    }

    errors
}
