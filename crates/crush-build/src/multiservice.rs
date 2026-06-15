use std::path::Path;
use std::fs;
use crate::detect::SubService;

pub struct MultiServiceDetector;

impl MultiServiceDetector {
    pub fn detect(root: &Path) -> Vec<SubService> {
        let mut services = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Pattern A: implicit monorepo — backend/ and/or frontend/ are the
        // services themselves (each contains project markers at its root).
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

        // Pattern B: explicit container dirs — apps/X, packages/X, services/X
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
        Self::check_pnpm_workspaces(root, &mut services, &mut seen);
        Self::check_cargo_workspace(root, &mut services, &mut seen);

        // Sort services deterministically by path to ensure consistent port assignment
        services.sort_by(|a, b| a.path.cmp(&b.path));

        // Save original ports and resolve conflicts
        let mut occupied_ports = std::collections::HashSet::new();
        for service in &mut services {
            service.original_port = service.port;
            let mut port = service.port;
            while occupied_ports.contains(&port) {
                port += 1;
            }
            service.port = port;
            occupied_ports.insert(port);
        }

        // Infer service dependency graph
        Self::infer_dependencies(&mut services);

        services
    }

    /// Builds a SubService if the dir contains a recognised project marker.
    /// Delegates to CrushSpecDetector recursively (with recursion guard skip_multiservice = true).
    fn sub_from_dir(path: &Path, name: &str) -> Option<SubService> {
        let detector = crate::detect::CrushSpecDetector::sub();
        let det = detector.detect(&path.to_path_buf());
        
        let has_marker = !matches!(det.runtime_type, crate::detect::RuntimeType::Generic)
            || det.dockerfile_found.is_some()
            || path.join("Crushfile").exists()
            || path.join("entrypoint.sh").exists();

        if !has_marker || !Self::is_runnable_service(path, &det) {
            return None;
        }

        let initial_port = Self::extract_port_from_env_or_scripts(path, det.port);

        Some(SubService {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            runtime_type: det.runtime_type.as_str().to_string(),
            port: initial_port,
            entry_point: det.entry_point,
            dev_entry_point: det.dev_entry_point,
            build_command: det.build_command,
            dev_install_command: det.dev_install_command,
            env_required: det.env_required,
            env_optional: det.env_optional,
            env_secrets: det.env_secrets,
            original_port: initial_port,
            depends_on: Vec::new(),
        })
    }

    fn is_runnable_service(path: &Path, det: &crate::detect::Detection) -> bool {
        if det.dockerfile_found.is_some() || path.join("Crushfile").exists() || path.join("entrypoint.sh").exists() {
            return true;
        }

        match det.runtime_type {
            // JS/TS packages: a monorepo's `packages/*` are usually *libraries*
            // (e.g. @scope/types with only a `build: tsc` script and a `main`
            // field) — they must NOT be run as services. Treat a package as a
            // runnable app only if it has a real (non-compiler) start/dev/serve
            // script or a server/frontend framework dependency.
            crate::detect::RuntimeType::Node
            | crate::detect::RuntimeType::TypeScript
            | crate::detect::RuntimeType::Bun
            | crate::detect::RuntimeType::Deno => Self::node_like_runnable(path),
            crate::detect::RuntimeType::Rust => {
                let has_main = path.join("src/main.rs").exists();
                let has_bin = if let Ok(cargo_content) = fs::read_to_string(path.join("Cargo.toml")) {
                    cargo_content.contains("[[bin]]")
                } else {
                    false
                };
                has_main || has_bin || det.framework_detected
            }
            crate::detect::RuntimeType::Python => {
                path.join("main.py").exists() || path.join("app.py").exists() || path.join("manage.py").exists() || det.framework_detected
            }
            crate::detect::RuntimeType::Go => {
                path.join("main.go").exists() || path.join("cmd").is_dir()
            }
            crate::detect::RuntimeType::Java => {
                det.framework_detected
            }
            crate::detect::RuntimeType::DotNet => {
                path.join("Program.cs").exists() || path.join("Startup.cs").exists() || path.join("src/Program.cs").exists()
            }
            crate::detect::RuntimeType::Generic => {
                false
            }
            // Mobile and anything else: only when a framework was actually
            // detected — never just because a synthesized entry is non-empty
            // (that wrongly promoted library packages to services).
            _ => det.framework_detected,
        }
    }

