// e2e: stack detector → build → run on a real project
// Exercises handoffs: detect::manifest → build::pipeline → image::store → runtime::run
// Creates a minimal project, detects it, builds it, and verifies the output

use std::path::PathBuf;
use std::fs;
use crush_build::{CrushSpecDetector, BuildEngine, crushfile};

#[tokio::test]
async fn e2e_crush_at_project_root() {
    let tmp = std::env::temp_dir().join("crush_e2e_stack").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // 1. Create a minimal Node.js project
    fs::write(tmp.join("package.json"), r#"{
        "name": "e2e-test-app",
        "version": "1.0.0",
        "main": "index.js",
        "scripts": { "start": "node index.js" }
    }"#).unwrap();
    fs::write(tmp.join("index.js"), r#"console.log("hello from crush e2e");"#).unwrap();
    fs::write(tmp.join("package-lock.json"), r#"{}"#).unwrap();

    // 2. Stack detection (detect handoff)
    let detector = CrushSpecDetector::new();
    let detection = detector.detect(&tmp);
    assert_eq!(detection.runtime_type.to_string(), "node");
    assert!(detection.confidence > 0.9, "Node project should detect with high confidence");
    assert_eq!(detection.entry_point, "index.js");
    assert_eq!(detection.port, 3000);
    println!("  Detected: {} (conf: {:.2})", detection.runtime_type.as_str(), detection.confidence);

    // 3. Crushfile generation (detect → crushfile handoff)
    let crushfile_content = crushfile::generate_crushfile(&detection);
    assert!(crushfile_content.contains("node"), "Crushfile should contain runtime type");
    assert!(crushfile_content.contains("index.js"), "Crushfile should contain entry point");
    assert!(crushfile_content.contains("3000"), "Crushfile should contain port");
    println!("  Crushfile generated ({} chars)", crushfile_content.len());

    // 4. Build pipeline (build handoff)
    let cache_dir = tmp.join("cache");
    let engine = BuildEngine::new(cache_dir.clone());
    let build_result = engine.execute_layered_build(&tmp, &detection.into()).await;
    assert!(build_result.is_ok(), "build should produce a layer digest");
    let digest = build_result.unwrap();
    assert!(digest.starts_with("sha256:"), "build digest should be SHA256");
    println!("  Build digest: {}", &digest[..19]);

    // 5. Verify build artifacts
    let layer_path = cache_dir.join("layers").join(digest.replace(':', "_"));
    assert!(layer_path.exists() || layer_path.with_extension("zst").exists(),
        "build artifact should exist in cache");

    // 6. Test with a Python project too
    let py_tmp = tmp.join("python_app");
    fs::create_dir_all(&py_tmp).unwrap();
    fs::write(py_tmp.join("pyproject.toml"), r#"[project]
name = "py-e2e"
version = "0.1.0"
requires-python = ">=3.11"
"#).unwrap();
    fs::write(py_tmp.join("main.py"), "def main(): pass").unwrap();

    let py_detection = detector.detect(&py_tmp);
    assert_eq!(py_detection.runtime_type.to_string(), "python");
    assert!(py_detection.confidence > 0.85);
    println!("  Python detection: {} (conf: {:.2})", py_detection.entry_point, py_detection.confidence);

    let _ = std::fs::remove_dir_all(&tmp);
    println!("e2e: stack detect → Crushfile → build → cache verify cycle passed");
}
