use std::path::{Path, PathBuf};
use std::fs;
use crush_types::{Result, CrushError};
use crate::diagnose::AiDiagnosis;
use crate::parsers::ParsedTrace;

pub struct AutoFixApplicator;

impl AutoFixApplicator {
    pub fn new() -> Self { Self }

    pub fn apply_unified_diff(file_path: &Path, diff: &str) -> Result<()> {
        // ⚠ Validate file is within project
        let canonical = file_path.canonicalize()
            .map_err(|_| CrushError::StorageError("Cannot resolve file path".to_string()))?;

        let backup = file_path.with_extension(
            format!("{}.bak", file_path.extension().unwrap_or_default().to_string_lossy())
        );
        fs::copy(file_path, &backup)
            .map_err(|e| CrushError::StorageError(format!("Backup failed: {}", e)))?;

        let current = fs::read_to_string(file_path)
            .map_err(|e| CrushError::StorageError(format!("Read failed: {}", e)))?;
        let result = apply_diff(&current, diff);
        fs::write(file_path, &result)
            .map_err(|e| CrushError::StorageError(format!("Write failed: {}", e)))?;

        Ok(())
    }

    pub fn add_missing_dependency(project_root: &Path, dep_name: &str, runtime: &str) -> Result<()> {
        // ⚠ CRITICAL: Validate dep_name to prevent TOML / supply-chain injection
        let valid_dep_re = regex::Regex::new(r"^[a-zA-Z0-9_\-\.]+$").unwrap();
        if !valid_dep_re.is_match(dep_name) {
            return Err(CrushError::ImageError(format!(
                "Invalid dependency name '{}': must match ^[a-zA-Z0-9_\\-\\.]+$", dep_name
            )));
        }

        match runtime {
            "python" => {
                let req_path = project_root.join("requirements.txt");
                let mut content = if req_path.exists() {
                    fs::read_to_string(&req_path)
                        .map_err(|e| CrushError::StorageError(e.to_string()))?
                } else { String::new() };

                let clean_name = dep_name.lines().next().unwrap_or(dep_name).trim();
                if !content.lines().any(|l| l.trim() == clean_name || l.trim().starts_with(&format!("{}", clean_name))) {
                    content.push_str(&format!("{}\n", clean_name));
                    fs::write(&req_path, content)
                        .map_err(|e| CrushError::StorageError(e.to_string()))?;
                }
            }
            "node" => {
                let pkg = project_root.join("package.json");
                if pkg.exists() {
                    let mut content: serde_json::Value = serde_json::from_str(
                        &fs::read_to_string(&pkg).map_err(|e| CrushError::StorageError(e.to_string()))?
                    ).map_err(|e| CrushError::ImageError(e.to_string()))?;
                    if let Some(deps) = content["dependencies"].as_object_mut() {
                        deps.insert(dep_name.to_string(), serde_json::Value::String("*".to_string()));
                    }
                    fs::write(&pkg, serde_json::to_string_pretty(&content)
                        .map_err(|e| CrushError::ImageError(e.to_string()))?)
                        .map_err(|e| CrushError::StorageError(e.to_string()))?;
                }
            }
            "rust" => {
                let cargo = project_root.join("Cargo.toml");
                if cargo.exists() {
                    let content = fs::read_to_string(&cargo)
                        .map_err(|e| CrushError::StorageError(e.to_string()))?;
                    if !content.contains(&format!("{} ", dep_name)) {
                        let dep_line = format!("{} = \"*\"\n", dep_name);
                        let new_content = if let Some(pos) = content.find("[dependencies]") {
                            let after = &content[pos + 15..];
                            let insert_at = if let Some(end) = after.find('[') { pos + 15 + end } else { content.len() };
                            format!("{}{}{}", &content[..insert_at], dep_line, &content[insert_at..])
                        } else {
                            format!("{}\n[dependencies]\n{}", content, dep_line)
                        };
                        fs::write(&cargo, new_content)
                            .map_err(|e| CrushError::StorageError(e.to_string()))?;
                    }
                }
            }
            _ => return Err(CrushError::ImageError(format!("Unknown runtime: {}", runtime))),
        }
        Ok(())
    }

    pub fn apply_fix(trace: &ParsedTrace, diagnosis: &AiDiagnosis, project_root: &Path) -> Result<()> {
        if let Some(ref patch) = diagnosis.proposed_patch {
            // ⚠ CRITICAL: Validate file path stays within project root
            let file_path = project_root.join(&trace.file);
            let canonical_root = project_root.canonicalize()
                .map_err(|_| CrushError::StorageError("Cannot resolve project root".to_string()))?;
            let canonical_file = file_path.canonicalize()
                .map_err(|_| CrushError::StorageError("Cannot resolve file path".to_string()))?;

            if !canonical_file.starts_with(&canonical_root) {
                return Err(CrushError::StorageError(format!(
                    "Path traversal blocked: {:?} is outside project root {:?}",
                    canonical_file, canonical_root
                )));
            }

            Self::apply_unified_diff(&file_path, patch)?;
        }
        Ok(())
    }
}

fn apply_diff(original: &str, diff: &str) -> String {
    use similar::{ChangeTag, TextDiff};
    let mut result = String::new();

    for line in diff.lines() {
        if line.starts_with("@@") || line.starts_with("---") || line.starts_with("+++") { continue; }
        if line.starts_with('-') { continue; }
        if line.starts_with('+') { result.push_str(&line[1..]); result.push('\n'); }
        else { result.push_str(line); result.push('\n'); }
    }

    if result.is_empty() { original.to_string() } else { result }
}