    /// A JS/TS package is a runnable app (not a library) when it has a real
    /// start/dev/serve script that isn't merely a compiler, or it depends on a
    /// server/frontend framework. Pure libraries (`build: tsc`, a `main`/
    /// `exports` field, no framework) return false.
    fn node_like_runnable(path: &Path) -> bool {
        let Ok(content) = fs::read_to_string(path.join("package.json")) else { return false };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else { return false };

        if let Some(scripts) = json["scripts"].as_object() {
            for key in ["start", "dev", "serve"] {
                if let Some(cmd) = scripts.get(key).and_then(|v| v.as_str()) {
                    if !Self::is_compiler_only(cmd) { return true; }
                }
            }
        }

        let frameworks = [
            "express", "koa", "fastify", "next", "nuxt", "react", "vue", "svelte",
            "astro", "vite", "remix", "@nestjs/core", "hono", "@angular/core",
            "react-scripts", "@sveltejs/kit", "apollo-server", "@apollo/server",
        ];
        for section in ["dependencies", "devDependencies"] {
            if let Some(deps) = json[section].as_object() {
                if frameworks.iter().any(|d| deps.contains_key(*d)) { return true; }
            }
        }
        false
    }

    /// True when a script just compiles/bundles a library (tsc, tsup, rollup…)
    /// rather than starting a long-running server.
    fn is_compiler_only(cmd: &str) -> bool {
        let first = cmd.trim().split_whitespace().next().unwrap_or("");
        matches!(first, "tsc" | "tsup" | "rollup" | "rimraf" | "rm" | "babel" | "swc" | "unbuild" | "tsdx")
    }

