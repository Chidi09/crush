use std::path::Path;
use std::fs;
use crate::detect::SubService;

pub struct MultiServiceDetector;

impl MultiServiceDetector {
    pub fn detect(root: &Path) -> Vec<SubService> {
        let mut services = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Pattern A: implicit monorepo — `backend/` and/or `frontend/` ARE the
        // services themselves (each contains project markers at its root).
        // Common in projects without a workspace tool (NCIC, Solexpay-style repos).
        for direct in &["backend", "frontend", "server", "client", "web", "api"] {
            let path = root.join(direct);
            if path.is_dir() {
                if let Some(sub) = Self::sub_from_dir(&path, direct) {
                    if seen.insert(sub.name.clone()) {
                        services.push(sub);
                    }
                }
            }
        }

        // Pattern B: explicit container dirs — `apps/X`, `packages/X`, `services/X`
        // are parents that hold many services; iterate their children.
        for container in &["apps", "packages", "services"] {
            let path = root.join(container);
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let sub_path = entry.path();
                        if !sub_path.is_dir() { continue; }
                        if let Some(name) = sub_path.file_name() {
                            let name_str = name.to_string_lossy().to_string();
                            if let Some(sub) = Self::sub_from_dir(&sub_path, &name_str) {
                                if seen.insert(sub.name.clone()) {
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

    /// Builds a SubService if the dir contains a recognised project marker.
    /// Picks a sensible default port per runtime + framework.
    fn sub_from_dir(path: &Path, name: &str) -> Option<SubService> {
        let (rt, port) = if path.join("package.json").exists() {
            let pkg = fs::read_to_string(path.join("package.json")).unwrap_or_default();
            let port = if pkg.contains("\"vite\"") { 5173 }
                else if pkg.contains("\"next\"") { 3000 }
                else if pkg.contains("\"@nestjs/core\"") { 3000 }
                else if pkg.contains("\"fastify\"") { 3000 }
                else if pkg.contains("\"express\"") { 3000 }
                else { 3000 };
            ("node", port)
        } else if path.join("Cargo.toml").exists() {
            ("rust", 8080)
        } else if path.join("go.mod").exists() {
            ("go", 8080)
        } else if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists() {
            ("python", 8000)
        } else if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
            ("java", 8080)
        } else if path.join("Gemfile").exists() {
            ("ruby", 3000)
        } else if path.join("composer.json").exists() {
            ("php", 8000)
        } else if path.join("mix.exs").exists() {
            ("elixir", 4000)
        } else {
            // .csproj — directory entries
            let has_csproj = std::fs::read_dir(path).ok()
                .map(|d| d.flatten().any(|e| e.path().extension()
                    .and_then(|x| x.to_str()) == Some("csproj")))
                .unwrap_or(false);
            if has_csproj { ("dotnet", 5000) } else { return None; }
        };
        Some(SubService {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            runtime_type: rt.to_string(),
            port,
        })
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
