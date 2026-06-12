use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use regex::Regex;
use crate::version::VersionResolver;
use crate::env::EnvDetector;
use crush_compat::{DockerfileParserV2, DockerInstruction, ComposeParser};

#[derive(Debug, Default, Clone)]
pub struct DockerfileHints {
    pub base_image: Option<String>,
    pub port: Option<u16>,
    pub entry_point: Option<String>,
    pub env_required: Vec<String>,
    pub env_optional: Vec<String>,
    pub env_secrets: Vec<String>,
    pub runtime_type: Option<RuntimeType>,
    pub runtime_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeType {
    Node, TypeScript, Python, Rust, Go, Java, DotNet, Ruby, Php, Elixir, Swift, Deno, Bun, Generic,
}

impl RuntimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::Node => "node",
            RuntimeType::TypeScript => "typescript",
            RuntimeType::Python => "python",
            RuntimeType::Rust => "rust",
            RuntimeType::Go => "go",
            RuntimeType::Java => "java",
            RuntimeType::DotNet => "dotnet",
            RuntimeType::Ruby => "ruby",
            RuntimeType::Php => "php",
            RuntimeType::Elixir => "elixir",
            RuntimeType::Swift => "swift",
            RuntimeType::Deno => "deno",
            RuntimeType::Bun => "bun",
            RuntimeType::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubService {
    pub name: String,
    pub path: String,
    pub runtime_type: String,
    pub port: u16,
    #[serde(default)]
    pub entry_point: String,
    #[serde(default)]
    pub dev_entry_point: String,
    #[serde(default)]
    pub build_command: String,
    #[serde(default)]
    pub dev_install_command: String,
    #[serde(default)]
    pub env_required: Vec<String>,
    #[serde(default)]
    pub env_optional: Vec<String>,
    #[serde(default)]
    pub env_secrets: Vec<String>,
    #[serde(default)]
    pub original_port: u16,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub project_name: String,
    pub runtime_type: RuntimeType,
    pub runtime_version: String,
    pub framework_name: String,
    pub framework_detected: bool,
    pub build_command: String,
    pub entry_point: String,
    #[serde(default)]
    pub dev_entry_point: String,
    #[serde(default)]
    pub dev_install_command: String,
    pub port: u16,
    pub confidence: f32,
    pub env_required: Vec<String>,
    pub env_optional: Vec<String>,
    pub env_secrets: Vec<String>,
    pub is_monorepo: bool,
    pub services: Vec<SubService>,
    pub dockerfile_found: Option<String>,
    pub base_image: String,
    /// For Generic fallback: immediate child dirs that look like real projects
    /// (have a package.json / Cargo.toml / etc). Empty otherwise.
    #[serde(default)]
    pub generic_subdir_hint: Vec<String>,
    /// High-level classification for UI treatment / messaging:
    /// "turbo" | "fullstack" | "spa" | "backend" | "" (unknown).
    /// Computed authoritatively here so the GUI doesn't re-guess.
    #[serde(default)]
    pub stack_kind: String,
    #[serde(default)]
    pub external_services: Vec<crate::env::ExternalService>,
}

struct Signals {
    scores: std::collections::HashMap<String, f32>,
}

impl Signals {
    fn new() -> Self { Self { scores: std::collections::HashMap::new() } }
    fn add(&mut self, framework: &str, weight: f32) {
        *self.scores.entry(framework.to_string()).or_insert(0.0) += weight;
    }
    fn winner(&self) -> Option<(&str, f32)> {
        self.scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.as_str(), *v))
    }
}

pub struct CrushSpecDetector {
    pub skip_multiservice: bool,
}

impl CrushSpecDetector {
    pub fn new() -> Self { Self { skip_multiservice: false } }
    pub fn sub() -> Self { Self { skip_multiservice: true } }

