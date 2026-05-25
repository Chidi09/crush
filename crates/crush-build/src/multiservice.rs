use std::path::Path;
use std::fs;
use crate::detect::SubService;

pub struct MultiServiceDetector;

impl MultiServiceDetector {
    pub fn detect(root: &Path) -> Vec<SubService> {
        let mut services = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let well_known = ["apps/", "packages/", "services/", "backend/", "frontend/"];
        for dir in &well_known {
            let path = root.join(dir);
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let sub_path = entry.path();
                        if sub_path.is_dir() {
                            if let Some(name) = sub_path.file_name() {
                                let name_str = name.to_string_lossy().to_string();
                                if seen.insert(name_str.clone()) {
                                    let mut sub = SubService {
                                        name: name_str,
                                        path: sub_path.to_string_lossy().to_string(),
                                        runtime_type: "unknown".to_string(),
                                        port: 0,
                                    };
                                    if sub_path.join("package.json").exists() {
                                        sub.runtime_type = "node".to_string();
                                        sub.port = 3000;
                                    } else if sub_path.join("Cargo.toml").exists() {
                                        sub.runtime_type = "rust".to_string();
                                        sub.port = 8080;
                                    } else if sub_path.join("go.mod").exists() {
                                        sub.runtime_type = "go".to_string();
                                        sub.port = 8080;
                                    }
                                    services.push(sub);
                                }
                            }
                        }
                    }
                }
            }
        }

        Self::check_npm_workspaces(root, &mut services, &mut seen);
        Self::check_cargo_workspace(root, &mut services, &mut seen);

        services
    }

    fn check_npm_workspaces(root: &Path, services: &mut Vec<SubService>, seen: &mut std::collections::HashSet<String>) {
        let pkg_path = root.join("package.json");
        if !pkg_path.exists() { return; }
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(workspaces) = json["workspaces"].as_array() {
                    for ws_pattern in workspaces {
                        if let Some(pattern) = ws_pattern.as_str() {
                            let glob_pattern = root.join(pattern);
                            let dir = glob_pattern.parent().unwrap_or(root);
                            if dir.is_dir() {
                                if let Ok(entries) = fs::read_dir(dir) {
                                    for entry in entries.flatten() {
                                        let p = entry.path();
                                        let pkg_json = p.join("package.json");
                                        if pkg_json.exists() {
                                            if let Some(name) = p.file_name() {
                                                let n = name.to_string_lossy().to_string();
                                                if seen.insert(n.clone()) {
                                                    services.push(SubService {
                                                        name: n, path: p.to_string_lossy().to_string(),
                                                        runtime_type: "node".to_string(), port: 3000,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_cargo_workspace(root: &Path, _services: &mut Vec<SubService>, _seen: &mut std::collections::HashSet<String>) {
        let cargo_path = root.join("Cargo.toml");
        if !cargo_path.exists() { return; }
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if content.contains("[workspace]") {
                // Workspace is detected; individual crate detection happens at the crate level
            }
        }
    }
}
