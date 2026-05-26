use crush_tui::watch::{classify_change, ChangeClass};
use std::path::Path;

#[test]
fn test_classify_change_hardcoded() {
    // Test exact hardcoded mappings for change classification
    assert_eq!(classify_change(Path::new("src/main.rs")), ChangeClass::SourceOnly);
    assert_eq!(classify_change(Path::new("crates/crush-cli/src/main.rs")), ChangeClass::SourceOnly);
    
    assert_eq!(classify_change(Path::new("Cargo.lock")), ChangeClass::LockfileChanged);
    assert_eq!(classify_change(Path::new("package-lock.json")), ChangeClass::LockfileChanged);
    assert_eq!(classify_change(Path::new("pnpm-lock.yaml")), ChangeClass::LockfileChanged);

    assert_eq!(classify_change(Path::new("Crushfile")), ChangeClass::CrushfileChanged);
    assert_eq!(classify_change(Path::new("crushfile.toml")), ChangeClass::CrushfileChanged);

    assert_eq!(classify_change(Path::new("README.md")), ChangeClass::Unknown);
    assert_eq!(classify_change(Path::new("license.txt")), ChangeClass::Unknown);
}