    pub fn detect(&self, root: &PathBuf) -> Detection {
        if !root.exists() {
            return self.fallback(root);
        }

        // 1. Crushfile
        let crushfile_path = root.join("Crushfile");
        if crushfile_path.exists() {
            if let Ok(cf) = crate::parser::CrushfileParser::parse(&crushfile_path) {
                let project_name = cf.project.as_ref().and_then(|p| p.name.clone()).unwrap_or_else(|| {
                    root.file_name().unwrap_or_default().to_string_lossy().to_string()
                });
                let runtime_str = cf.project.as_ref().and_then(|p| p.runtime.clone()).unwrap_or_default();
                let runtime_type = match runtime_str.to_lowercase().as_str() {
                    "node" => RuntimeType::Node,
                    "typescript" | "ts" => RuntimeType::TypeScript,
                    "python" | "py" => RuntimeType::Python,
                    "rust" | "rs" => RuntimeType::Rust,
                    "go" => RuntimeType::Go,
                    "java" => RuntimeType::Java,
                    "dotnet" => RuntimeType::DotNet,
                    "ruby" => RuntimeType::Ruby,
                    "php" => RuntimeType::Php,
                    "elixir" => RuntimeType::Elixir,
                    "swift" => RuntimeType::Swift,
                    "deno" => RuntimeType::Deno,
                    "bun" => RuntimeType::Bun,
                    _ => RuntimeType::Generic,
                };
                let build_command = cf.build.as_ref().and_then(|b| b.command.clone()).unwrap_or_default();
                let entry_point = cf.build.as_ref().and_then(|b| b.entry.clone()).unwrap_or_default();
                let port = cf.build.as_ref().and_then(|b| b.port).unwrap_or(8080);
                let base_image = cf.build.as_ref().and_then(|b| b.base.clone()).unwrap_or_else(|| "ubuntu:22.04".to_string());

                let mut env_required = Vec::new();
                let mut env_optional = Vec::new();
                let mut env_secrets = Vec::new();
                if let Some(ref sec_list) = cf.secrets {
                    for s in sec_list {
                        env_secrets.push(s.id.clone());
                    }
                }
                if let Some(ref env_map) = cf.env {
                    for (k, v) in env_map {
                        if v.is_empty() {
                            env_required.push(k.clone());
                        } else {
                            env_optional.push(k.clone());
                        }
                    }
                }

                return Detection {
                    project_name,
                    runtime_type,
                    runtime_version: "latest".to_string(),
                    framework_name: cf.project.as_ref().and_then(|p| p.project_type.clone()).unwrap_or_default(),
                    framework_detected: true,
                    build_command,
                    entry_point,
                    dev_entry_point: String::new(),
                    dev_install_command: String::new(),
                    port,
                    confidence: 1.0,
                    env_required,
                    env_optional,
                    env_secrets,
                    is_monorepo: false,
                    services: Vec::new(),
                    dockerfile_found: None,
                    base_image,
                    generic_subdir_hint: Vec::new(),
                    external_services: Vec::new(),
                    stack_kind: String::new(),
                };
            }
        }

        // 2. Compose file
        if !self.skip_multiservice {
            if let Some(compose_path) = Self::find_compose_file(root) {
                if let Ok(content) = std::fs::read_to_string(&compose_path) {
                    let interpolated = Self::interpolate_env(&content);
                    let parser = ComposeParser::new();
                    if let Ok(compose) = parser.parse(&interpolated, &compose_path) {
                        let mut sub_services = Vec::new();
                        if let Some(ref services_map) = compose.services {
                            for (name, svc) in services_map {
                                // Handle profiles
                                if let Some(ref profiles) = svc.profiles {
                                    let active_profiles_str = std::env::var("COMPOSE_PROFILES").unwrap_or_default();
                                    let active_profiles: Vec<&str> = active_profiles_str.split(',').map(|s| s.trim()).collect();
                                    let matches_active = profiles.iter().any(|p| active_profiles.contains(&p.as_str()));
                                    if !matches_active {
                                        continue;
                                    }
                                }

                                let build_ctx = if let Some(ref build_val) = svc.build {
                                    if let Some(ctx_str) = build_val.as_str() {
                                        Some(ctx_str.to_string())
                                    } else if let Some(map) = build_val.as_object() {
                                        map.get("context").and_then(|c| c.as_str()).map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                if let Some(ctx) = build_ctx {
                                    let service_path = root.join(&ctx);
                                    if service_path.is_dir() {
                                        let sub_detector = CrushSpecDetector::sub();
                                        let det = sub_detector.detect(&service_path);

                                        let mut port = det.port;
                                        if let Some(ref ports_list) = svc.ports {
                                            for p_str in ports_list {
                                                if let Some((hp, _)) = Self::parse_compose_port_pair(p_str) {
                                                    port = hp;
                                                    break;
                                                }
                                            }
                                        }

                                        let mut env_required = det.env_required.clone();
                                        let mut env_optional = det.env_optional.clone();
                                        let mut env_secrets = det.env_secrets.clone();

                                        if let Some(ref env_val) = svc.environment {
                                            if let Some(map) = env_val.as_object() {
                                                for (k, v) in map {
                                                    let is_empty = v.as_str().map(|s| s.is_empty()).unwrap_or(true);
                                                    let upper = k.to_uppercase();
                                                    let is_secret = upper.contains("SECRET")
                                                        || upper.contains("PASSWORD")
                                                        || upper.contains("TOKEN")
                                                        || upper.contains("KEY")
                                                        || upper.contains("PASS");
                                                    if is_secret {
                                                        if !env_secrets.contains(k) { env_secrets.push(k.clone()); }
                                                    } else if is_empty {
                                                        if !env_required.contains(k) { env_required.push(k.clone()); }
                                                    } else {
                                                        if !env_optional.contains(k) { env_optional.push(k.clone()); }
                                                    }
                                                }
                                            } else if let Some(arr) = env_val.as_array() {
                                                for item in arr {
                                                    if let Some(s) = item.as_str() {
                                                        if let Some((k, v)) = s.split_once('=') {
                                                            let k = k.trim().to_string();
                                                            let v = v.trim().to_string();
                                                            let upper = k.to_uppercase();
                                                            let is_secret = upper.contains("SECRET")
                                                                || upper.contains("PASSWORD")
                                                                || upper.contains("TOKEN")
                                                                || upper.contains("KEY")
                                                                || upper.contains("PASS");
                                                            if is_secret {
                                                                if !env_secrets.contains(&k) { env_secrets.push(k); }
                                                            } else if v.is_empty() {
                                                                if !env_required.contains(&k) { env_required.push(k); }
                                                            } else {
                                                                if !env_optional.contains(&k) { env_optional.push(k); }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(ref env_files) = svc.env_file {
                                            for ef in env_files {
                                                let ef_path = service_path.join(ef);
                                                if ef_path.exists() {
                                                    if let Ok(ef_content) = std::fs::read_to_string(&ef_path) {
                                                        for line in ef_content.lines() {
                                                            let line = line.trim().trim_start_matches("export ").trim_start();
                                                            if line.is_empty() || line.starts_with('#') { continue; }
                                                            if let Some((k, v)) = line.split_once('=') {
                                                                let k = k.trim().to_string();
                                                                let v = v.trim().to_string();
                                                                let upper = k.to_uppercase();
                                                                let is_secret = upper.contains("SECRET")
                                                                    || upper.contains("PASSWORD")
                                                                    || upper.contains("TOKEN")
                                                                    || upper.contains("KEY")
                                                                    || upper.contains("PASS");
                                                                if is_secret {
                                                                    if !env_secrets.contains(&k) { env_secrets.push(k); }
                                                                } else if v.is_empty() {
                                                                    if !env_required.contains(&k) { env_required.push(k); }
                                                                } else {
                                                                    if !env_optional.contains(&k) { env_optional.push(k); }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        let mut depends_on = Vec::new();
                                        if let Some(ref dep_val) = svc.depends_on {
                                            if let Some(arr) = dep_val.as_array() {
                                                for v in arr {
                                                    if let Some(s) = v.as_str() { depends_on.push(s.to_string()); }
                                                }
                                            } else if let Some(obj) = dep_val.as_object() {
                                                for k in obj.keys() { depends_on.push(k.clone()); }
                                            } else if let Some(s) = dep_val.as_str() {
                                                depends_on.push(s.to_string());
                                            }
                                        }

                                        let mut entry_point = det.entry_point.clone();
                                        if let Some(ref ep_val) = svc.entrypoint {
                                            if let Some(s) = ep_val.as_str() {
                                                entry_point = s.to_string();
                                            } else if let Some(arr) = ep_val.as_array() {
                                                entry_point = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>().join(" ");
                                            }
                                        } else if let Some(ref cmd_val) = svc.command {
                                            if let Some(s) = cmd_val.as_str() {
                                                entry_point = s.to_string();
                                            } else if let Some(arr) = cmd_val.as_array() {
                                                entry_point = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>().join(" ");
                                            }
                                        }

                                        sub_services.push(SubService {
                                            name: name.clone(),
                                            path: service_path.to_string_lossy().to_string(),
                                            runtime_type: det.runtime_type.as_str().to_string(),
                                            port,
                                            entry_point,
                                            dev_entry_point: det.dev_entry_point.clone(),
                                            build_command: det.build_command.clone(),
                                            dev_install_command: det.dev_install_command.clone(),
                                            env_required,
                                            env_optional,
                                            env_secrets,
                                            original_port: port,
                                            depends_on,
                                        });
                                    }
                                }
                            }
                        }

                        if !sub_services.is_empty() {
                            let mut env_required = Vec::new();
                            let mut env_optional = Vec::new();
                            let mut env_secrets = Vec::new();
                            for sub in &sub_services {
                                for r in &sub.env_required {
                                    if !env_required.contains(r) { env_required.push(r.clone()); }
                                }
                                for o in &sub.env_optional {
                                    if !env_optional.contains(o) { env_optional.push(o.clone()); }
                                }
                                for s in &sub.env_secrets {
                                    if !env_secrets.contains(s) { env_secrets.push(s.clone()); }
                                }
                            }

                            let project_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();

                            return Detection {
                                project_name,
                                runtime_type: RuntimeType::Generic,
                                runtime_version: "latest".to_string(),
                                framework_name: "compose".to_string(),
                                framework_detected: true,
                                build_command: String::new(),
                                entry_point: String::new(),
                                dev_entry_point: String::new(),
                                dev_install_command: String::new(),
                                port: 8080,
                                confidence: 1.0,
                                env_required,
                                env_optional,
                                env_secrets,
                                is_monorepo: true,
                                services: sub_services,
                                dockerfile_found: None,
                                base_image: "ubuntu:22.04".to_string(),
                                generic_subdir_hint: Vec::new(),
                                external_services: Vec::new(),
                                stack_kind: String::new(),
                            };
                        }
                    }
                }
            }
        }

        // 3. User Dockerfile & heuristics
        let dockerfile_path = Self::find_dockerfile(root).map(|rel| root.join(rel));
        let dockerfile_hints = dockerfile_path.as_ref().and_then(|p| Self::extract_dockerfile_hints(p));

        let mut detections: Vec<Detection> = Vec::new();

        if let Some(d) = self.try_node(root) { detections.push(d); }
        if let Some(d) = self.try_python(root) { detections.push(d); }
        if let Some(d) = self.try_rust(root) { detections.push(d); }
        if let Some(d) = self.try_go(root) { detections.push(d); }
        if let Some(d) = self.try_java(root) { detections.push(d); }
        if let Some(d) = self.try_dotnet(root) { detections.push(d); }
        if let Some(d) = self.try_ruby(root) { detections.push(d); }
        if let Some(d) = self.try_php(root) { detections.push(d); }
        if let Some(d) = self.try_elixir(root) { detections.push(d); }
        if let Some(d) = self.try_swift(root) { detections.push(d); }

        let mut best = if !detections.is_empty() {
            detections.into_iter()
                .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap()
        } else if let Some(ref hints) = dockerfile_hints {
            self.try_dockerfile(root, hints)
        } else {
            self.fallback(root)
        };

        // Apply Dockerfile overrides if present
        if let Some(ref hints) = dockerfile_hints {
            if let Some(port) = hints.port {
                best.port = port;
            }
            if let Some(ref ep) = hints.entry_point {
                best.entry_point = ep.clone();
            }
            if let Some(ref bi) = hints.base_image {
                best.base_image = bi.clone();
            }
            if let Some(ref rt) = hints.runtime_type {
                best.runtime_type = rt.clone();
            }
            if let Some(ref rv) = hints.runtime_version {
                best.runtime_version = rv.clone();
            }
            for req in &hints.env_required {
                if !best.env_required.contains(req) && !best.env_optional.contains(req) && !best.env_secrets.contains(req) {
                    best.env_required.push(req.clone());
                }
            }
            for opt in &hints.env_optional {
                if !best.env_required.contains(opt) && !best.env_optional.contains(opt) && !best.env_secrets.contains(opt) {
                    best.env_optional.push(opt.clone());
                }
            }
            for sec in &hints.env_secrets {
                if !best.env_required.contains(sec) && !best.env_optional.contains(sec) && !best.env_secrets.contains(sec) {
                    best.env_secrets.push(sec.clone());
                }
            }
        }

        let services = if self.skip_multiservice {
            Vec::new()
        } else {
            crate::multiservice::MultiServiceDetector::detect(root)
        };
        if !services.is_empty() {
            best.is_monorepo = true;
            best.services = services;
        }

        let mut env = if best.is_monorepo {
            crate::env::EnvScanResult {
                required: Vec::new(),
                optional: Vec::new(),
                secrets: Vec::new(),
            }
        } else {
            EnvDetector::scan(root)
        };

        for service in &best.services {
            for req in &service.env_required {
                if !env.required.contains(req) && !env.optional.contains(req) && !env.secrets.contains(req) {
                    env.required.push(req.clone());
                }
            }
            for opt in &service.env_optional {
                if !env.required.contains(opt) && !env.optional.contains(opt) && !env.secrets.contains(opt) {
                    env.optional.push(opt.clone());
                }
            }
            for sec in &service.env_secrets {
                if !env.required.contains(sec) && !env.optional.contains(sec) && !env.secrets.contains(sec) {
                    env.secrets.push(sec.clone());
                }
            }
        }
        best.env_required = env.required;
        best.env_optional = env.optional;
        best.env_secrets = env.secrets;

        best.dockerfile_found = Self::find_dockerfile(root);

        best.project_name = root.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        best.base_image = Self::resolve_base_image(&best);
        best.stack_kind = Self::classify(&best, root);
        best
    }

    /// Authoritative high-level classification used by the GUI for the
    /// stack-aware Run-button glow + "optimising for …" messaging.
    /// Returns "turbo" | "fullstack" | "spa" | "backend" | "".
    fn classify(d: &Detection, root: &Path) -> String {
        let f = d.framework_name.as_str();

        // Monorepo orchestrators take precedence — the whole repo is "turbo".
        let has_orchestrator = root.join("turbo.json").exists()
            || root.join("nx.json").exists()
            || root.join("lerna.json").exists();
        if matches!(f, "Turborepo" | "Nx" | "Lerna" | "Monorepo")
            || (d.is_monorepo && has_orchestrator)
        {
            return "turbo".into();
        }

        match f {
            // SSR / meta-frameworks
            "Next.js" | "Nuxt" | "SvelteKit" | "Remix" | "SolidStart" | "Qwik" | "AnalogJS" => "fullstack".into(),
            // Astro ships static by default; only server/hybrid output is fullstack.
            "Astro" => match Self::astro_output(root) {
                "server" | "hybrid" => "fullstack".into(),
                _ => "spa".into(),
            },
            // Client-only apps
            "React" | "Vue" | "Preact" | "Solid" | "Angular" | "Vite" | "Svelte" => "spa".into(),
            // Backends — no glow for now, but tagged for future messaging.
            "Express" | "Fastify" | "NestJS" | "Hono" | "Elysia" | "tRPC"
            | "FastAPI" | "Flask" | "Django" | "Tornado" | "aiohttp" | "Starlette" | "Litestar"
            | "Actix-web" | "Axum" | "Rocket" | "Warp" | "Tide"
            | "Gin" | "Echo" | "Fiber" | "Chi" | "gRPC"
            | "Spring Boot" | "Spring" | "Quarkus" | "Micronaut"
            | "Laravel" | "Symfony" | "Slim" | "CodeIgniter" | "CakePHP"
            | "Rails" | "Sinatra" | "Hanami" | "Grape" | "Phoenix" | "Vapor"
            | "ASP.NET Core" | ".NET" => "backend".into(),
            _ => String::new(),
        }
    }

    /// Astro's render target from its config: "server" / "hybrid" / "static".
    /// Defaults to "static" (Astro's default) when unspecified.
    fn astro_output(root: &Path) -> &'static str {
        for cfg in ["astro.config.mjs", "astro.config.ts", "astro.config.js", "astro.config.cjs"] {
            if let Ok(c) = fs::read_to_string(root.join(cfg)) {
                let compact: String = c.chars().filter(|ch| !ch.is_whitespace()).collect();
                if compact.contains("output:'server'") || compact.contains("output:\"server\"") {
                    return "server";
                }
                if compact.contains("output:'hybrid'") || compact.contains("output:\"hybrid\"") {
                    return "hybrid";
                }
                // An SSR adapter implies a server target even without `output:`.
                if c.contains("@astrojs/node") || c.contains("@astrojs/vercel")
                    || c.contains("@astrojs/netlify") || c.contains("@astrojs/cloudflare")
                {
                    return "server";
                }
                return "static";
            }
        }
        "static"
    }

    fn find_compose_file(root: &Path) -> Option<PathBuf> {
        let names = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
        for n in &names {
            let p = root.join(n);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    }

    fn interpolate_env(content: &str) -> String {
        let re = regex::Regex::new(r"\$\{([^}:]+)(?::-(.*?))?\}").unwrap();
        re.replace_all(content, |caps: &regex::Captures| {
            let var = &caps[1];
            let default = caps.get(2).map(|m| m.as_str());
            std::env::var(var).ok()
                .or_else(|| default.map(|s| s.to_string()))
                .unwrap_or_else(|| format!("${{{}}}", var))
        }).to_string()
    }

    fn parse_compose_port_pair(s: &str) -> Option<(u16, u16)> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 1 {
            let port = parts[0].parse::<u16>().ok()?;
            Some((port, port))
        } else if parts.len() == 2 {
            let host_port = parts[0].parse::<u16>().ok()?;
            let container_port = parts[1].parse::<u16>().ok()?;
            Some((host_port, container_port))
        } else if parts.len() == 3 {
            let host_port = parts[1].parse::<u16>().ok()?;
            let container_port = parts[2].parse::<u16>().ok()?;
            Some((host_port, container_port))
        } else {
            None
        }
    }

    fn parse_image_runtime(image: &str) -> (Option<RuntimeType>, Option<String>) {
        let parts: Vec<&str> = image.split(':').collect();
        let img_name = parts[0].split('/').last().unwrap_or(parts[0]).to_lowercase();
        let tag = if parts.len() > 1 { Some(parts[1].split('@').next().unwrap_or(parts[1]).to_string()) } else { None };

        let rt = if img_name.contains("node") { Some(RuntimeType::Node) }
            else if img_name.contains("python") { Some(RuntimeType::Python) }
            else if img_name.contains("rust") { Some(RuntimeType::Rust) }
            else if img_name.contains("golang") || img_name == "go" { Some(RuntimeType::Go) }
            else if img_name.contains("openjdk") || img_name.contains("temurin") || img_name.contains("corretto") { Some(RuntimeType::Java) }
            else if img_name.contains("dotnet") || img_name.contains("aspnet") { Some(RuntimeType::DotNet) }
            else if img_name.contains("ruby") { Some(RuntimeType::Ruby) }
            else if img_name.contains("php") { Some(RuntimeType::Php) }
            else if img_name.contains("elixir") { Some(RuntimeType::Elixir) }
            else if img_name.contains("swift") { Some(RuntimeType::Swift) }
            else if img_name.contains("bun") { Some(RuntimeType::Bun) }
            else if img_name.contains("deno") { Some(RuntimeType::Deno) }
            else { None };

        let mut ver = None;
        if let Some(ref t) = tag {
            let ver_re = regex::Regex::new(r"^(\d+(?:\.\d+)*(?:\.\d+)*)").unwrap();
            if let Some(caps) = ver_re.captures(t) {
                ver = Some(caps[1].to_string());
            }
        }
        (rt, ver)
    }

    fn extract_dockerfile_hints(path: &Path) -> Option<DockerfileHints> {
        let parser = DockerfileParserV2::new();
        let dockerfile = parser.parse_path(path).ok()?;
        let mut hints = DockerfileHints::default();

        for img in &dockerfile.base_images {
            if let (Some(rt), Some(ver)) = Self::parse_image_runtime(img) {
                hints.runtime_type = Some(rt);
                hints.runtime_version = Some(ver);
            }
        }

        if let Some(stage) = dockerfile.stages.last() {
            if let Some(ref img) = stage.base_image {
                hints.base_image = Some(img.clone());
                if let (Some(rt), Some(ver)) = Self::parse_image_runtime(img) {
                    hints.runtime_type = Some(rt);
                    hints.runtime_version = Some(ver);
                }
            }

            for instr in &stage.instructions {
                match instr {
                    DockerInstruction::Expose { ports } => {
                        for p in ports {
                            let clean_p = p.split('/').next().unwrap_or(p);
                            if let Ok(parsed) = clean_p.trim().parse::<u16>() {
                                hints.port = Some(parsed);
                            }
                        }
                    }
                    DockerInstruction::Env { pairs } => {
                        for (k, v) in pairs {
                            let upper = k.to_uppercase();
                            let is_secret = upper.contains("SECRET")
                                || upper.contains("PASSWORD")
                                || upper.contains("TOKEN")
                                || upper.contains("KEY")
                                || upper.contains("PASS");
                            if is_secret {
                                if !hints.env_secrets.contains(k) {
                                    hints.env_secrets.push(k.clone());
                                }
                            } else if v.is_empty() {
                                if !hints.env_required.contains(k) {
                                    hints.env_required.push(k.clone());
                                }
                            } else {
                                if !hints.env_optional.contains(k) {
                                    hints.env_optional.push(k.clone());
                                }
                            }
                        }
                    }
                    DockerInstruction::Cmd { args, is_json } | DockerInstruction::Entrypoint { args, is_json } => {
                        let val = if *is_json {
                            args.join(" ")
                        } else if args.len() >= 3 && args[0] == "/bin/sh" && args[1] == "-c" {
                            args[2].clone()
                        } else {
                            args.join(" ")
                        };
                        if !val.is_empty() {
                            hints.entry_point = Some(val);
                        }
                    }
                    _ => {}
                }
            }
        }
        Some(hints)
    }

    fn try_dockerfile(&self, root: &Path, hints: &DockerfileHints) -> Detection {
        let rt = hints.runtime_type.clone().unwrap_or(RuntimeType::Generic);
        let version = hints.runtime_version.clone().unwrap_or_else(|| "latest".to_string());
        let port = hints.port.unwrap_or(8080);
        let entry_point = hints.entry_point.clone().unwrap_or_default();
        let base_image = hints.base_image.clone().unwrap_or_else(|| "ubuntu:22.04".to_string());

        Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: rt,
            runtime_version: version,
            framework_name: "dockerfile".to_string(),
            framework_detected: true,
            build_command: String::new(),
            entry_point,
            dev_entry_point: String::new(),
            dev_install_command: String::new(),
            port,
            confidence: 1.0,
            env_required: hints.env_required.clone(),
            env_optional: hints.env_optional.clone(),
            env_secrets: hints.env_secrets.clone(),
            is_monorepo: false,
            services: Vec::new(),
            dockerfile_found: Self::find_dockerfile(root),
            base_image,
            generic_subdir_hint: Vec::new(),
            external_services: Vec::new(),
            stack_kind: String::new(),
        }
    }

    /// Locate a user-authored Dockerfile. Checks the canonical name plus the
    /// lowercase and Podman (`Containerfile`) variants, both at the root and
    /// in the usual infra directories. Files generated by `crush eject`
    /// (marked `# crush:eject` on line 1) are not the user's Dockerfile and
    /// are skipped so detection doesn't report our own output back to us.
    fn find_dockerfile(root: &Path) -> Option<String> {
        let dirs = ["", "infra", "docker", ".docker", "deploy", "ops", "devops"];
        let names = ["Dockerfile", "dockerfile", "Containerfile"];
        for d in &dirs {
            for n in &names {
                let rel = if d.is_empty() { n.to_string() } else { format!("{}/{}", d, n) };
                let p = root.join(d).join(n);
                if p.is_file() && !Self::is_eject_generated(&p) {
                    return Some(rel);
                }
            }
        }
        None
    }

    fn resolve_base_image(d: &Detection) -> String {
        let ver = d.runtime_version.trim();
        let unknown = ver.is_empty() || ver == "latest" || ver == "lts";
        let major = ver.split('.').next().unwrap_or(ver);
        let major_minor = {
            let parts: Vec<&str> = ver.splitn(3, '.').collect();
            if parts.len() >= 2 { format!("{}.{}", parts[0], parts[1]) } else { major.to_string() }
        };

        match d.runtime_type {
            RuntimeType::Node | RuntimeType::TypeScript => {
                if unknown { "node:lts-alpine".to_string() }
                else { format!("node:{}-alpine", major) }
            }
            RuntimeType::Bun => {
                if unknown { "oven/bun:latest".to_string() }
                else { format!("oven/bun:{}", major) }
            }
            RuntimeType::Deno => {
                if unknown { "denoland/deno:latest".to_string() }
                else { format!("denoland/deno:{}", ver) }
            }
            RuntimeType::Python => {
                if unknown { "python:3-slim".to_string() }
                else { format!("python:{}-slim", major_minor) }
            }
            RuntimeType::Rust => {
                "rust:alpine".to_string()
            }
            RuntimeType::Go => {
                if unknown { "golang:1-alpine".to_string() }
                else { format!("golang:{}-alpine", major_minor) }
            }
            RuntimeType::Java => {
                if unknown { "eclipse-temurin:21-jre-alpine".to_string() }
                else { format!("eclipse-temurin:{}-jre-alpine", major) }
            }
            RuntimeType::DotNet => {
                if unknown { "mcr.microsoft.com/dotnet/aspnet:8".to_string() }
                else { format!("mcr.microsoft.com/dotnet/aspnet:{}", major_minor) }
            }
            RuntimeType::Ruby => {
                if unknown { "ruby:3-alpine".to_string() }
                else { format!("ruby:{}-alpine", major_minor) }
            }
            RuntimeType::Php => {
                if unknown { "php:8-fpm-alpine".to_string() }
                else { format!("php:{}-fpm-alpine", major_minor) }
            }
            RuntimeType::Elixir => {
                if unknown { "elixir:alpine".to_string() }
                else { format!("elixir:{}-alpine", major_minor) }
            }
            RuntimeType::Swift => {
                if unknown { "swift:slim".to_string() }
                else { format!("swift:{}-slim", major_minor) }
            }
            RuntimeType::Generic => {
                "ubuntu:22.04".to_string()
            }
        }
    }

    fn try_node(&self, root: &Path) -> Option<Detection> {
        let pkg = root.join("package.json");
        if !pkg.exists() { return None; }
        let content = fs::read_to_string(pkg).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        let has_ts = root.join("tsconfig.json").exists();
        let has_bun = root.join("bun.lockb").exists();
        let has_deno = root.join("deno.json").exists() || root.join("deno.jsonc").exists();

        let rt = if has_deno { RuntimeType::Deno }
        else if has_bun { RuntimeType::Bun }
        else if has_ts { RuntimeType::TypeScript }
        else { RuntimeType::Node };

        let pkg_name = json["name"].as_str().unwrap_or("app");
        // package.json#version is the *package's* version, not the Node runtime
        // version. The runtime constraint lives in `engines.node`.
        let engines_node = json["engines"]["node"].as_str();
        let version = VersionResolver::resolve(root, engines_node);

        let (framework, confidence_bump) = self.detect_node_framework(&json, root);
        let framework_detected = !framework.is_empty();
        let mut confidence = if framework_detected && confidence_bump >= 0.09 {
            0.99 // direct dependency match — near-certain
        } else if has_ts {
            0.97
        } else {
            0.93
        };
        if confidence < 0.99 { confidence += confidence_bump; }

        let scripts = json["scripts"].as_object();
        let build_cmd = self.infer_node_build(&json, root, has_ts, has_deno);
        let entry = self.infer_node_entry(&json, scripts, root, has_ts);
        // Trust the dev/start script over the framework signal for port:
        // an Angular project whose `dev` is literally `vite` should report
        // 5173 (vite's port), not 4200 (ng serve's). Same for the opposite.
        let port = {
            let framework_port = Self::detect_port_framework(&framework, 3000);
            let script_port = scripts.and_then(|s| {
                for key in ["dev", "start"] {
                    if let Some(v) = s.get(key).and_then(|x| x.as_str()) {
                        let t = v.trim().to_lowercase();
                        // Look at the first token (the actual command).
                        let cmd0 = t.split_whitespace().next().unwrap_or("");
                        let p = match cmd0 {
                            "vite" => Some(5173),
                            "nuxt" => Some(3000),
                            "next" => Some(3000),
                            "astro" => Some(4321),
                            "ng" if t.contains("serve") => Some(4200),
                            "remix-serve" => Some(3000),
                            _ => None,
                        };
                        if p.is_some() { return p; }
                    }
                }
                None
            });
            script_port.unwrap_or(framework_port)
        };

        let is_monorepo = json["workspaces"].is_array()
            || json["workspaces"]["packages"].is_array()
            || root.join("pnpm-workspace.yaml").exists()
            || root.join("turbo.json").exists()
            || root.join("nx.json").exists()
            || root.join("lerna.json").exists();

        // Surface the orchestrator so the detection line reads e.g. "Node.js · Turborepo"
        let surfaced_framework = if framework.is_empty() && is_monorepo {
            if root.join("turbo.json").exists() { "Turborepo".to_string() }
            else if root.join("nx.json").exists() { "Nx".to_string() }
            else if root.join("lerna.json").exists() { "Lerna".to_string() }
            else { "Monorepo".to_string() }
        } else { framework };

        let pm = Self::pick_node_pm(root);
        let (dev_entry, dev_install, entry_prod, final_build_cmd) = if has_deno {
            ("deno task dev".to_string(), "".to_string(), entry.clone(), build_cmd)
        } else {
            let dev_entry = format!("{} run dev", pm);
            let dev_install = format!("{} install", pm);
            let framework_entry_prod = match surfaced_framework.as_str() {
                "Vite" => format!("{} exec vite preview -- --port $PORT --host 0.0.0.0", pm),
                "AnalogJS" => format!("{} exec vite preview -- --port $PORT --host 0.0.0.0", pm),
                "Angular" => format!("{} run start", pm),
                "Next.js" => format!("{} run start", pm),
                "Nuxt" => format!("{} run preview", pm),
                "SvelteKit" => "node build/index.js".to_string(),
                "Remix" => format!("{} run start", pm),
                "Astro" => format!("{} run preview", pm),
                _ => entry.clone(),
            };
            // Apply the docker-shape heuristic to the framework branch:
            // when no prod-shape Docker artifacts exist, the default `crush`
            // invocation should match the native `pnpm dev` workflow (HMR),
            // not the framework's preview/start path.
            let entry_prod = if Self::has_prod_docker_shape(root) {
                framework_entry_prod
            } else {
                dev_entry.clone()
            };
            // If we flipped entry to dev_entry (no prod-shape Docker), skip
            // the heavy `pnpm run build` step too — dev servers compile on the fly.
            let final_build_cmd = if Self::has_prod_docker_shape(root) {
                format!("{} install && {}", pm, build_cmd)
            } else {
                format!("{} install", pm)
            };
            (dev_entry, dev_install, entry_prod, final_build_cmd)
        };

        Some(Detection {
            project_name: pkg_name.to_string(),
            runtime_type: rt,
            runtime_version: version,
            framework_name: surfaced_framework.clone(),
            framework_detected: !surfaced_framework.is_empty(),
            build_command: final_build_cmd,
            entry_point: entry_prod,
            dev_entry_point: dev_entry,
            dev_install_command: dev_install,
            port,
            confidence: confidence.min(1.0),
            env_required: vec![],
            env_optional: vec![],
            env_secrets: vec![],
            is_monorepo,
            services: vec![],
            dockerfile_found: None,
            base_image: String::new(),
            generic_subdir_hint: vec![],
            stack_kind: String::new(),
            external_services: vec![],
        })
    }

    fn detect_node_framework(&self, json: &serde_json::Value, root: &Path) -> (String, f32) {
        let deps = Self::merge_deps(json);
        let dep_set: std::collections::HashSet<&str> = deps.iter().map(|s| s.as_str()).collect();
        let has_file = |name: &str| root.join(name).exists();
        let script_contains = |key: &str, needle: &str| {
            json["scripts"][key].as_str().map(|s| s.contains(needle)).unwrap_or(false)
        };

        let mut s = Signals::new();

        // High-confidence: specific config files
        if has_file("next.config.js") || has_file("next.config.ts") || has_file("next.config.mjs") { s.add("Next.js", 10.0); }
        if has_file("nuxt.config.ts") || has_file("nuxt.config.js") { s.add("Nuxt", 10.0); }
        if has_file("svelte.config.js") || has_file("svelte.config.ts") { s.add("SvelteKit", 10.0); }
        if has_file("astro.config.mjs") || has_file("astro.config.ts") { s.add("Astro", 10.0); }
        if has_file("nest-cli.json") || has_file(".nestrc") { s.add("NestJS", 10.0); }
        if has_file("remix.config.js") || has_file("remix.config.ts") { s.add("Remix", 10.0); }
        if has_file("qwik.config.ts") { s.add("Qwik", 10.0); }

        // High-confidence: explicit dependencies
        if dep_set.contains("next") { s.add("Next.js", 8.0); }
        if dep_set.contains("nuxt") { s.add("Nuxt", 8.0); }
        if dep_set.contains("@sveltejs/kit") { s.add("SvelteKit", 8.0); }
        if dep_set.contains("astro") { s.add("Astro", 8.0); }
        if dep_set.contains("@nestjs/core") { s.add("NestJS", 8.0); }
        if dep_set.contains("remix") || dep_set.contains("@remix-run/node") { s.add("Remix", 8.0); }
        if dep_set.contains("@builder.io/qwik") { s.add("Qwik", 8.0); }
        if dep_set.contains("@solidjs/start") { s.add("SolidStart", 8.0); }
        if dep_set.contains("@analogjs/platform") || dep_set.contains("@analogjs/router") { s.add("AnalogJS", 10.0); }
        if dep_set.contains("@angular/core") { s.add("Angular", 9.0); }
        if dep_set.contains("fastify") { s.add("Fastify", 8.0); }
        if dep_set.contains("express") { s.add("Express", 6.0); }
        if dep_set.contains("hono") { s.add("Hono", 8.0); }
        if dep_set.contains("elysia") { s.add("Elysia", 8.0); }
        if dep_set.contains("@trpc/server") { s.add("tRPC", 4.0); }

        // Medium-confidence: start script patterns
        if script_contains("dev", "next dev") || script_contains("start", "next start") { s.add("Next.js", 5.0); }
        if script_contains("dev", "nuxt dev") { s.add("Nuxt", 5.0); }
        if has_file("vite.config.ts") || has_file("vite.config.js") { s.add("Vite", 5.0); }
        if script_contains("dev", "fastify") || script_contains("start", "fastify") { s.add("Fastify", 4.0); }
        if script_contains("dev", "svelte-kit") { s.add("SvelteKit", 4.0); }

        // SPA view libraries. Weighted to beat a bare Vite signal (5.0) so a
        // plain React/Vue/Solid app reports its library, while every SSR
        // meta-framework above (8–10) still wins — they all depend on these.
        if dep_set.contains("react") || dep_set.contains("react-dom") { s.add("React", 6.0); }
        if dep_set.contains("react-scripts") { s.add("React", 4.0); } // Create React App
        if dep_set.contains("vue") && !dep_set.contains("nuxt") { s.add("Vue", 6.0); }
        if dep_set.contains("@vue/cli-service") { s.add("Vue", 4.0); }
        if dep_set.contains("preact") { s.add("Preact", 6.0); }
        if dep_set.contains("solid-js") && !dep_set.contains("@solidjs/start") { s.add("Solid", 6.0); }

        match s.winner() {
            Some((framework, score)) if score >= 4.0 => {
                let is_direct_dep = score >= 8.0;
                (framework.to_string(), if is_direct_dep { 0.09 } else { (score / 20.0).min(0.07) })
            }
            _ => (String::new(), 0.0),
        }
    }

    fn infer_node_build(&self, json: &serde_json::Value, root: &Path, has_ts: bool, has_deno: bool) -> String {
        if has_deno {
            return "deno task build".to_string();
        }
        if root.join("bun.lockb").exists() {
            return "bun run build".to_string();
        }

        // Detect package manager from lockfile
        let pm = if root.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if root.join("yarn.lock").exists() {
            "yarn"
        } else {
            "npm"
        };

        let scripts = json["scripts"].as_object();
        if scripts.map(|s| s.contains_key("build")).unwrap_or(false) {
            return format!("{} run build", pm);
        }
        if scripts.map(|s| s.contains_key("start")).unwrap_or(false)
            && !scripts.map(|s| s.contains_key("build")).unwrap_or(false)
        {
            return format!("{} start", pm);
        }
        if root.join("vite.config.ts").exists() || root.join("vite.config.js").exists() {
            return format!("{} run build", pm);
        }
        if has_ts && root.join("tsconfig.json").exists() {
            return format!("{} run build", pm);
        }
        format!("{} install", pm)
    }

    fn infer_node_entry(&self, json: &serde_json::Value, scripts: Option<&serde_json::Map<String, serde_json::Value>>, root: &Path, has_ts: bool) -> String {
        let pm = Self::pick_node_pm(root);

        // Decide dev vs prod entry by whether the repo ships prod-shape Docker
        // artifacts. If yes (Dockerfile or compose), match what docker would
        // run: `start`. If not (standalone repo where the alternative is
        // native `pnpm dev`), prefer `dev` so HMR works.
        let prod_shape = Self::has_prod_docker_shape(root);
        let key_order: &[&str] = if prod_shape { &["start", "dev"] } else { &["dev", "start"] };

        let is_monorepo = json["workspaces"].is_array()
            || json["workspaces"]["packages"].is_array()
            || root.join("pnpm-workspace.yaml").exists()
            || root.join("turbo.json").exists()
            || root.join("nx.json").exists()
            || root.join("lerna.json").exists();
        if is_monorepo {
            if let Some(scripts) = scripts {
                for key in key_order {
                    if scripts.get(*key).and_then(|v| v.as_str()).is_some() {
                        return format!("{} run {}", pm, key);
                    }
                }
            }
            return format!("{} start", pm);
        }

        if let Some(scripts) = scripts {
            for key in key_order {
                if let Some(v) = scripts.get(*key) {
                    let s = v.as_str().unwrap_or("");
                    if s.is_empty() { continue; }
                    if *key == "start" {
                        for prefix in ["node ", "ts-node ", "bun ", "deno ", "tsx "] {
                            if let Some(cmd) = s.strip_prefix(prefix) {
                                return cmd.trim().to_string();
                            }
                        }
                    }
                    return format!("{} run {}", pm, key);
                }
            }
        }
        if let Some(main) = json["main"].as_str() {
            if root.join(main).exists() { return main.to_string(); }
        }
        if let Some(bin) = json["bin"].as_str() {
            if root.join(bin).exists() { return bin.to_string(); }
        }
        if has_ts {
            if root.join("src/index.ts").exists() { "src/index.ts".to_string() }
            else { "dist/index.js".to_string() }
        } else {
            if root.join("index.js").exists() { "index.js".to_string() }
            else { "src/index.js".to_string() }
        }
    }

    fn pick_node_pm(root: &Path) -> &'static str {
        if root.join("bun.lockb").exists() { "bun" }
        else if root.join("pnpm-lock.yaml").exists() { "pnpm" }
        else if root.join("yarn.lock").exists() { "yarn" }
        else { "npm" }
    }

    /// Returns true when the repo ships prod-shape Docker artifacts:
    /// a non-dev Dockerfile at the root, or a compose file in one of the
    /// usual locations. Dev-shape compose (`docker-compose.dev.yml`,
    /// `compose.dev.yaml`) and `Dockerfile.dev` are ignored.
    fn has_prod_docker_shape(root: &Path) -> bool {
        if Self::find_dockerfile(root).is_some() { return true; }

        let dirs = [".", "infra", "docker", ".docker", "deploy", "ops", "devops"];
        let names = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
        for d in &dirs {
            for n in &names {
                let p = root.join(d).join(n);
                if p.exists() && !Self::is_eject_generated(&p) { return true; }
            }
        }
        false
    }

    fn is_eject_generated(path: &Path) -> bool {
        // Read only the first 256 bytes; the marker is on line 1.
        if let Ok(mut f) = fs::File::open(path) {
            use std::io::Read;
            let mut buf = [0u8; 256];
            if let Ok(n) = f.read(&mut buf) {
                return std::str::from_utf8(&buf[..n])
                    .map(|s| s.contains("# crush:eject"))
                    .unwrap_or(false);
            }
        }
        false
    }

    fn try_python(&self, root: &Path) -> Option<Detection> {
        let has_pyproject = root.join("pyproject.toml").exists();
        let has_requirements = root.join("requirements.txt").exists();
        let has_setup = root.join("setup.py").exists();
        let has_poetry = root.join("poetry.lock").exists();
        let has_uv = root.join("uv.lock").exists();
        let has_pdm = root.join("pdm.lock").exists();
        let has_conda = root.join("environment.yml").exists() || root.join("environment.yaml").exists();

        // manage.py is an unambiguous Django marker — proceed even when the
        // project ships no requirements/pyproject (deps installed via OS pkg
        // manager, system Python, etc.) so we don't get hijacked by a stray
        // package.json at the same root.
        let has_manage_py = root.join("manage.py").exists();
        if !has_pyproject && !has_requirements && !has_setup && !has_conda && !has_manage_py { return None; }

        let version = if let Some(v) = VersionResolver::from_python_version(root) {
            v
        } else if has_pyproject {
            if let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) {
                if let Ok(json) = toml::from_str::<serde_json::Value>(&content) {
                    json["project"]["requires-python"].as_str().unwrap_or("3.11").trim_start_matches(">= ").to_string()
                } else { "3.11".to_string() }
            } else { "3.11".to_string() }
        } else { "3.11".to_string() };

        let py_deps = Self::parse_python_deps(root);
        let has_py_dep = |name: &str| py_deps.iter().any(|d| d == name);

        let mut sig = Signals::new();
        if has_py_dep("fastapi") { sig.add("FastAPI", 10.0); }
        if has_py_dep("flask") { sig.add("Flask", 10.0); }
        if has_py_dep("django") { sig.add("Django", 10.0); }
        if has_py_dep("tornado") { sig.add("Tornado", 8.0); }
        if has_py_dep("aiohttp") { sig.add("aiohttp", 8.0); }
        if has_py_dep("starlette") && !has_py_dep("fastapi") { sig.add("Starlette", 7.0); }
        if has_py_dep("litestar") { sig.add("Litestar", 8.0); }
        if root.join("manage.py").exists() { sig.add("Django", 9.0); }

        let direct_dep_match = sig.winner().map(|(_, score)| score >= 8.0).unwrap_or(false);

        let (framework, entry_file, port) = match sig.winner() {
            Some(("FastAPI", _)) => ("FastAPI", "main.py", 8000),
            Some(("Flask", _)) => ("Flask", "app.py", 5000),
            Some(("Django", _)) => ("Django", "manage.py", 8000),
            Some(("Tornado", _)) => ("Tornado", "main.py", 8888),
            Some(("aiohttp", _)) => ("aiohttp", "main.py", 8080),
            Some(("Starlette", _)) => ("Starlette", "main.py", 8000),
            Some(("Litestar", _)) => ("Litestar", "app.py", 8000),
            _ => {
                // Fall back to file-name heuristics
                if root.join("manage.py").exists() { ("Django", "manage.py", 8000) }
                else if root.join("app.py").exists() && has_py_dep("flask") { ("Flask", "app.py", 5000) }
                else if root.join("main.py").exists() && has_py_dep("fastapi") { ("FastAPI", "main.py", 8000) }
                else { 
                    ("Python Script", if root.join("main.py").exists() { "main.py" } else { "app.py" }, 8080) 
                }
            }
        };

        let build_cmd = if has_uv {
            // Strip --frozen: the lockfile was generated on the Docker/CI platform
            // (usually Linux) and --frozen prevents uv from resolving native wheels
            // for the current OS, which forces source builds that can fail.
            let base = Self::extract_uv_sync_from_dockerfile(root)
                .map(|cmd| {
                    cmd.split_whitespace()
                        .filter(|w| *w != "--frozen")
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_else(|| "uv sync --no-dev --no-install-project".to_string());
            base
        } else if has_pdm {
            "pdm install --prod".to_string()
        } else if has_poetry {
            "poetry install --no-dev".to_string()
        } else if has_requirements {
            "pip install -r requirements.txt".to_string()
        } else if has_pyproject {
            "pip install -e .".to_string()
        } else if has_conda {
            "conda env create -f environment.yml".to_string()
        } else if root.join(".venv").exists() {
            "echo 'using existing .venv'".to_string()
        } else {
            "echo 'no manifest — using system python (add requirements.txt or .venv if deps are missing)'".to_string()
        };

        // Invoke venv binaries directly — avoids uv trying to reinstall the editable
        // project package (which fails when the project has no __init__.py or missing src/).
        let venv_bin = if cfg!(target_os = "windows") { r".venv\Scripts\" } else { ".venv/bin/" };
        let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
        let has_venv = root.join(".venv").exists();
        // Use the venv's python whenever a .venv exists — PATH python is
        // unrelated unless the user activated the venv themselves.
        let py = if has_uv || has_venv {
            format!("{}{}{}", venv_bin, "python", exe_suffix)
        } else {
            "python".to_string()
        };
        // Prefer the asgi target from entrypoint.sh / Dockerfile CMD if present —
        // user-defined module paths beat heuristics (e.g. src.core.main:app).
        let asgi_target = Self::detect_asgi_target(root)
            .unwrap_or_else(|| format!("{}:app", entry_file.trim_end_matches(".py")));


        let dev_install = build_cmd.clone();
        // Drop collectstatic for Django dev: runserver serves statics with
        // DEBUG=True, and projects without STATIC_ROOT/STATICFILES_DIRS
        // configured fail the step. User can add it back via Crushfile.
        let final_build_cmd = build_cmd.clone();

        let workers = if cfg!(target_os = "windows") { "" } else { " --workers 2" };
        let entry_prod = match framework {
            "FastAPI" | "Starlette" => {
                if has_uv {
                    format!("{}uvicorn{} {} --host 0.0.0.0 --port $PORT{}", venv_bin, exe_suffix, asgi_target, workers)
                } else {
                    format!("uvicorn {} --host 0.0.0.0 --port $PORT{}", asgi_target, workers)
                }
            }
            "Litestar" => {
                let module = entry_file.trim_end_matches(".py");
                if has_uv {
                    format!("{}litestar{} --app {}:app run --host 0.0.0.0 --port $PORT{}", venv_bin, exe_suffix, module, workers)
                } else {
                    format!("litestar --app {}:app run --host 0.0.0.0 --port $PORT{}", module, workers)
                }
            }
            "Flask" => {
                let module = entry_file.trim_end_matches(".py");
                if cfg!(target_os = "windows") {
                    if has_uv {
                        format!("{}flask{} --app {} run --host=0.0.0.0 --port=$PORT", venv_bin, exe_suffix, module)
                    } else {
                        format!("flask --app {} run --host=0.0.0.0 --port=$PORT", module)
                    }
                } else {
                    format!("gunicorn {}:app -b 0.0.0.0:$PORT", module)
                }
            }
            "Django" => {
                if cfg!(target_os = "windows") {
                    format!("{} manage.py runserver 0.0.0.0:$PORT", py)
                } else {
                    let project_dir_name = Self::detect_django_project_name(root)
                        .unwrap_or_else(|| root.file_name().unwrap_or_default().to_string_lossy().to_string());
                    format!("gunicorn {}.wsgi -b 0.0.0.0:$PORT", project_dir_name)
                }
            }
            _ => format!("{} {}", py, entry_file),
        };

        let dev_entry = match framework {
            "FastAPI" | "Starlette" => {
                if has_uv {
                    format!("{}uvicorn{} {} --host 0.0.0.0 --port $PORT --reload", venv_bin, exe_suffix, asgi_target)
                } else {
                    format!("uvicorn {} --host 0.0.0.0 --port $PORT --reload", asgi_target)
                }
            }
            "Litestar" => {
                let module = entry_file.trim_end_matches(".py");
                if has_uv {
                    format!("{}litestar{} --app {}:app run --host 0.0.0.0 --port $PORT", venv_bin, exe_suffix, module)
                } else {
                    format!("litestar --app {}:app run --host 0.0.0.0 --port $PORT", module)
                }
            }
            "Flask" => {
                let module = entry_file.trim_end_matches(".py");
                if has_uv {
                    format!("{}flask{} --app {} run --host=0.0.0.0 --port=$PORT", venv_bin, exe_suffix, module)
                } else {
                    format!("flask --app {} run --host=0.0.0.0 --port=$PORT", module)
                }
            }
            "Django" => format!("{} manage.py runserver 0.0.0.0:$PORT", py),
            _ => format!("{} {}", py, entry_file),
        };

        let confidence = if direct_dep_match { 0.99 }
            else if root.join("manage.py").exists() || root.join("app.py").exists() { 0.92 }
            else { 0.85 };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Python,
            runtime_version: version,
            framework_name: framework.to_string(),
            framework_detected: framework != "Python",
            build_command: final_build_cmd,
            entry_point: entry_prod,
            dev_entry_point: dev_entry,
            dev_install_command: dev_install,
            port,
            confidence,
            ..Default::default()
        })
    }

    fn detect_django_project_name(root: &Path) -> Option<String> {
        let manage_py = root.join("manage.py");
        if let Ok(content) = fs::read_to_string(manage_py) {
            let re = Regex::new(r#"DJANGO_SETTINGS_MODULE['"]\s*,\s*['"]([A-Za-z0-9_.]+)(?:\.settings)?['"]"#).ok()?;
            if let Some(caps) = re.captures(&content) {
                let full_module = &caps[1];
                if let Some(first_part) = full_module.split('.').next() {
                    return Some(first_part.to_string());
                }
            }
        }
        None
    }

    /// Greps entrypoint.sh / start.sh / Dockerfile for `uvicorn <module>:<app>`
    /// and returns the `module:app` string. Lets users define module paths
    /// (e.g. `src.core.main:app`) instead of trusting filename heuristics.
    fn detect_asgi_target(root: &Path) -> Option<String> {
        let app_re = Regex::new(r"\b([A-Za-z0-9_.]+):([A-Za-z0-9_]+)\b").ok()?;
        for candidate in &["entrypoint.sh", "start.sh", "run.sh", "Dockerfile"] {
            if let Ok(content) = fs::read_to_string(root.join(candidate)) {
                for line in content.lines() {
                    let lower = line.to_lowercase();
                    if lower.contains("uvicorn") || lower.contains("gunicorn") {
                        for caps in app_re.captures_iter(line) {
                            let val = &caps[2];
                            if !val.chars().all(|c| c.is_ascii_digit()) {
                                return Some(format!("{}:{}", &caps[1], val));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_uv_sync_from_dockerfile(root: &Path) -> Option<String> {
        let dockerfile = fs::read_to_string(root.join("Dockerfile")).ok()?;
        dockerfile.lines()
            .find_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("RUN") && trimmed.contains("uv sync") {
                    // Strip "RUN " prefix and any trailing backslash continuation
                    let cmd = trimmed.trim_start_matches("RUN").trim().trim_end_matches('\\').trim();
                    Some(cmd.to_string())
                } else {
                    None
                }
            })
    }

    fn try_rust(&self, root: &Path) -> Option<Detection> {
        let cargo = root.join("Cargo.toml");
        if !cargo.exists() { return None; }
        let content = fs::read_to_string(&cargo).ok()?;
        let json: serde_json::Value = toml::from_str(&content).ok()?;

        let manifest_ver = json["package"]["version"].as_str();
        let version = VersionResolver::resolve(root, manifest_ver);

        let bin_name = json["package"]["name"].as_str().unwrap_or("app");
        let bin_target = json["bin"].as_array()
            .and_then(|arr| arr.first())
            .and_then(|b| b["name"].as_str())
            .unwrap_or(bin_name);

        let framework = Self::detect_rust_framework(&json, root);
        let framework_detected = !framework.is_empty();
        let port = if framework.contains("Actix") { 8080 }
        else if framework.contains("Axum") { 3000 }
        else if framework.contains("Rocket") { 8000 }
        else if framework.contains("Warp") { 3030 }
        else { 8080 };

        Some(Detection {
            project_name: bin_name.to_string(),
            runtime_type: RuntimeType::Rust,
            runtime_version: version,
            framework_name: framework,
            framework_detected,
            build_command: "cargo build --release".to_string(),
            entry_point: if std::env::consts::OS == "windows" { format!("target/release/{}.exe", bin_target) } else { format!("target/release/{}", bin_target) },
            dev_entry_point: "cargo run".to_string(),
            dev_install_command: "".to_string(),
            port,
            confidence: 0.99,
            ..Default::default()
        })
    }

    fn try_go(&self, root: &Path) -> Option<Detection> {
        let gomod = root.join("go.mod");
        if !gomod.exists() { return None; }
        let content = fs::read_to_string(&gomod).ok()?;
        let go_ver = content.lines()
            .find(|l| l.starts_with("go "))
            .map(|l| l.trim_start_matches("go ").trim().to_string())
            .unwrap_or_else(|| "1.21".to_string());
        let go_ver = VersionResolver::resolve(root, Some(&go_ver));

        let module = content.lines()
            .find(|l| l.starts_with("module "))
            .map(|l| l.trim_start_matches("module ").trim())
            .unwrap_or("app");

        let bin_name = module.split('/').last().unwrap_or("app");

        let go_mod_content = fs::read_to_string(root.join("go.mod")).unwrap_or_default();
        let framework = if go_mod_content.contains("github.com/gin-gonic/gin") {
            "Gin"
        } else if go_mod_content.contains("github.com/labstack/echo") {
            "Echo"
        } else if go_mod_content.contains("github.com/gofiber/fiber") {
            "Fiber"
        } else if go_mod_content.contains("github.com/go-chi/chi") {
            "Chi"
        } else if go_mod_content.contains("google.golang.org/grpc") {
            "gRPC"
        } else { "" };

        let main_path = if root.join("cmd").is_dir() {
            "cmd/main.go"
        } else if root.join("main.go").exists() {
            "main.go"
        } else {
            "."
        };

        let go_bin = if std::env::consts::OS == "windows" { format!("{}.exe", bin_name) } else { bin_name.to_string() };
        let build_cmd = if main_path == "." {
            format!("go build -o {} .", go_bin)
        } else {
            format!("go build -o {} {}", go_bin, main_path)
        };
        let run_cmd = if main_path == "." {
            "go run .".to_string()
        } else {
            format!("go run {}", main_path)
        };

        Some(Detection {
            project_name: bin_name.to_string(),
            runtime_type: RuntimeType::Go,
            runtime_version: go_ver,
            framework_name: framework.to_string(),
            framework_detected: !framework.is_empty(),
            build_command: build_cmd,
            entry_point: if std::env::consts::OS == "windows" { format!("{}\\{}.exe", ".", bin_name) } else { format!("./{}", bin_name) },
            dev_entry_point: run_cmd,
            dev_install_command: "".to_string(),
            port: 8080,
            confidence: 0.95,
            ..Default::default()
        })
    }

    fn try_java(&self, root: &Path) -> Option<Detection> {
        let has_maven = root.join("pom.xml").exists();
        let has_gradle = root.join("build.gradle").exists() || root.join("build.gradle.kts").exists();

        if !has_maven && !has_gradle { return None; }

        let version = VersionResolver::resolve(root, None);

        let (framework, port) = if has_maven {
            if let Ok(content) = fs::read_to_string(root.join("pom.xml")) {
                if content.contains("spring-boot") { ("Spring Boot", 8080) }
                else if content.contains("quarkus") { ("Quarkus", 8080) }
                else if content.contains("micronaut") { ("Micronaut", 8080) }
                else { ("Java (Maven)", 8080) }
            } else { ("Java (Maven)", 8080) }
        } else {
            let gradle_path = if root.join("build.gradle").exists() { root.join("build.gradle") } else { root.join("build.gradle.kts") };
            if let Ok(content) = fs::read_to_string(&gradle_path) {
                if content.contains("spring-boot") || content.contains("springBoot") { ("Spring Boot", 8080) }
                else if content.contains("quarkus") { ("Quarkus", 8080) }
                else if content.contains("micronaut") { ("Micronaut", 8080) }
                else { ("Java (Gradle)", 8080) }
            } else { ("Java (Gradle)", 8080) }
        };

        // Docker-shape heuristic (same rule used for Node frameworks):
        // when no Dockerfile/compose is present we treat this as a dev
        // workflow and prefer the framework's run-from-source entry —
        // skips the 30-90s `mvn package` step and enables Spring Boot
        // DevTools hot-restart if the dep is on the classpath.
        let dev_shape = !Self::has_prod_docker_shape(root);

        let (build_cmd, entry_prod, dev_entry) = if has_maven {
            let mvn_dev = match framework {
                "Spring Boot" => "mvn spring-boot:run -Dmaven.test.skip=true".to_string(),
                "Quarkus" => "mvn quarkus:dev -Dmaven.test.skip=true".to_string(),
                "Micronaut" => "mvn mn:run -Dmaven.test.skip=true".to_string(),
                _ => "mvn spring-boot:run -Dmaven.test.skip=true".to_string(),
            };
            let mvn_prod_entry = "java -jar target/*.jar".to_string();
            let mvn_prod_build = "mvn -B package -Dmaven.test.skip=true".to_string();
            // No package step needed when we're going to spring-boot:run —
            // the plugin compiles + runs in one shot.
            let (build, entry) = if dev_shape {
                ("mvn -B compile -Dmaven.test.skip=true".to_string(), mvn_dev.clone())
            } else {
                (mvn_prod_build, mvn_prod_entry)
            };
            (build, entry, mvn_dev)
        } else {
            let gradle_dev = match framework {
                "Spring Boot" => "gradle bootRun -x test".to_string(),
                "Quarkus" => "gradle quarkusDev -x test".to_string(),
                "Micronaut" => "gradle run -x test".to_string(),
                _ => "gradle bootRun -x test".to_string(),
            };
            let (build, entry) = if dev_shape {
                ("gradle classes -x test".to_string(), gradle_dev.clone())
            } else {
                ("gradle bootJar -x test".to_string(), "java -jar build/libs/*.jar".to_string())
            };
            (build, entry, gradle_dev)
        };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Java,
            runtime_version: version,
            framework_name: framework.to_string(),
            framework_detected: framework != "Java (Maven)" && framework != "Java (Gradle)",
            build_command: build_cmd,
            entry_point: entry_prod,
            dev_entry_point: dev_entry,
            dev_install_command: "".to_string(),
            port,
            confidence: 0.99,
            ..Default::default()
        })
    }

    fn try_dotnet(&self, root: &Path) -> Option<Detection> {
        let mut csproj = Self::find_file(root, ".csproj");
        if csproj.is_none() {
            csproj = Self::find_file(&root.join("src"), ".csproj");
        }
        let has_global = root.join("global.json").exists();

        if csproj.is_none() && !has_global { return None; }

        let version = VersionResolver::resolve(root, None);
        let project_name = csproj.as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "app".to_string());

        Some(Detection {
            project_name: project_name.clone(),
            runtime_type: RuntimeType::DotNet,
            runtime_version: version,
            framework_name: if has_global { "ASP.NET Core" } else { ".NET" }.to_string(),
            framework_detected: true,
            build_command: "dotnet publish -c Release -o out".to_string(),
            entry_point: format!("dotnet out/{}.dll", project_name),
            dev_entry_point: "dotnet watch run".to_string(),
            dev_install_command: "".to_string(),
            port: 5000,
            confidence: 0.95,
            ..Default::default()
        })
    }

    fn try_ruby(&self, root: &Path) -> Option<Detection> {
        let has_gemfile = root.join("Gemfile").exists();
        let has_ruby_version = root.join(".ruby-version").exists();
        if !has_gemfile && !has_ruby_version { return None; }

        let version = VersionResolver::resolve(root, None);
        let gems = Self::parse_gemfile(root);
        let has_gem = |name: &str| gems.iter().any(|g| g == name);

        let (framework, entry, port) = if has_gem("rails") {
            ("Rails", "bundle exec rails server -b 0.0.0.0 -p 3000".to_string(), 3000)
        } else if has_gem("sinatra") {
            ("Sinatra", "bundle exec ruby app.rb".to_string(), 4567)
        } else if has_gem("hanami") {
            ("Hanami", "bundle exec hanami server".to_string(), 2300)
        } else if has_gem("grape") {
            ("Grape", "bundle exec rackup".to_string(), 9292)
        } else if root.join("config/application.rb").exists() || root.join("bin/rails").exists() {
            ("Rails", "bundle exec rails server -b 0.0.0.0 -p 3000".to_string(), 3000)
        } else {
            ("", "bundle exec ruby app.rb".to_string(), 8080)
        };

        let direct_gem = has_gem("rails") || has_gem("sinatra") || has_gem("hanami") || has_gem("grape");
        let confidence = if direct_gem { 0.99 } else if !framework.is_empty() { 0.93 } else { 0.87 };

        let dev_install = "bundle install".to_string();
        let (build_cmd, entry_prod, dev_entry) = if framework == "Rails" {
            (
                "bundle install && bundle exec rails assets:precompile".to_string(),
                "bundle exec rails assets:precompile && RAILS_ENV=production bundle exec rails server -b 0.0.0.0 -p $PORT".to_string(),
                "bundle exec rails server -b 0.0.0.0 -p $PORT".to_string()
            )
        } else {
            (
                "bundle install".to_string(),
                entry.clone(),
                entry.clone()
            )
        };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Ruby,
            runtime_version: version,
            framework_name: framework.to_string(),
            framework_detected: !framework.is_empty(),
            build_command: build_cmd,
            entry_point: entry_prod,
            dev_entry_point: dev_entry,
            dev_install_command: dev_install,
            port,
            confidence,
            ..Default::default()
        })
    }

    fn try_php(&self, root: &Path) -> Option<Detection> {
        if !root.join("composer.json").exists() { return None; }

        let deps = Self::parse_composer_deps(root);
        let has_dep = |name: &str| deps.iter().any(|d| d.contains(name));

        let mut sig = Signals::new();
        if has_dep("laravel/framework") { sig.add("Laravel", 10.0); }
        if has_dep("symfony/framework-bundle") { sig.add("Symfony", 10.0); }
        if has_dep("slim/slim") { sig.add("Slim", 8.0); }
        if has_dep("codeigniter4") { sig.add("CodeIgniter", 8.0); }
        if has_dep("cakephp/cakephp") { sig.add("CakePHP", 8.0); }
        if root.join("artisan").exists() { sig.add("Laravel", 9.0); }
        if root.join("bin/console").exists() { sig.add("Symfony", 7.0); }

        let direct_dep = sig.winner().map(|(_, score)| score >= 8.0).unwrap_or(false);

        let (framework, entry, port) = match sig.winner() {
            Some(("Laravel", _)) =>
                ("Laravel", "php artisan serve --host=0.0.0.0 --port=8000".to_string(), 8000),
            Some(("Symfony", _)) =>
                ("Symfony", "php -S 0.0.0.0:8000 -t public".to_string(), 8000),
            Some(("Slim", _)) =>
                ("Slim", "php -S 0.0.0.0:8080 -t public".to_string(), 8080),
            Some(("CodeIgniter", _)) =>
                ("CodeIgniter", "php spark serve --host=0.0.0.0 --port=8080".to_string(), 8080),
            Some(("CakePHP", _)) =>
                ("CakePHP", "bin/cake server -H 0.0.0.0 -p 8080".to_string(), 8080),
            _ => ("", "php -S 0.0.0.0:8080 -t public".to_string(), 8080),
        };

        let confidence = if direct_dep { 0.99 } else if !framework.is_empty() { 0.90 } else { 0.85 };

        let dev_install = "composer install".to_string();
        let (build_cmd, entry_prod, dev_entry) = if framework == "Laravel" {
            (
                "composer install --no-dev --optimize-autoloader && php artisan config:cache".to_string(),
                "php artisan serve --host=0.0.0.0 --port=$PORT".to_string(),
                "php artisan serve --host=0.0.0.0 --port=$PORT".to_string()
            )
        } else {
            let p_str = port.to_string();
            let entry_with_port = entry.replace(&p_str, "$PORT");
            (
                "composer install --no-dev --optimize-autoloader".to_string(),
                entry_with_port.clone(),
                entry_with_port
            )
        };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Php,
            runtime_version: VersionResolver::resolve(root, None),
            framework_name: framework.to_string(),
            framework_detected: !framework.is_empty(),
            build_command: build_cmd,
            entry_point: entry_prod,
            dev_entry_point: dev_entry,
            dev_install_command: dev_install,
            port,
            confidence,
            ..Default::default()
        })
    }

    fn try_elixir(&self, root: &Path) -> Option<Detection> {
        if !root.join("mix.exs").exists() { return None; }
        let has_phoenix = root.join("lib").is_dir()
            && Self::find_file(root, "_web.ex").is_some();
        let entry = if has_phoenix { "mix phx.server" } else { "mix run --no-halt" };
        let proj_name = root.file_name().unwrap_or_default().to_string_lossy().to_string();
        let dev_install = "mix deps.get".to_string();
        let (build_cmd, entry_prod) = if has_phoenix {
            (
                "mix deps.get && MIX_ENV=prod mix release".to_string(),
                format!("_build/prod/rel/{}/bin/{} start", proj_name, proj_name)
            )
        } else {
            (
                "mix deps.get && MIX_ENV=prod mix compile".to_string(),
                "MIX_ENV=prod mix run --no-halt".to_string()
            )
        };

        Some(Detection {
            project_name: proj_name,
            runtime_type: RuntimeType::Elixir,
            runtime_version: VersionResolver::resolve(root, None),
            framework_name: if has_phoenix { "Phoenix" } else { "" }.to_string(),
            framework_detected: has_phoenix,
            build_command: build_cmd,
            entry_point: entry_prod,
            dev_entry_point: entry.to_string(),
            dev_install_command: dev_install,
            port: if has_phoenix { 4000 } else { 8080 },
            confidence: if has_phoenix { 0.97 } else { 0.85 },
            ..Default::default()
        })
    }

    fn try_swift(&self, root: &Path) -> Option<Detection> {
        if !root.join("Package.swift").exists() { return None; }
        let has_vapor = Self::file_contains(root.join("Package.swift"), "vapor").unwrap_or(false);
        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Swift,
            runtime_version: VersionResolver::resolve(root, None),
            framework_name: if has_vapor { "Vapor" } else { "" }.to_string(),
            framework_detected: has_vapor,
            build_command: "swift build -c release".to_string(),
            entry_point: ".build/release/app".to_string(),
            dev_entry_point: "swift run".to_string(),
            dev_install_command: "".to_string(),
            port: if has_vapor { 8080 } else { 8080 },
            confidence: 0.85,
            ..Default::default()
        })
    }

    fn fallback(&self, root: &Path) -> Detection {
        let hint = Self::generic_subdir_hint(root);
        Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Generic,
            runtime_version: "latest".to_string(),
            framework_name: String::new(),
            framework_detected: false,
            build_command: "echo 'No build required'".to_string(),
            entry_point: "entrypoint.sh".to_string(),
            dev_entry_point: "entrypoint.sh".to_string(),
            dev_install_command: "".to_string(),
            port: 80,
            confidence: 0.5,
            generic_subdir_hint: hint,
            ..Default::default()
        }
    }

    /// Scan immediate child directories for project markers. If found, return
    /// a list of relative paths so the CLI can suggest "did you mean to cd into X?"
    fn generic_subdir_hint(root: &Path) -> Vec<String> {
        let markers = [
            "package.json", "Cargo.toml", "go.mod", "pyproject.toml",
            "requirements.txt", "pom.xml", "build.gradle", "build.gradle.kts",
            "Gemfile", "composer.json", "mix.exs", "Package.swift",
        ];
        let mut hits = Vec::new();
        let entries = match std::fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => return hits,
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" || name == "target" { continue; }
            let p = entry.path();
            if markers.iter().any(|m| p.join(m).exists()) {
                hits.push(name);
            }
        }
        hits.sort();
        hits
    }

    fn detect_rust_framework(json: &serde_json::Value, root: &Path) -> String {
        if let Some(deps) = json["dependencies"].as_object() {
            if deps.contains_key("actix-web") { return "Actix-web".to_string(); }
            if deps.contains_key("axum") { return "Axum".to_string(); }
            if deps.contains_key("rocket") { return "Rocket".to_string(); }
            if deps.contains_key("warp") { return "Warp".to_string(); }
            if deps.contains_key("tide") { return "Tide".to_string(); }
        }
        if root.join("src/main.rs").exists() {
            if let Ok(src) = fs::read_to_string(root.join("src/main.rs")) {
                if src.contains("use actix_web") || src.contains("extern crate actix_web") { return "Actix-web".to_string(); }
                if src.contains("use axum") { return "Axum".to_string(); }
                if src.contains("use rocket") || src.contains("extern crate rocket") { return "Rocket".to_string(); }
            }
        }
        String::new()
    }

    fn detect_port_framework(framework: &str, default: u16) -> u16 {
        match framework {
            "Express" => 3000,
            "Fastify" => 3000,
            "Next.js" => 3000,
            "Nuxt" => 3000,
            "NestJS" => 3000,
            "Hono" => 3000,
            "Remix" => 3000,
            "SvelteKit" => 5173,
            "Astro"     => 4321,
            "SolidStart"=> 3000,
            "Qwik"      => 5173,
            "Vite"      => 5173,
            "AnalogJS"  => 5173,
            "Angular"   => 4200,
            _ => default,
        }
    }

    fn merge_deps(json: &serde_json::Value) -> Vec<String> {
        let mut deps = Vec::new();
        if let Some(d) = json["dependencies"].as_object() {
            deps.extend(d.keys().cloned());
        }
        if let Some(d) = json["devDependencies"].as_object() {
            deps.extend(d.keys().cloned());
        }
        if let Some(d) = json["peerDependencies"].as_object() {
            deps.extend(d.keys().cloned());
        }
        deps
    }

    fn parse_python_deps(root: &Path) -> Vec<String> {
        let mut deps = Vec::new();

        if let Ok(content) = fs::read_to_string(root.join("requirements.txt")) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('-') { continue; }
                let name = line.split(['=', '>', '<', '!', '[', ';']).next()
                    .unwrap_or("").trim().to_lowercase();
                if !name.is_empty() { deps.push(name); }
            }
        }

        if let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) {
            if let Ok(val) = toml::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = val["project"]["dependencies"].as_array() {
                    for dep in arr {
                        if let Some(s) = dep.as_str() {
                            let name = s.split(['=', '>', '<', '!', '[', ';', ' ']).next()
                                .unwrap_or("").trim().to_lowercase();
                            if !name.is_empty() { deps.push(name); }
                        }
                    }
                }
                if let Some(obj) = val["tool"]["poetry"]["dependencies"].as_object() {
                    for key in obj.keys() {
                        if key.to_lowercase() != "python" { deps.push(key.to_lowercase()); }
                    }
                }
            }
        }

        deps
    }

    fn parse_gemfile(root: &Path) -> Vec<String> {
        let mut gems = Vec::new();
        if let Ok(content) = fs::read_to_string(root.join("Gemfile")) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("gem ") {
                    let name = line.trim_start_matches("gem ")
                        .trim_matches(&[' ', '\'', '"'][..])
                        .split(['\'', '"', ','])
                        .next().unwrap_or("").trim().to_lowercase();
                    if !name.is_empty() { gems.push(name); }
                }
            }
        }
        gems
    }

    fn parse_composer_deps(root: &Path) -> Vec<String> {
        let mut deps = Vec::new();
        if let Ok(content) = fs::read_to_string(root.join("composer.json")) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                for section in &["require", "require-dev"] {
                    if let Some(obj) = val[section].as_object() {
                        deps.extend(obj.keys().map(|k| k.to_lowercase()));
                    }
                }
            }
        }
        deps
    }

    fn file_contains(path: PathBuf, needle: &str) -> Option<bool> {
        fs::read_to_string(path).ok().map(|c| c.contains(needle))
    }

    fn find_file(root: &Path, ext: &str) -> Option<PathBuf> {
        let entries = fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == &ext[1..]).unwrap_or(false) {
                return Some(path);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_project(dir: &Path) {
        fs::write(dir.join("package.json"), r#"{"name":"app","scripts":{"dev":"vite"}}"#).unwrap();
    }

    #[test]
    fn finds_root_dockerfile() {
        let dir = tempfile::TempDir::new().unwrap();
        node_project(dir.path());
        fs::write(dir.path().join("Dockerfile"), "FROM node:20\n").unwrap();
        let d = CrushSpecDetector::new().detect(&dir.path().to_path_buf());
        assert_eq!(d.dockerfile_found.as_deref(), Some("Dockerfile"));
    }

    #[test]
    fn finds_dockerfile_in_docker_dir_and_containerfile() {
        let dir = tempfile::TempDir::new().unwrap();
        node_project(dir.path());
        fs::create_dir(dir.path().join("docker")).unwrap();
        fs::write(dir.path().join("docker/Dockerfile"), "FROM node:20\n").unwrap();
        let d = CrushSpecDetector::new().detect(&dir.path().to_path_buf());
        assert_eq!(d.dockerfile_found.as_deref(), Some("docker/Dockerfile"));

        let dir2 = tempfile::TempDir::new().unwrap();
        node_project(dir2.path());
        fs::write(dir2.path().join("Containerfile"), "FROM node:20\n").unwrap();
        let d2 = CrushSpecDetector::new().detect(&dir2.path().to_path_buf());
        assert_eq!(d2.dockerfile_found.as_deref(), Some("Containerfile"));
    }

    #[test]
    fn eject_generated_dockerfile_is_not_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        node_project(dir.path());
        fs::write(dir.path().join("Dockerfile"), "# crush:eject\nFROM node:20\n").unwrap();
        let d = CrushSpecDetector::new().detect(&dir.path().to_path_buf());
        assert_eq!(d.dockerfile_found, None, "crush's own eject output is not the user's Dockerfile");
    }

    #[test]
    fn no_dockerfile_reports_none() {
        let dir = tempfile::TempDir::new().unwrap();
        node_project(dir.path());
        let d = CrushSpecDetector::new().detect(&dir.path().to_path_buf());
        assert_eq!(d.dockerfile_found, None);
    }
}

impl Default for Detection {
    fn default() -> Self {
        Self {
            project_name: String::new(),
            runtime_type: RuntimeType::Generic,
            runtime_version: "latest".to_string(),
            framework_name: String::new(),
            framework_detected: false,
            build_command: String::new(),
            entry_point: String::new(),
            dev_entry_point: String::new(),
            dev_install_command: String::new(),
            port: 80,
            confidence: 0.0,
            env_required: vec![],
            env_optional: vec![],
            env_secrets: vec![],
            is_monorepo: false,
            services: vec![],
            dockerfile_found: None,
            base_image: "ubuntu:22.04".to_string(),
            generic_subdir_hint: vec![],
            stack_kind: String::new(),
            external_services: vec![],
        }
    }
}
