use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use regex::Regex;
use crate::version::VersionResolver;
use crate::env::EnvDetector;

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
    pub port: u16,
    pub confidence: f32,
    pub env_required: Vec<String>,
    pub env_optional: Vec<String>,
    pub env_secrets: Vec<String>,
    pub is_monorepo: bool,
    pub services: Vec<SubService>,
    pub dockerfile_found: Option<String>,
    pub base_image: String,
}

pub struct CrushSpecDetector;

impl CrushSpecDetector {
    pub fn new() -> Self { Self }

    pub fn detect(&self, root: &PathBuf) -> Detection {
        if !root.exists() {
            return self.fallback(root);
        }

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

        let mut best = detections.into_iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| self.fallback(root));

        let env = EnvDetector::scan(root);
        best.env_required = env.required;
        best.env_optional = env.optional;
        best.env_secrets = env.secrets;

        let services = crate::multiservice::MultiServiceDetector::detect(root);
        if !services.is_empty() {
            best.is_monorepo = true;
            best.services = services;
        }

        if root.join("Dockerfile").exists() {
            best.dockerfile_found = Some("Dockerfile".to_string());
        }

        best.project_name = root.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        best.base_image = Self::resolve_base_image(&best);
        best
    }

    fn resolve_base_image(d: &Detection) -> String {
        let ver = d.runtime_version.trim();
        // Extract major version only (e.g. "20.11.0" → "20", "3.11.2" → "3.11")
        let major = ver.split('.').next().unwrap_or(ver);
        let major_minor = {
            let parts: Vec<&str> = ver.splitn(3, '.').collect();
            if parts.len() >= 2 { format!("{}.{}", parts[0], parts[1]) } else { major.to_string() }
        };

        match d.runtime_type {
            RuntimeType::Node | RuntimeType::TypeScript => {
                format!("node:{}-alpine", major)
            }
            RuntimeType::Bun => {
                format!("oven/bun:{}", major)
            }
            RuntimeType::Deno => {
                format!("denoland/deno:{}", ver)
            }
            RuntimeType::Python => {
                format!("python:{}-slim", major_minor)
            }
            RuntimeType::Rust => {
                "rust:alpine".to_string()  // Rust builds produce a static binary; use scratch or alpine
            }
            RuntimeType::Go => {
                format!("golang:{}-alpine", major_minor)
            }
            RuntimeType::Java => {
                format!("eclipse-temurin:{}-jre-alpine", major)
            }
            RuntimeType::DotNet => {
                format!("mcr.microsoft.com/dotnet/aspnet:{}", major_minor)
            }
            RuntimeType::Ruby => {
                format!("ruby:{}-alpine", major_minor)
            }
            RuntimeType::Php => {
                format!("php:{}-fpm-alpine", major_minor)
            }
            RuntimeType::Elixir => {
                format!("elixir:{}-alpine", major_minor)
            }
            RuntimeType::Swift => {
                format!("swift:{}-slim", major_minor)
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
        let manifest_ver = json["version"].as_str();
        let version = VersionResolver::resolve(root, manifest_ver);

        let (framework, confidence_bump) = self.detect_node_framework(&json, root);
        let framework_detected = !framework.is_empty();
        let mut confidence = if has_ts { 0.97 } else { 0.93 };
        confidence += confidence_bump;

        let scripts = json["scripts"].as_object();
        let build_cmd = self.infer_node_build(&json, root, has_ts, has_deno);
        let entry = self.infer_node_entry(&json, scripts, root, has_ts);
        let port = Self::detect_port_framework(&framework, 3000);

        Some(Detection {
            project_name: pkg_name.to_string(),
            runtime_type: rt,
            runtime_version: version,
            framework_name: framework,
            framework_detected,
            build_command: build_cmd,
            entry_point: entry,
            port,
            confidence: confidence.min(1.0),
            env_required: vec![],
            env_optional: vec![],
            env_secrets: vec![],
            is_monorepo: false,
            services: vec![],
            dockerfile_found: None,
            base_image: String::new(),
        })
    }

    fn detect_node_framework(&self, json: &serde_json::Value, root: &Path) -> (String, f32) {
        let deps = Self::merge_deps(json);
        let has_file = |name: &str| root.join(name).exists();

        if has_file("svelte.config.js") || has_file("svelte.config.ts")
            || deps.iter().any(|d| d == "@sveltejs/kit") {
            ("SvelteKit".to_string(), 0.05)
        } else if has_file("astro.config.mjs") || has_file("astro.config.ts") || has_file("astro.config.js")
            || deps.iter().any(|d| d == "astro") {
            ("Astro".to_string(), 0.05)
        } else if deps.iter().any(|d| d == "solid-js") && deps.iter().any(|d| d == "@solidjs/start") {
            ("SolidStart".to_string(), 0.05)
        } else if has_file("qwik.config.ts") || deps.iter().any(|d| d == "@builder.io/qwik") {
            ("Qwik".to_string(), 0.04)
        } else if deps.iter().any(|d| d == "next") || has_file("next.config.js") || has_file("next.config.ts") {
            ("Next.js".to_string(), 0.05)
        } else if deps.iter().any(|d| d == "nuxt") || has_file("nuxt.config.ts") || has_file("nuxt.config.js") {
            ("Nuxt".to_string(), 0.05)
        } else if deps.iter().any(|d| d == "@nestjs/core") || has_file("nest-cli.json") {
            ("NestJS".to_string(), 0.04)
        } else if deps.iter().any(|d| d == "express") {
            ("Express".to_string(), 0.02)
        } else if deps.iter().any(|d| d == "fastify") {
            ("Fastify".to_string(), 0.02)
        } else if deps.iter().any(|d| d == "hono") {
            ("Hono".to_string(), 0.02)
        } else if deps.iter().any(|d| d == "remix") || has_file("remix.config.js") {
            ("Remix".to_string(), 0.04)
        } else {
            (String::new(), 0.0)
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
        if let Some(main) = json["main"].as_str() {
            return main.to_string();
        }
        if let Some(bin) = json["bin"].as_str() {
            return bin.to_string();
        }
        if let Some(scripts) = scripts {
            if let Some(start) = scripts.get("start") {
                let start_str = start.as_str().unwrap_or("");
                if let Some(cmd) = start_str.strip_prefix("node ") {
                    return cmd.trim().to_string();
                }
                if let Some(cmd) = start_str.strip_prefix("ts-node ") {
                    return cmd.trim().to_string();
                }
                if let Some(cmd) = start_str.strip_prefix("bun ") {
                    return cmd.trim().to_string();
                }
                if let Some(cmd) = start_str.strip_prefix("deno ") {
                    return cmd.trim().to_string();
                }
            }
        }
        if has_ts {
            if root.join("src/index.ts").exists() { "src/index.ts".to_string() }
            else { "dist/index.js".to_string() }
        } else {
            if root.join("index.js").exists() { "index.js".to_string() }
            else { "src/index.js".to_string() }
        }
    }

    fn try_python(&self, root: &Path) -> Option<Detection> {
        let has_pyproject = root.join("pyproject.toml").exists();
        let has_requirements = root.join("requirements.txt").exists();
        let has_setup = root.join("setup.py").exists();
        let has_poetry = root.join("poetry.lock").exists();
        let has_uv = root.join("uv.lock").exists();
        let has_pdm = root.join("pdm.lock").exists();
        let has_conda = root.join("environment.yml").exists() || root.join("environment.yaml").exists();

        if !has_pyproject && !has_requirements && !has_setup && !has_conda { return None; }

        let version = if let Some(v) = VersionResolver::from_python_version(root) {
            v
        } else if has_pyproject {
            if let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) {
                if let Ok(json) = toml::from_str::<serde_json::Value>(&content) {
                    json["project"]["requires-python"].as_str().unwrap_or("3.11").trim_start_matches(">= ").to_string()
                } else { "3.11".to_string() }
            } else { "3.11".to_string() }
        } else { "3.11".to_string() };

        let (framework, entry, port) = if root.join("manage.py").exists() {
            ("Django", "manage.py", 8000)
        } else if root.join("app.py").exists() || root.join("application.py").exists() {
            let app = if root.join("app.py").exists() { "app.py" } else { "application.py" };
            if Self::file_contains(root.join(app), "flask").unwrap_or(false) {
                ("Flask", app, 5000)
            } else {
                ("FastAPI", app, 8000)
            }
        } else if root.join("main.py").exists() {
            ("FastAPI", "main.py", 8000)
        } else if root.join("api.py").exists() {
            ("FastAPI", "api.py", 8000)
        } else {
            ("Python", "main.py", 8080)
        };

        let build_cmd = if has_uv {
            "uv sync --no-dev".to_string()
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
        } else {
            "pip install -r requirements.txt".to_string()
        };

        let confidence = if root.join("manage.py").exists() || root.join("app.py").exists() { 0.92 } else { 0.85 };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Python,
            runtime_version: version,
            framework_name: framework.to_string(),
            framework_detected: framework != "Python",
            build_command: build_cmd,
            entry_point: entry.to_string(),
            port,
            confidence,
            ..Default::default()
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

        let framework = Self::detect_rust_framework(&content, root);
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
            entry_point: format!("target/release/{}", bin_target),
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

        Some(Detection {
            project_name: bin_name.to_string(),
            runtime_type: RuntimeType::Go,
            runtime_version: go_ver,
            framework_name: framework.to_string(),
            framework_detected: !framework.is_empty(),
            build_command: "go build -o app .".to_string(),
            entry_point: format!("./{}", bin_name),
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

        let build = if has_maven { "mvn package -DskipTests".to_string() } else { "gradle bootJar".to_string() };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Java,
            runtime_version: version,
            framework_name: framework.to_string(),
            framework_detected: true,
            build_command: build,
            entry_point: "target/*.jar".to_string(),
            port,
            confidence: 0.9,
            ..Default::default()
        })
    }

    fn try_dotnet(&self, root: &Path) -> Option<Detection> {
        let csproj = Self::find_file(root, ".csproj");
        let has_global = root.join("global.json").exists();

        if csproj.is_none() && !has_global { return None; }

        let version = VersionResolver::resolve(root, None);
        let project_name = csproj.as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "app".to_string());

        Some(Detection {
            project_name,
            runtime_type: RuntimeType::DotNet,
            runtime_version: version,
            framework_name: if has_global { "ASP.NET Core" } else { "" }.to_string(),
            framework_detected: has_global,
            build_command: "dotnet publish -c Release -o out".to_string(),
            entry_point: "out/app".to_string(),
            port: 5000,
            confidence: 0.88,
            ..Default::default()
        })
    }

    fn try_ruby(&self, root: &Path) -> Option<Detection> {
        let has_gemfile = root.join("Gemfile").exists();
        let has_ruby_version = root.join(".ruby-version").exists();
        if !has_gemfile && !has_ruby_version { return None; }

        let version = VersionResolver::resolve(root, None);
        let is_rails = root.join("config/application.rb").exists()
            || root.join("bin/rails").exists();

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Ruby,
            runtime_version: version,
            framework_name: if is_rails { "Rails" } else { "" }.to_string(),
            framework_detected: is_rails,
            build_command: "bundle install".to_string(),
            entry_point: if is_rails { "config.ru".to_string() } else { "app.rb".to_string() },
            port: if is_rails { 3000 } else { 8080 },
            confidence: 0.87,
            ..Default::default()
        })
    }

    fn try_php(&self, root: &Path) -> Option<Detection> {
        if !root.join("composer.json").exists() { return None; }

        let is_laravel = root.join("artisan").exists()
            || Self::file_contains(root.join("composer.json"), "laravel/framework").unwrap_or(false);
        let is_symfony = Self::file_contains(root.join("composer.json"), "symfony/framework-bundle").unwrap_or(false);

        let (framework, entry, port, confidence) = if is_laravel {
            ("Laravel", "public/index.php", 8000, 0.92f32)
        } else if is_symfony {
            ("Symfony", "public/index.php", 8000, 0.90)
        } else {
            ("", "public/index.php", 8080, 0.85)
        };

        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Php,
            runtime_version: VersionResolver::resolve(root, None),
            framework_name: framework.to_string(),
            framework_detected: !framework.is_empty(),
            build_command: "composer install --no-dev --optimize-autoloader".to_string(),
            entry_point: entry.to_string(),
            port,
            confidence,
            ..Default::default()
        })
    }

    fn try_elixir(&self, root: &Path) -> Option<Detection> {
        if !root.join("mix.exs").exists() { return None; }
        let has_phoenix = root.join("lib").is_dir()
            && Self::find_file(root, "_web.ex").is_some();
        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Elixir,
            runtime_version: VersionResolver::resolve(root, None),
            framework_name: if has_phoenix { "Phoenix" } else { "" }.to_string(),
            framework_detected: has_phoenix,
            build_command: "mix release".to_string(),
            entry_point: "_build/prod/rel/app/bin/app".to_string(),
            port: if has_phoenix { 4000 } else { 8080 },
            confidence: 0.85,
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
            port: if has_vapor { 8080 } else { 8080 },
            confidence: 0.85,
            ..Default::default()
        })
    }

    fn fallback(&self, root: &Path) -> Detection {
        Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Generic,
            runtime_version: "latest".to_string(),
            framework_name: String::new(),
            framework_detected: false,
            build_command: "echo 'No build required'".to_string(),
            entry_point: "entrypoint.sh".to_string(),
            port: 80,
            confidence: 0.5,
            ..Default::default()
        }
    }

    fn detect_rust_framework(content: &str, root: &Path) -> String {
        if content.contains("actix-web") { return "Actix-web".to_string(); }
        if content.contains("axum") { return "Axum".to_string(); }
        if content.contains("rocket") { return "Rocket".to_string(); }
        if content.contains("warp") { return "Warp".to_string(); }
        if content.contains("tide") { return "Tide".to_string(); }
        if root.join("src/main.rs").exists() {
            if let Ok(src) = fs::read_to_string(root.join("src/main.rs")) {
                if src.contains("actix_web") { return "Actix-web".to_string(); }
                if src.contains("axum") { return "Axum".to_string(); }
                if src.contains("rocket") { return "Rocket".to_string(); }
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
            port: 80,
            confidence: 0.0,
            env_required: vec![],
            env_optional: vec![],
            env_secrets: vec![],
            is_monorepo: false,
            services: vec![],
            dockerfile_found: None,
            base_image: "ubuntu:22.04".to_string(),
        }
    }
}
