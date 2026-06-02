//! Additional language stack-trace parsers (v0.9): Ruby, PHP, .NET, Deno, C/C++.
//!
//! Each parser turns a raw stderr blob into a [`StackTrace`]. They were written
//! against real, version-pinned crash dumps (Ruby 3.3, PHP 8.3, .NET 8, Deno,
//! GCC/GDB/ASan) so the frame grammars match what those runtimes actually emit.
//! All patterns are RE2-compatible (the `regex` crate has no look-around).

use crate::parsers::{ParsedFrame, StackTrace};
use regex::Regex;

const MAX_FRAMES: usize = 64;

fn strip_quotes(s: &str) -> String {
    s.trim()
        .trim_matches(|c| c == '\'' || c == '`' || c == '"')
        .to_string()
}

// ── Ruby 3.3 (YARV) ─────────────────────────────────────────────────────────
// Header forms:  `SomeError: message`  |  trailing `(SomeError)` on a line.
// Frame forms:   `path.rb:399:in 'method'`  or  `from path.rb:45:in \`method'`
pub fn parse_ruby(stderr: &str) -> Option<StackTrace> {
    let re_frame =
        Regex::new(r"^\s*(?:from\s+)?(?P<file>.+?\.rb):(?P<line>\d+):in [`'\x22]?(?P<fn>[^'`\x22\n]+)")
            .ok()?;
    let re_head_colon =
        Regex::new(r"^(?P<ex>[A-Z][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)*): (?P<msg>.+)$").ok()?;
    let re_head_paren = Regex::new(r"\((?P<ex>[A-Z][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)*)\)\s*$").ok()?;

    let mut frames = Vec::new();
    for line in stderr.lines() {
        if let Some(c) = re_frame.captures(line) {
            frames.push(ParsedFrame {
                file: c["file"].to_string(),
                line: c["line"].parse().unwrap_or(0),
                column: None,
                function: strip_quotes(&c["fn"]),
                module: None,
            });
            if frames.len() >= MAX_FRAMES {
                break;
            }
        }
    }
    if frames.is_empty() {
        return None;
    }

    // Exception type/message: prefer a `(ErrorClass)` tag, else `Class: msg`.
    let mut exception_type = String::new();
    let mut message = String::new();
    for line in stderr.lines() {
        if let Some(c) = re_head_paren.captures(line) {
            exception_type = c["ex"].to_string();
            message = line[..c.get(0).unwrap().start()]
                .trim()
                .trim_end_matches(':')
                .trim()
                .to_string();
            break;
        }
    }
    if exception_type.is_empty() {
        for line in stderr.lines() {
            if let Some(c) = re_head_colon.captures(line) {
                exception_type = c["ex"].to_string();
                message = c["msg"].to_string();
                break;
            }
        }
    }

    let (file, line) = frames
        .first()
        .map(|f| (f.file.clone(), f.line))
        .unwrap_or_default();
    Some(StackTrace {
        language: "Ruby".into(),
        exception_type,
        message,
        file,
        line,
        stack_frames: frames,
    })
}

// ── PHP 8.3 (Zend) ──────────────────────────────────────────────────────────
// Header: `Fatal error: Uncaught <Class>: <msg> in <file>:<line>`
// Frames: `#0 /path/file.php(139): Class::method(args)`  /  `#N {main}`
pub fn parse_php(stderr: &str) -> Option<StackTrace> {
    let re_head = Regex::new(
        r"Uncaught (?P<ex>[A-Za-z0-9_\\]+): (?P<msg>.+?) in (?P<file>[^:]+):(?P<line>\d+)",
    )
    .ok()?;
    let re_frame =
        Regex::new(r"^#(?P<idx>\d+) (?P<file>.+?)\((?P<line>\d+)\): (?P<fn>.+)$").ok()?;
    let re_internal = Regex::new(r"^#(?P<idx>\d+) \[internal function\]: (?P<fn>.+)$").ok()?;

    let (mut exception_type, mut message, mut top_file, mut top_line) =
        (String::new(), String::new(), String::new(), 0u32);
    if let Some(c) = re_head.captures(stderr) {
        exception_type = c["ex"].trim_start_matches('\\').to_string();
        message = c["msg"].to_string();
        top_file = c["file"].to_string();
        top_line = c["line"].parse().unwrap_or(0);
    } else if !stderr.contains("Stack trace:") {
        return None;
    }

    let mut frames = Vec::new();
    for line in stderr.lines() {
        if let Some(c) = re_frame.captures(line) {
            frames.push(ParsedFrame {
                file: c["file"].to_string(),
                line: c["line"].parse().unwrap_or(0),
                column: None,
                function: c["fn"].to_string(),
                module: None,
            });
        } else if let Some(c) = re_internal.captures(line) {
            frames.push(ParsedFrame {
                file: "[internal function]".into(),
                line: 0,
                column: None,
                function: c["fn"].to_string(),
                module: None,
            });
        }
        if frames.len() >= MAX_FRAMES {
            break;
        }
    }

    if exception_type.is_empty() && frames.is_empty() {
        return None;
    }
    if top_file.is_empty() {
        if let Some(f) = frames.first() {
            top_file = f.file.clone();
            top_line = f.line;
        }
    }
    Some(StackTrace {
        language: "PHP".into(),
        exception_type,
        message,
        file: top_file,
        line: top_line,
        stack_frames: frames,
    })
}

