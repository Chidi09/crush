<div align="center">
  <img src="crush-web/public/logo.png" alt="Crush" width="120" />

  <h1>Crush</h1>
  <p><strong>Run your project on Windows the way you'd want to: deps already up, no Docker daemon, no WSL2.</strong></p>
  <p>Native Postgres &nbsp;·&nbsp; Auto-detected stack &nbsp;·&nbsp; Job Object cleanup &nbsp;·&nbsp; Eject to Dockerfile for prod</p>

  <p>
    <a href="https://github.com/Chidi09/crush/releases/latest"><img src="https://img.shields.io/github/v/release/Chidi09/crush?style=flat-square" alt="Latest release" /></a>
    <a href="https://github.com/Chidi09/crush/blob/main/Cargo.toml"><img src="https://img.shields.io/badge/rustc-stable-orange?logo=rust&style=flat-square" alt="Rust Version" /></a>
    <a href="https://github.com/Chidi09/crush/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=flat-square" alt="License" /></a>
    <a href="https://github.com/Chidi09/crush/stargazers"><img src="https://img.shields.io/github/stars/Chidi09/crush?logo=github&style=flat-square" alt="Stars" /></a>
    <a href="#status"><img src="https://img.shields.io/badge/status-alpha-orange?style=flat-square" alt="Status" /></a>
  </p>
</div>

---

> **Status:** Alpha. The 0.7.x line is daily-driven on Windows by the author across 12 real projects. Detection covers Node / Next / Nuxt / Vite / AnalogJS / Angular / SvelteKit / Astro / Spring Boot / Quarkus / Django / Flask / FastAPI / Rust / Go. Schemas and CLI flags are settling but may still change.

---

## What Crush actually does

You `cd` into a project. You type `crush`. Crush:

1. **Detects the stack** — language, framework, port, dev vs prod entry, whether it's a monorepo
2. **Auto-starts the deps it needs** — native PostgreSQL, Redis-compat (Garnet on Windows), MySQL — by parsing `docker-compose.yml`, `application.yml`, or `.env`
3. **Picks the right entry** — `mvn spring-boot:run` if no Dockerfile / `mvn package + java -jar` if there is; `pnpm dev` for standalone frontends / `pnpm start` for monorepos with a compose file
4. **Skips work that's already done** — `pnpm install` is skipped when `node_modules` is newer than the lockfile; image pack is skipped when source fingerprints match
5. **Surfaces useful URLs** — probes `/swagger-ui/index.html`, `/v3/api-docs`, `/actuator/health`, etc. and prints them
6. **Kills the whole process tree on Ctrl+C** via a Windows Job Object — no orphan `node` processes holding port 3000 between runs

It's not a container runtime in the Docker sense. **It runs your app natively, end-to-end.** The Job Object is for clean teardown, not isolation. For production you `crush eject` to get a real Dockerfile + docker-compose.yml and deploy that.

```
$ crush
   ↳ starting db (postgres:17-alpine) [from application.yml]... ok  [native]
   ↳ detected: Java · Spring Boot (+ 1 service)
   ✓ image fresh — skipping pack
   ↳ dependencies layer cached (unchanged)
 ✓ crushed to image solexpay_backend:latest (0 MB)
   ↳ warm run — launching
   ✓ build fresh — target/...jar newer than sources

  .   ____          _            __ _ _
 /\\ / ___'_ __ _ _(_)_ __  __ _ \ \ \ \
( ( )\___ | '_ | '_| | '_ \/ _` | \ \ \ \
 \\/  ___)| |_)| | | | | || (_| |  ) ) ) )
  '  |____| .__|_| |_|_| |_\__, | / / / /
 =========|_|==============|___/=/_/_/_/

 :: Spring Boot ::                (v3.4.4)

 ✓ running natively on :8080 — started in 30.9s (total: 42.8s!)
   ↳ open:
     [docs]   http://localhost:8080/swagger-ui/index.html
     [health] http://localhost:8080/actuator/health
