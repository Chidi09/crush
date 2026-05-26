use crush_ai::parsers::{parse_nodejs, parse_python, parse_rust_panic};
use crush_ai::offline::OfflinePatterns;
use crush_ai::source::extract_context;

#[test]
fn test_parse_nodejs() {
    let trace_str = "TypeError: Cannot read property 'foo' of undefined\n    at Object.<anonymous> (/app/index.js:5:10)\n    at Module._compile (node:internal/modules/cjs/loader:1101:14)";
    let parsed = parse_nodejs(trace_str);
    assert!(parsed.is_some());
    let t = parsed.unwrap();
    assert_eq!(t.language, "Node");
    assert_eq!(t.exception_type, "TypeError");
    assert_eq!(t.file, "/app/index.js");
    assert_eq!(t.line, 5);
    assert_eq!(t.stack_frames.len(), 2);
    assert_eq!(t.stack_frames[0].column, Some(10));
}

#[test]
fn test_parse_python() {
    let trace_str = "Traceback (most recent call last):\n  File \"/app/main.py\", line 12, in <module>\n    func()\n  File \"/app/utils.py\", line 4, in func\n    return 1 / 0\nZeroDivisionError: division by zero";
    let parsed = parse_python(trace_str);
    assert!(parsed.is_some());
    let t = parsed.unwrap();
    assert_eq!(t.language, "Python");
    assert_eq!(t.exception_type, "ZeroDivisionError");
    assert_eq!(t.file, "/app/utils.py");
    assert_eq!(t.line, 4);
    assert_eq!(t.stack_frames.len(), 2);
}

#[test]
fn test_parse_rust_panic() {
    let trace_str = "thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', src/main.rs:10:15\nstack backtrace:\n   0: std::panicking::begin_panic\n   1: crush::main\n             at src/main.rs:10";
    let parsed = parse_rust_panic(trace_str);
    assert!(parsed.is_some());
    let t = parsed.unwrap();
    assert_eq!(t.language, "Rust");
    assert_eq!(t.exception_type, "Panic");
    assert_eq!(t.file, "src/main.rs");
    assert_eq!(t.line, 10);
}

#[test]
fn test_offline_patterns() {
    let patterns = OfflinePatterns::new();
    let res = patterns.match_stderr("Error: Cannot find module './foo'");
    assert!(res.is_some());
    let r = res.unwrap();
    assert!(r.root_cause.contains("module"));
}

#[test]
fn test_extract_context() {
    let tmp = std::env::temp_dir().join(format!("test_extract_{}.txt", std::process::id()));
    std::fs::write(&tmp, "line1\nline2\nline3\nline4\nline5").unwrap();

    let ctx = extract_context(&tmp.to_string_lossy(), 3, 1);
    assert!(ctx.is_some());
    let c = ctx.unwrap();
    assert_eq!(c.before, vec!["line2".to_string()]);
    assert_eq!(c.target_line, "line3".to_string());
    assert_eq!(c.after, vec!["line4".to_string()]);

    let _ = std::fs::remove_file(&tmp);
}
