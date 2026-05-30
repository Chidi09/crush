# Zero-Config Stack Detection

Crush includes a built-in **Auto-detection Engine** that determines a project's stack, dependencies, framework, and entry point without requiring any Dockerfiles or manual manifests.

---

## Fingerprint Detection Heuristics

When you run `crush` in a directory, the runtime scans the directory tree (respecting `.gitignore` filters) for signature files that uniquely identify a programming language or framework:

| Stack / Language | Signature / Lock File | Framework Inferences | Default Port |
| :--- | :--- | :--- | :--- |
| **Node.js** | `package.json`, `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock` | Express, Next.js, SvelteKit, NestJS | `3000` / `5173` |
| **Python** | `requirements.txt`, `pyproject.toml`, `Pipfile`, `poetry.lock` | FastAPI, Django, Flask | `8000` |
| **Go** | `go.mod`, `go.sum` | Gin, Fiber, Echo | `8080` |
| **Rust** | `Cargo.toml`, `Cargo.lock` | Actix-web, Axum, Rocket | `8080` |
| **Ruby** | `Gemfile`, `Gemfile.lock` | Ruby on Rails, Sinatra | `3000` |
| **PHP** | `composer.json` | Laravel, Symfony | `8000` |
| **Java / JVM** | `pom.xml`, `build.gradle` | Spring Boot, Quarkus | `8080` |

---

## Compilation & Bundling Mechanics

Once the stack is inferred:
1. **Dependency Separation**: Crush parses the manifest file (e.g., `package.json` or `Cargo.toml`) to extract dependency declarations.
2. **Deterministic Layering**: It builds a dedicated *Dependency Layer* containing only external dependencies. This layer is aggressively cached. If the manifest or lockfile is unchanged, the layer is fully reused.
3. **Application Compilation**: The application code is packed as a separate, ultra-lightweight overlay layer. This decoupling enables sub-second build times for iterative code changes.
