use std::path::{Path, PathBuf};
use std::fs;
use crush_types::Result;

pub struct CrushIgnore {
    patterns: Vec<String>,
}

impl CrushIgnore {
    pub fn load(root: &Path) -> Self {
        let mut patterns = Vec::new();

        let ignore_files = [".crushignore", ".dockerignore"];
        for file in &ignore_files {
            let path = root.join(file);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            patterns.push(line.to_string());
                        }
                    }
                }
                break;
            }
        }

        Self { patterns }
    }

    pub fn default_patterns() -> Vec<String> {
        vec![
            ".git".to_string(),
            ".git/".to_string(),
            ".DS_Store".to_string(),
            "Thumbs.db".to_string(),
            "__pycache__".to_string(),
            "*.pyc".to_string(),
            ".env".to_string(),
            ".env.*".to_string(),
            ".editorconfig".to_string(),
            ".vscode".to_string(),
            ".idea".to_string(),
            "*.swp".to_string(),
            "*.swo".to_string(),
            "target/".to_string(),
            "node_modules/".to_string(),  // wrong location catch
        ]
    }

    pub fn is_ignored(&self, path: &Path, root: &Path) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let path_str = relative.to_string_lossy().replace('\\', "/");

        for pattern in &self.patterns {
            if self.match_pattern(&path_str, pattern) {
                return true;
            }
        }

        for pattern in &Self::default_patterns() {
            if self.match_pattern(&path_str, pattern) {
                return true;
            }
        }

        false
    }

    fn match_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern.starts_with('/') {
            let trimmed = pattern.trim_start_matches('/');
            return path == trimmed || path.starts_with(&format!("{}/", trimmed));
        }
        if pattern.ends_with('/') {
            let dir = pattern.trim_end_matches('/');
            return path == dir || path.starts_with(&format!("{}/", dir))
                || path.contains(&format!("/{}/", dir));
        }
        if pattern.contains('*') {
            let re_str = format!("^{}$", regex::escape(pattern)
                .replace(r"\*", ".*")
                .replace(r"\?", "."));
            if let Ok(re) = regex::Regex::new(&re_str) {
                return re.is_match(path);
            }
        }
        path == pattern
            || path.ends_with(&format!("/{}", pattern))
            || path.starts_with(&format!("{}/", pattern))
    }

    pub fn filter_entries(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let walker = walkdir::WalkDir::new(root).into_iter()
            .filter_entry(|e| !Self::is_entry_skipped(e));

        for entry in walker {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() && !self.is_ignored(entry.path(), root) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }

        Ok(files)
    }

    fn is_entry_skipped(entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_string_lossy();
        name == ".git" || name == "target" || name == "node_modules"
    }
}