```

---

## What Crush is **not**

Worth saying clearly so nobody is misled:

- **Not a Docker Desktop replacement.** It doesn't run Linux containers on Windows. There is no isolation, no namespacing — your app runs as a normal native process.
- **Not a Linux container runtime.** A Linux backend exists in `crush-runtime-linux/` but it's experimental scaffolding, not battle-tested.
- **Not yet a macOS tool.** A `crush-runtime-macos/` crate exists but isn't shipped.
- **Cold starts aren't magic.** A cold Vite/Angular dev server still takes 30–90s because esbuild has to bundle your deps. Crush makes the *overhead around that* near-zero — warm runs are ~2s including detection. We don't shortcut your framework.

What we *are* honest about: cold starts are bound by whatever your framework does. We cut the 10–14s "pnpm install Already up to date" tax and the 60s "mvn package" tax when those steps aren't actually needed.

---

## Honest performance numbers

From real projects, warm runs (deps installed, image cached):

| Project | Stack | Warm `crush` → "running" |
|---|---|---|
| Clifton-ville-website | Vite | ~4s (then 7s vite optimize) |
| MarketCircleFrontend | Nuxt | ~3s (then ~20s nuxt dev compile) |
| Solexpay-backend | Spring Boot | ~3s (then ~30s Spring context load) |
| gazillion-be-staging | FastAPI + pgvector | ~4s (postgres + uvicorn + extension setup) |
| safe-meet | Turbo monorepo | ~3s |

Cold runs (first time, fresh `node_modules`):
- Vite: 30–90s, dominated by esbuild bundling
- Spring Boot: 60s `mvn compile` + 30s context = ~90s
- Next.js: 20–60s depending on dep tree

Crush isn't faster than running these tools yourself. It's faster than running them, plus `docker compose up -d postgres`, plus checking the right `.env`, plus typing `pnpm install` first.

---

## What works today (v0.7.72)

Detection and run:
- Node.js / TypeScript: Next / Nuxt / Vite / SvelteKit / AnalogJS / Angular / Astro / Express / Fastify / NestJS / Hono / Qwik
- Java: Spring Boot / Quarkus / Micronaut (Maven and Gradle)
- Python: FastAPI / Django / Flask / Litestar / Starlette
- Go, Rust, Ruby (Rails), PHP (Laravel), Elixir (Phoenix), .NET
- Monorepos: Turborepo / Nx / pnpm-workspace / Lerna

Auto-managed deps:
- PostgreSQL (system install detected) with idempotent user/password sync
- pgvector (Windows: compiles against host PG via MSVC on first use)
- Redis-compat: Garnet (Windows) or Redis (Unix)
- MySQL / MariaDB

Quality-of-life:
- `.crushignore` to extend the fingerprint skip list for big repos
- `--memory`, `--cpus`, `--priority` resource caps via Job Object
- `--watch` for hot-restart on source change
- Job Object guarantees the entire spawned tree dies on `crush` exit
- `crush eject` writes a marked Dockerfile + compose for prod (marker so it doesn't poison subsequent dev runs)
- `crush update` self-updates from GitHub releases
- `crush detect --json` / `crush ps --format json` / `crush services ps --format json` / `crush history --format json` for tooling

AI (requires `ANTHROPIC_API_KEY`):
- `crush debug <container>` — parse a stack trace, suggest a fix

---

## What's stubbed or experimental

Listed honestly so you don't hit a wall:

- Linux runtime (`crush-runtime-linux`): scaffolded, not daily-driven
- macOS runtime (`crush-runtime-macos`): scaffolded, not built in CI
- WASM runtime (`crush-runtime-wasm`): scaffolded
- `crush scan` / `crush sbom`: present but limited coverage
- `crush registry` local server: present but underused
- Multi-arch builds (`--platform linux/amd64,linux/arm64`): cross-build works in CI for releases; native cross-compile from Windows is not supported
- `crush compose`: parses compose but doesn't manage full lifecycle
- `crush watch` (subcommand vs `--watch` flag): on Windows the subcommand exits with a hint pointing at `--watch` (the subcommand uses Linux overlayfs)

The 0.8 GUI is in planning — see `CRUSH_V8_PLAN.md`.

---

## Install

### Windows (the supported platform)

Download the latest release: <https://github.com/Chidi09/crush/releases/latest>

Place `crush-x.y.z-windows-x86_64.exe` somewhere on your PATH (rename to `crush.exe`).

Or, once installed, future updates:
```powershell
crush update
```

### From source

```bash
git clone https://github.com/Chidi09/crush.git
cd crush
cargo build --release -p crush-cli
# Binary at target/release/crush-cli.exe (rename to crush.exe)
```

Cross-compiling for Windows from Linux is what we use in CI:
```bash
# Linux host:
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64
cargo build --release --target x86_64-pc-windows-gnu -p crush-cli
```

Homebrew, Scoop, and `cargo install crush-cli` are **not** published yet — the README used to claim they were. They'll arrive once macOS / Linux are first-class.

### Optional: pgvector on Windows

If you have projects that use `pgvector/pgvector:*` images, install Visual Studio Build Tools so crush can compile the extension against your PostgreSQL:

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows10SDK.19041"
```

