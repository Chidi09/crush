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
        Some(Self::build_sub_service(path, name, rt, port))
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
                                                    services.push(Self::build_sub_service(&p, &n, "node", 3000));
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

    fn build_sub_service(path: &Path, name: &str, rt: &str, port: u16) -> SubService {
        let (entry, dev_entry, build_cmd, dev_install) = match rt {
            "node" => {
                let pkg_content = fs::read_to_string(path.join("package.json")).unwrap_or_default();
                let has_ts = path.join("tsconfig.json").exists();
                let has_deno = path.join("deno.json").exists() || path.join("deno.jsonc").exists();
                let has_bun = path.join("bun.lockb").exists();
                
                let pm = if has_bun { "bun" }
                    else if path.join("pnpm-lock.yaml").exists() { "pnpm" }
                    else if path.join("yarn.lock").exists() { "yarn" }
                    else { "npm" };
                
                let dev_install = if has_deno { "".to_string() } else { format!("{} install", pm) };
                let dev_entry = if has_deno { "deno task dev".to_string() } else { format!("{} run dev", pm) };
                
                // Let's determine framework
                let framework = if pkg_content.contains("\"next\"") || path.join("next.config.js").exists() || path.join("next.config.ts").exists() { "Next.js" }
                    else if pkg_content.contains("\"nuxt\"") || path.join("nuxt.config.ts").exists() { "Nuxt" }
                    else if pkg_content.contains("\"@sveltejs/kit\"") || path.join("svelte.config.js").exists() { "SvelteKit" }
                    else if pkg_content.contains("\"remix\"") || path.join("remix.config.js").exists() { "Remix" }
                    else if pkg_content.contains("\"astro\"") || path.join("astro.config.mjs").exists() { "Astro" }
                    else if pkg_content.contains("\"vite\"") || path.join("vite.config.ts").exists() || path.join("vite.config.js").exists() { "Vite" }
                    else { "other" };
                
                let entry_prod = match framework {
                    "Vite" => format!("{} exec vite preview -- --port $PORT --host 0.0.0.0", pm),
                    "Next.js" => format!("{} run start", pm),
                    "Nuxt" => format!("{} run preview", pm),
                    "SvelteKit" => "node build/index.js".to_string(),
                    "Remix" => format!("{} run start", pm),
                    "Astro" => format!("{} run preview", pm),
                    _ => {
                        let pkg_json: serde_json::Value = serde_json::from_str(&pkg_content).unwrap_or(serde_json::Value::Null);
                        let scripts = pkg_json["scripts"].as_object();
                        let mut resolved_entry = format!("{} start", pm);
                        if let Some(scripts) = scripts {
                            if let Some(start) = scripts.get("start") {
                                let start_str = start.as_str().unwrap_or("");
                                for prefix in ["node ", "ts-node ", "bun ", "deno ", "tsx "] {
                                    if let Some(cmd) = start_str.strip_prefix(prefix) {
                                        resolved_entry = cmd.trim().to_string();
                                        break;
                                    }
                                }
                            }
                        }
                        resolved_entry
                    }
                };
                
                let build_script_exists = pkg_content.contains("\"build\"");
                let build_cmd = if has_deno {
                    "deno task build".to_string()
                } else if build_script_exists {
                    format!("{} install && {} run build", pm, pm)
                } else {
                    format!("{} install", pm)
                };
                
                (entry_prod, dev_entry, build_cmd, dev_install)
            }
            "go" => {
                let main_path = if path.join("cmd").is_dir() { "cmd/main.go" } else if path.join("main.go").exists() { "main.go" } else { "." };
                let bin = if std::env::consts::OS == "windows" { format!("{}.exe", name) } else { name.to_string() };
                let build_cmd = if main_path == "." {
                    format!("go build -o {} .", bin)
                } else {
                    format!("go build -o {} {}", bin, main_path)
                };
                let run_cmd = if main_path == "." { "go run .".to_string() } else { format!("go run {}", main_path) };
                (format!("./{}", bin), run_cmd, build_cmd, "".to_string())
            }
            "rust" => (
                format!("target/release/{}", name),
                "cargo run".to_string(),
                "cargo build --release".to_string(),
                "".to_string()
            ),
            "python" => {
                let has_uv = path.join("uv.lock").exists();
                let has_pdm = path.join("pdm.lock").exists();
                let has_poetry = path.join("poetry.lock").exists();
                let has_requirements = path.join("requirements.txt").exists();
                let has_pyproject = path.join("pyproject.toml").exists();
                let has_conda = path.join("environment.yml").exists() || path.join("environment.yaml").exists();
                
                let build_cmd = if has_uv {
                    "uv sync --no-dev --no-install-project".to_string()
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
                
                let dev_install = build_cmd.clone();
                let venv_bin = if cfg!(target_os = "windows") { r".venv\Scripts\" } else { ".venv/bin/" };
                let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
                let py = if has_uv { format!("{}{}{}", venv_bin, "python", exe_suffix) } else { "python".to_string() };
                
                let py_deps = if has_requirements {
                    fs::read_to_string(path.join("requirements.txt")).unwrap_or_default()
                } else { "".to_string() };
                
                let framework = if py_deps.contains("fastapi") || path.join("main.py").exists() { "FastAPI" }
                    else if py_deps.contains("django") || path.join("manage.py").exists() { "Django" }
                    else if py_deps.contains("flask") || path.join("app.py").exists() { "Flask" }
                    else { "Python" };
                
                let entry_file = if framework == "Django" { "manage.py" }
                    else if framework == "Flask" { "app.py" }
                    else { "main.py" };
                
                let entry_prod = match framework {
                    "FastAPI" => {
                        let workers = if cfg!(target_os = "windows") { "" } else { " --workers 2" };
                        if has_uv {
                            format!("{}uvicorn{} main:app --host 0.0.0.0 --port $PORT{}", venv_bin, exe_suffix, workers)
                        } else {
                            format!("uvicorn main:app --host 0.0.0.0 --port $PORT{}", workers)
                        }
                    }
                    "Flask" => {
                        if cfg!(target_os = "windows") {
                            if has_uv { format!("{}flask{} --app app run --host=0.0.0.0 --port=$PORT", venv_bin, exe_suffix) }
                            else { "flask --app app run --host=0.0.0.0 --port=$PORT".to_string() }
                        } else {
                            "gunicorn app:app -b 0.0.0.0:$PORT".to_string()
                        }
                    }
                    "Django" => {
                        if cfg!(target_os = "windows") {
                            format!("{} manage.py runserver 0.0.0.0:$PORT", py)
                        } else {
                            format!("gunicorn {}.wsgi -b 0.0.0.0:$PORT", name)
                        }
                    }
                    _ => format!("{} {}", py, entry_file),
                };
                
                let dev_entry = match framework {
                    "FastAPI" => {
                        if has_uv { format!("{}uvicorn{} main:app --host 0.0.0.0 --port $PORT --reload", venv_bin, exe_suffix) }
                        else { "uvicorn main:app --host 0.0.0.0 --port $PORT --reload".to_string() }
                    }
                    "Flask" => {
                        if has_uv { format!("{}flask{} --app app run --host=0.0.0.0 --port=$PORT", venv_bin, exe_suffix) }
                        else { "flask --app app run --host=0.0.0.0 --port=$PORT".to_string() }
                    }
                    "Django" => format!("{} manage.py runserver 0.0.0.0:$PORT", py),
                    _ => format!("{} {}", py, entry_file),
                };
                
                (entry_prod, dev_entry, build_cmd, dev_install)
            }
            "java" => {
                let has_maven = path.join("pom.xml").exists();
                let (build_cmd, entry_prod, dev_entry) = if has_maven {
                    (
                        "mvn -B package -Dmaven.test.skip=true".to_string(),
                        "java -jar target/*.jar".to_string(),
                        "mvn spring-boot:run -Dmaven.test.skip=true".to_string()
                    )
                } else {
                    (
                        "gradle bootJar -x test".to_string(),
                        "java -jar build/libs/*.jar".to_string(),
                        "gradle bootRun -x test".to_string()
                    )
                };
                (entry_prod, dev_entry, build_cmd, "".to_string())
            }
            "ruby" => {
                let has_gemfile = path.join("Gemfile").exists();
                let framework = if has_gemfile && fs::read_to_string(path.join("Gemfile")).unwrap_or_default().contains("rails") { "Rails" } else { "other" };
                let (build_cmd, entry_prod, dev_entry) = if framework == "Rails" {
                    (
                        "bundle install && bundle exec rails assets:precompile".to_string(),
                        "bundle exec rails assets:precompile && RAILS_ENV=production bundle exec rails server -b 0.0.0.0 -p $PORT".to_string(),
                        "bundle exec rails server -b 0.0.0.0 -p $PORT".to_string()
                    )
                } else {
                    (
                        "bundle install".to_string(),
                        "bundle exec ruby app.rb".to_string(),
                        "bundle exec ruby app.rb".to_string()
                    )
                };
                (entry_prod, dev_entry, build_cmd, "bundle install".to_string())
            }
            "php" => {
                let has_laravel = path.join("artisan").exists();
                let (build_cmd, entry_prod, dev_entry) = if has_laravel {
                    (
                        "composer install --no-dev --optimize-autoloader && php artisan config:cache".to_string(),
                        "php artisan serve --host=0.0.0.0 --port=$PORT".to_string(),
                        "php artisan serve --host=0.0.0.0 --port=$PORT".to_string()
                    )
                } else {
                    (
                        "composer install --no-dev --optimize-autoloader".to_string(),
                        "php -S 0.0.0.0:$PORT -t public".to_string(),
                        "php -S 0.0.0.0:$PORT -t public".to_string()
                    )
                };
                (entry_prod, dev_entry, build_cmd, "composer install".to_string())
            }
            "elixir" => {
                let has_phoenix = path.join("lib").is_dir();
                let (build_cmd, entry_prod, dev_entry) = if has_phoenix {
                    (
                        "mix deps.get && MIX_ENV=prod mix release".to_string(),
                        format!("_build/prod/rel/{}/bin/{} start", name, name),
                        "mix phx.server".to_string()
                    )
                } else {
                    (
                        "mix deps.get && MIX_ENV=prod mix compile".to_string(),
                        "MIX_ENV=prod mix run --no-halt".to_string(),
                        "mix run --no-halt".to_string()
                    )
                };
                (entry_prod, dev_entry, build_cmd, "mix deps.get".to_string())
            }
            _ => (
                "entrypoint.sh".to_string(),
                "entrypoint.sh".to_string(),
                "echo 'No build required'".to_string(),
                "".to_string()
            )
        };
        
        SubService {
            name: name.to_string(),
            path: path.to_string_lossy().to_string(),
            runtime_type: rt.to_string(),
            port,
            entry_point: entry,
            dev_entry_point: dev_entry,
            build_command: build_cmd,
            dev_install_command: dev_install,
        }
    }
}
