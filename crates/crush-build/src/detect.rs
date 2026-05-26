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

        best
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
            framework_detected: !framework.is_empty(),
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
        })
    }

    fn detect_node_framework(&self, json: &serde_json::Value, root: &Path) -> (String, f32) {
        let deps = Self::merge_deps(json);
        let has_file = |name: &str| root.join(name).exists();

        if deps.iter().any(|d| d == "next") || has_file("next.config.js") || has_file("next.config.ts") {
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
        if has_ts && root.join("tsconfig.json").exists() {
            return "npm run build".to_string();
        }
        if let Some(scripts) = json["scripts"].as_object() {
            if scripts.contains_key("build") {
                return "npm run build".to_string();
            }
            if scripts.contains_key("start") {
                return "npm start".to_string();
            }
        }
        if root.join("vite.config.ts").exists() || root.join("vite.config.js").exists() {
            return "npm run build".to_string();
        }
        "npm install".to_string()
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

        if !has_pyproject && !has_requirements && !has_setup { return None; }

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

        let build_cmd = if has_poetry {
            "poetry install".to_string()
        } else if has_requirements {
            "pip install -r requirements.txt".to_string()
        } else if has_pyproject {
            "pip install -e .".to_string()
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

        let framework = if root.join("gin").exists() || Self::file_contains(root.join("go.mod"), "gin").unwrap_or(false) {
            "Gin"
        } else if Self::file_contains(root.join("go.mod"), "echo").unwrap_or(false) {
            "Echo"
        } else if Self::file_contains(root.join("go.mod"), "fiber").unwrap_or(false) {
            "Fiber"
        } else if Self::file_contains(root.join("go.mod"), "chi").unwrap_or(false) {
            "Chi"
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
        Some(Detection {
            project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
            runtime_type: RuntimeType::Php,
            runtime_version: VersionResolver::resolve(root, None),
            framework_name: "".to_string(),
            framework_detected: false,
            build_command: "composer install --no-dev".to_string(),
            entry_point: "public/index.php".to_string(),
            port: 8080,
            confidence: 0.85,
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
        }
    }
}