First `crush` run that needs pgvector clones the source, builds, and installs. Subsequent runs skip the build. The install step writes to `C:\Program Files\PostgreSQL\<v>\` — run that first time from an elevated terminal.

---

## CLI reference (commands that actually work today)

| Command | What it does |
|---|---|
| `crush` | Detect → start deps → build → run |
| `crush detect` | Print detection (text) or `--json` |
| `crush eject` | Write `Dockerfile` + `docker-compose.yml` from detection |
| `crush update` | Self-update from GitHub releases |
| `crush ps` / `ps --format json` | List containers (mostly empty on Windows-native) |
| `crush services ps` / `--format json` / `--all-projects` | List native deps (postgres, garnet, …) |
| `crush history` / `--format json` | Recent build outcomes |
| `crush images` / `--format json` | List image manifests |
| `crush pull <ref>` | Pull from an OCI registry |
| `crush stop <id>` / `crush logs <id>` | Container lifecycle |
| `crush stats` | TUI dashboard (live CPU/mem) |
| `crush debug <id>` | AI crash diagnosis (needs `ANTHROPIC_API_KEY`) |

Flags worth knowing:
- `--rebuild` — bust the warm-run cache and re-run install/build
- `--repack` — force image re-pack
- `--watch` — hot-restart on source change
- `--memory 1G --cpus 0.5` — Job Object resource caps
- `--priority high` — boost the spawned tree's scheduling priority (Windows)
- `--no-proxy` — skip the reverse proxy (monorepos with backend+frontend)

`crush <cmd> --help` for the full flag set.

---

## Architecture

Crush is a Cargo workspace. Crates marked **active** are daily-driven; others are scaffolded for the 0.8/0.9 roadmap.

```
crush/
├── crates/
│   ├── crush-cli/             # CLI entry, detection-to-spawn flow              [active]
│   ├── crush-build/           # Stack detection, build engine, run-event types  [active]
│   ├── crush-services/        # Native postgres/redis/mysql, pgvector extension [active]
│   ├── crush-image/           # OCI image store + sled-backed metadata          [active]
│   ├── crush-registry/        # OCI pull (push partial)                         [partial]
│   ├── crush-reliability/     # Restart policies, health checks                 [partial]
│   ├── crush-runtime-windows/ # Job Objects, ConPTY                             [active for cleanup; no isolation]
│   ├── crush-runtime-linux/   # Scaffolded                                      [experimental]
│   ├── crush-runtime-macos/   # Scaffolded                                      [experimental]
│   ├── crush-runtime-wasm/    # Scaffolded                                      [experimental]
│   ├── crush-compat/          # Dockerfile + compose parser                     [partial]
│   ├── crush-ai/              # Anthropic-backed crash diagnosis                [active]
│   ├── crush-tui/             # ratatui-based ps/stats dashboard                [active]
│   ├── crush-network/         # Reverse proxy, port mapping                     [partial]
│   ├── crush-volume/          # Bind mounts, named volumes                      [partial]
│   ├── crush-types/           # Shared structs (Container, Image, …)            [active]
│   └── crush-api/             # HTTP API surface                                [partial]
├── crush-web/                 # Marketing site (AnalogJS)
├── benches/                   # Criterion benchmarks
├── docs/                      # mdBook source
└── extensions/                # VS Code / JetBrains / Neovim integrations
```

### Key design decisions

- **Single static binary** — `crush.exe` runs with no runtime deps. Release profile uses `lto = true`, `codegen-units = 1`, `strip = true`.
- **Async-first** — Tokio throughout.
- **Platform-gated runtimes** — each runtime crate is `#[cfg]`-gated.
- **Content-addressable storage** — image store uses SHA-256 digests via `sha2` + `sled`.

---

## Contributing

Crush is early — pick anything from the [issues](https://github.com/Chidi09/crush/issues) or:

- Add detection for a stack we don't handle yet (drop a `try_<stack>(root)` in `crates/crush-build/src/detect.rs`)
- Improve the run-event refactor (`crates/crush-build/src/run.rs` — the function body lives in `crush-cli/src/main.rs` Commands::Default and needs extracting; see `CRUSH_V8_PLAN.md`)
- Fix a platform-specific spawn quirk
- Move a `partial` crate to `active` by completing its coverage

How to contribute:
1. Fork, branch from `main`
2. `cargo build` must succeed (cross-compile to `x86_64-pc-windows-gnu` for Windows)
3. Run `cargo clippy` — must pass
4. PR with a clear description

If you have a larger feature in mind, open an issue first to discuss.

---

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

<div align="center">
  <sub><a href="https://github.com/Chidi09/crush">github.com/Chidi09/crush</a> · Built with Rust by <a href="https://github.com/Chidi09">@Chidi09</a></sub>
</div>