// ── .NET 8 (CoreCLR) ────────────────────────────────────────────────────────
// Header: `Unhandled exception. <Type>: <msg>`   inner via `---> <Type>: <msg>`
// Frames: `   at NS.Class.Method(params)[ in <file>:line <n>]`
pub fn parse_dotnet(stderr: &str) -> Option<StackTrace> {
    let re_head =
        Regex::new(r"Unhandled exception\. (?P<ex>[A-Za-z0-9_\.]+(?:Exception)): (?P<msg>.+)").ok()?;
    let re_inner =
        Regex::new(r"---> (?P<ex>[A-Za-z0-9_\.]+(?:Exception)): (?P<msg>.+)").ok()?;
    let re_frame = Regex::new(
        r"^\s*at (?P<fn>[^\n]+?)(?: in (?P<file>.+):line (?P<line>\d+))?\s*$",
    )
    .ok()?;

    let mut exception_type = String::new();
    let mut message = String::new();
    if let Some(c) = re_head.captures(stderr) {
        exception_type = c["ex"].to_string();
        message = c["msg"].trim().to_string();
    }
    // The innermost exception is the most actionable root cause.
    if let Some(c) = re_inner.captures(stderr) {
        exception_type = c["ex"].to_string();
        message = c["msg"].trim().to_string();
    }
    if exception_type.is_empty() {
        return None;
    }

    let mut frames = Vec::new();
    for line in stderr.lines() {
        if !line.trim_start().starts_with("at ") {
            continue;
        }
        if let Some(c) = re_frame.captures(line) {
            frames.push(ParsedFrame {
                file: c.name("file").map(|m| m.as_str().to_string()).unwrap_or_default(),
                line: c.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                column: None,
                function: c["fn"].trim().to_string(),
                module: None,
            });
        }
        if frames.len() >= MAX_FRAMES {
            break;
        }
    }

    let (file, line) = frames
        .iter()
        .find(|f| !f.file.is_empty())
        .map(|f| (f.file.clone(), f.line))
        .unwrap_or_default();
    Some(StackTrace {
        language: ".NET".into(),
        exception_type,
        message,
        file,
        line,
        stack_frames: frames,
    })
}

// ── Deno / V8 ───────────────────────────────────────────────────────────────
// Header: `error: Uncaught (in promise) <Type>: <msg>`
// Frames: `at fn (file:///path.ts:29:20)`  /  `at async file:///x.ts:3:1`
pub fn parse_deno(stderr: &str) -> Option<StackTrace> {
    let re_head = Regex::new(
        r"error: Uncaught (?:\(in promise\) )?(?P<ex>[A-Za-z_][A-Za-z0-9_]*): (?P<msg>.+)",
    )
    .ok()?;
    let re_frame_paren =
        Regex::new(r"^\s*at (?:async )?(?P<fn>[^\n(]+?) \((?P<loc>[^)\n]+)\)\s*$").ok()?;
    let re_frame_bare = Regex::new(
        r"^\s*at (?:async )?(?P<loc>(?:file://|https://|ext:|node:|deno:)[^\s)]+)\s*$",
    )
    .ok()?;

    let (mut exception_type, mut message) = (String::new(), String::new());
    if let Some(c) = re_head.captures(stderr) {
        exception_type = c["ex"].to_string();
        message = c["msg"].trim().to_string();
    } else {
        return None;
    }

    let mut frames = Vec::new();
    for line in stderr.lines() {
        let (func, loc) = if let Some(c) = re_frame_paren.captures(line) {
            (c["fn"].trim().to_string(), c["loc"].to_string())
        } else if let Some(c) = re_frame_bare.captures(line) {
            (String::new(), c["loc"].to_string())
        } else {
            continue;
        };
        let (file, ln, col) = split_loc(&loc);
        frames.push(ParsedFrame {
            file,
            line: ln,
            column: col,
            function: func,
            module: None,
        });
        if frames.len() >= MAX_FRAMES {
            break;
        }
    }

    let (file, line) = frames
        .first()
        .map(|f| (f.file.clone(), f.line))
        .unwrap_or_default();
    Some(StackTrace {
        language: "Deno".into(),
        exception_type,
        message,
        file,
        line,
        stack_frames: frames,
    })
}