    fn extract_port_from_env_or_scripts(path: &Path, default_port: u16) -> u16 {
        for env_file in &[".env", ".env.local", ".env.example", ".env.development"] {
            let p = path.join(env_file);
            if p.exists() {
                if let Ok(content) = fs::read_to_string(&p) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("PORT=") {
                            if let Some(val) = line.split('=').nth(1) {
                                if let Ok(parsed) = val.trim().parse::<u16>() {
                                    return parsed;
                                }
                            }
                        }
                    }
                }
            }
        }

        let pkg_path = path.join("package.json");
        if pkg_path.exists() {
            if let Ok(content) = fs::read_to_string(&pkg_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(scripts) = json["scripts"].as_object() {
                        for (_, cmd) in scripts {
                            if let Some(cmd_str) = cmd.as_str() {
                                if let Some(p) = Self::parse_port_from_cmd(cmd_str) {
                                    return p;
                                }
                            }
                        }
                    }
                }
            }
        }
        default_port
    }

    fn parse_port_from_cmd(cmd: &str) -> Option<u16> {
        let port_re = regex::Regex::new(r"(?:--port|-p)\s+(\d+)").unwrap();
        if let Some(caps) = port_re.captures(cmd) {
            if let Ok(p) = caps[1].parse::<u16>() {
                return Some(p);
            }
        }
        let env_port_re = regex::Regex::new(r"PORT=(\d+)").unwrap();
        if let Some(caps) = env_port_re.captures(cmd) {
            if let Ok(p) = caps[1].parse::<u16>() {
                return Some(p);
            }
        }
        None
    }

    fn resolve_workspace_globs(
        root: &Path,
        patterns: &[String],
        services: &mut Vec<SubService>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        for pattern in patterns {
            let glob_pattern = if pattern.ends_with('/') {
                format!("{}{}", pattern, "*")
            } else if !pattern.contains('*') {
                format!("{}/{}", pattern, "*")
            } else {
                pattern.clone()
            };

            let full_glob = root.join(&glob_pattern).to_string_lossy().to_string();
            if let Ok(entries) = glob::glob(&full_glob) {
                for entry in entries.flatten() {
                    if entry.is_dir() {
                        if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
                            if name == "node_modules" || name == "target" || name.starts_with('.') {
                                continue;
                            }
                            if let Some(sub) = Self::sub_from_dir(&entry, name) {
                                if seen.insert(sub.name.clone()) {
                                    services.push(sub);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_npm_workspaces(root: &Path, services: &mut Vec<SubService>, seen: &mut std::collections::HashSet<String>) {
        let pkg_path = root.join("package.json");
        if !pkg_path.exists() { return; }
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(workspaces) = json["workspaces"].as_array() {
                    let patterns: Vec<String> = workspaces.iter()
                        .filter_map(|w| w.as_str().map(|s| s.to_string()))
                        .collect();
                    Self::resolve_workspace_globs(root, &patterns, services, seen);
                } else if let Some(packages) = json["workspaces"]["packages"].as_array() {
                    let patterns: Vec<String> = packages.iter()
                        .filter_map(|p| p.as_str().map(|s| s.to_string()))
                        .collect();
                    Self::resolve_workspace_globs(root, &patterns, services, seen);
                }
            }
        }
    }

    fn check_pnpm_workspaces(root: &Path, services: &mut Vec<SubService>, seen: &mut std::collections::HashSet<String>) {
        let pnpm_ws = root.join("pnpm-workspace.yaml");
        if !pnpm_ws.exists() { return; }
        if let Ok(content) = fs::read_to_string(&pnpm_ws) {
            #[derive(serde::Deserialize)]
            struct PnpmWorkspace {
                packages: Option<Vec<String>>,
            }
            if let Ok(ws) = serde_yaml::from_str::<PnpmWorkspace>(&content) {
                if let Some(packages) = ws.packages {
                    Self::resolve_workspace_globs(root, &packages, services, seen);
                }
            }
        }
    }

    fn check_cargo_workspace(root: &Path, services: &mut Vec<SubService>, seen: &mut std::collections::HashSet<String>) {
        let cargo_toml = root.join("Cargo.toml");
        if !cargo_toml.exists() { return; }
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            if let Ok(toml_val) = toml::from_str::<serde_json::Value>(&content) {
                if let Some(members) = toml_val["workspace"]["members"].as_array() {
                    let member_patterns: Vec<String> = members.iter()
                        .filter_map(|m| m.as_str().map(|s| s.to_string()))
                        .collect();
                    Self::resolve_workspace_globs(root, &member_patterns, services, seen);
                }
            }
        }
    }

    fn infer_dependencies(services: &mut [SubService]) {
        let service_infos: Vec<(String, String, u16)> = services.iter()
            .map(|s| (s.name.clone(), s.path.clone(), s.port))
            .collect();

        for service in services {
            let mut deps = Vec::new();
            let s_path = Path::new(&service.path);
            
            let mut files = Vec::new();
            Self::collect_source_files(s_path, &mut files, 0);
            
            for env_file in &[".env", ".env.local", ".env.example", ".env.development", "package.json", "Cargo.toml"] {
                let p = s_path.join(env_file);
                if p.exists() {
                    files.push(p);
                }
            }

            for file in files {
                if let Ok(content) = fs::read_to_string(&file) {
                    for (other_name, _, other_port) in &service_infos {
                        if other_name == &service.name { continue; }
                        
                        let port_pattern1 = format!("localhost:{}", other_port);
                        let port_pattern2 = format!("127.0.0.1:{}", other_port);
                        let host_pattern = format!("://{}", other_name);
                        let host_pattern_raw = format!("@{}", other_name);
                        
                        if content.contains(&port_pattern1)
                            || content.contains(&port_pattern2)
                            || content.contains(&host_pattern)
                            || content.contains(&host_pattern_raw)
                            || content.contains(other_name) && (content.contains("URL") || content.contains("HOST") || content.contains("API"))
                        {
                            if !deps.contains(other_name) {
                                deps.push(other_name.clone());
                            }
                        }
                    }
                }
            }
            service.depends_on = deps;
        }
    }

    fn collect_source_files(dir: &Path, result: &mut Vec<std::path::PathBuf>, depth: usize) {
        if depth > 10 { return; }
        let read_dir = match fs::read_dir(dir) {
            Ok(d) => d,
            Err(_) => return,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if name.starts_with('.') && name != "." && name != ".." { continue; }
                if matches!(name.as_ref(),
                    "node_modules" | "target" | "dist" | "build" | "venv" | ".venv" |
                    "__pycache__" | "obj" | "bin" | ".gradle" | "vendor" | "deps" |
                    "_build" | "out" | ".git" | ".cache"
                ) {
                    continue;
                }
                Self::collect_source_files(&path, result, depth + 1);
            } else if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "js" | "ts" | "jsx" | "tsx" | "py" | "rs" | "java" | "cs" | "go" | "rb" | "php") {
                        result.push(path);
                    }
                }
            }
        }
    }
}