// ── C/C++ runtime: GDB backtrace + AddressSanitizer ─────────────────────────
// GDB header: `Thread 1 "x" received signal SIGSEGV, Segmentation fault.`
// GDB frame:  `#1  0x0000... in main (argc=2, ...) at view.cpp:452`
// ASan head:  `==535852==ERROR: AddressSanitizer: heap-use-after-free ...`
pub fn parse_cpp(stderr: &str) -> Option<StackTrace> {
    let re_gdb_head =
        Regex::new(r#"received signal (?P<sig>SIG[A-Z]+), (?P<msg>[^\n]+)"#).ok()?;
    let re_asan_head =
        Regex::new(r"ERROR: AddressSanitizer: (?P<kind>[A-Za-z0-9_-]+)").ok()?;
    let re_gdb_frame = Regex::new(
        r"^#(?P<idx>\d+)\s+(?:0x[0-9a-fA-F]+ in )?(?P<fn>.+?)(?: from (?P<lib>\S+)| at (?P<file>[^:\s]+):(?P<line>\d+))?\s*$",
    )
    .ok()?;
    let re_asan_frame = Regex::new(
        r"^\s*#(?P<idx>\d+) 0x[0-9a-fA-F]+ in (?P<fn>\S+) (?P<file>[^:\s]+):(?P<line>\d+)",
    )
    .ok()?;

    let (mut exception_type, mut message) = (String::new(), String::new());
    if let Some(c) = re_asan_head.captures(stderr) {
        exception_type = format!("AddressSanitizer: {}", &c["kind"]);
        message = stderr
            .lines()
            .find(|l| l.contains("AddressSanitizer:"))
            .unwrap_or("")
            .trim()
            .to_string();
    } else if let Some(c) = re_gdb_head.captures(stderr) {
        exception_type = c["sig"].to_string();
        message = c["msg"].trim().to_string();
    } else {
        return None;
    }

    let mut frames = Vec::new();
    let asan = exception_type.starts_with("AddressSanitizer");
    for line in stderr.lines() {
        let caps = if asan {
            re_asan_frame.captures(line)
        } else {
            // only treat lines beginning with '#<n>' as gdb frames
            if line.trim_start().starts_with('#') {
                re_gdb_frame.captures(line)
            } else {
                None
            }
        };
        if let Some(c) = caps {
            frames.push(ParsedFrame {
                file: c.name("file").map(|m| m.as_str().to_string()).unwrap_or_default(),
                line: c.name("line").and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
                column: None,
                function: c.name("fn").map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                module: c.name("lib").map(|m| m.as_str().to_string()),
            });
        }
        if frames.len() >= MAX_FRAMES {
            break;
        }
    }

    let (file, line) = frames
        .iter()
        .find(|f| !f.file.is_empty())
        .map(|f| (f.file.clone(), f.line))
        .unwrap_or_default();
    Some(StackTrace {
        language: "C/C++".into(),
        exception_type,
        message,
        file,
        line,
        stack_frames: frames,
    })
}

/// Split a V8/Deno locator like `file:///x/y.ts:29:20`, `node:crypto:32:10`,
/// or `ext:deno_node/internal/crypto/cipher.ts:150:21` into (file, line, col).
fn split_loc(loc: &str) -> (String, u32, Option<u32>) {
    let parts: Vec<&str> = loc.rsplitn(3, ':').collect(); // [col, line, file]
    if parts.len() == 3 {
        if let (Ok(col), Ok(line)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            return (parts[2].to_string(), line, Some(col));
        }
    }
    let p2: Vec<&str> = loc.rsplitn(2, ':').collect(); // [line, file]
    if p2.len() == 2 {
        if let Ok(line) = p2[0].parse::<u32>() {
            return (p2[1].to_string(), line, None);
        }
    }
    (loc.to_string(), 0, None)
}
