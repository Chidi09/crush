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

> **Status:** Alpha. The 0.8.x line is daily-driven by the author across 12+ real projects. Detection covers Node / Next / Nuxt / Vite / AnalogJS / Angular / SvelteKit / Astro / Spring Boot / Quarkus / Django / Flask / FastAPI / Rust / Go. A desktop GUI (Tauri 2 + Svelte) shipped in 0.8. Schemas and CLI flags are settling but may still change.

---

## What Crush actually does

You `cd` into a project. You type `crush`. Crush:

1. **Detects the stack** — language, framework, port, dev vs prod entry, whether it's a monorepo. Crushfile > compose > Dockerfile > heuristics — explicit files always win.
2. **Auto-starts the deps it needs** — native PostgreSQL, Redis-compat (Garnet on Windows), MySQL — by parsing `docker-compose.yml`, `application.yml`, or `.env`. For monorepos, per-service env vars are merged in priority order.
3. **Picks the right entry** — `mvn spring-boot:run` if no Dockerfile / `mvn package + java -jar` if there is; `pnpm dev` for standalone frontends / `pnpm start` for monorepos with a compose file.
4. **Skips work that's already done** — `pnpm install` is skipped when `node_modules` is newer than the lockfile; image pack is skipped when source fingerprints match.
5. **Scans for secrets and env requirements** — before building, Crush scans every source file for leaked credentials (AWS, GCP, GitHub, OpenAI, GitLab, npm, Vercel, Cloudflare tokens; DB connection strings; high-entropy strings) and warns before execution. It also reads `.env.example` / `.env.sample` / `.env.template` and code references (`process.env`, `os.environ`, `std::env::var`, `System.getenv`, etc.) to infer which vars are required vs optional.
6. **Surfaces useful URLs** — probes `/swagger-ui/index.html`, `/v3/api-docs`, `/actuator/health`, etc. and prints them.
7. **Kills the whole process tree on Ctrl+C** via a Windows Job Object — no orphan `node` processes holding port 3000 between runs.

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

## What works today (v0.8.4)

Detection and run:
- Node.js / TypeScript: Next / Nuxt / Vite / SvelteKit / AnalogJS / Angular / Astro / Express / Fastify / NestJS / Hono / Qwik
- Java: Spring Boot / Quarkus / Micronaut (Maven and Gradle)
- Python: FastAPI / Django / Flask / Litestar / Starlette
- Go, Rust, Ruby (Rails), PHP (Laravel), Elixir (Phoenix), .NET
- Monorepos: Turborepo / Nx / pnpm-workspace / Lerna
- Dockerfile / Containerfile detection in nested `infra/`, `docker/`, `.docker/`, `deploy/`, `ops/` directories. Files annotated `# crush:eject` are skipped so ejected Dockerfiles don't loop back into detection.
- `docker-compose.yml` / `compose.yml` support with full service graph and per-service env merge.

Auto-managed deps:
- PostgreSQL (system install detected) with idempotent user/password sync
- pgvector (Windows: compiles against host PG via MSVC on first use)
- Redis-compat: Garnet (Windows) or Redis (Unix)
- MySQL / MariaDB

Security scanning (runs at build time, before execution):
- Leaked credentials: AWS, GCP, GitHub PATs, OpenAI project keys, GitLab PATs, npm tokens, Vercel tokens, Cloudflare API tokens
- Database connection strings: PostgreSQL, MySQL, MongoDB, Redis, AMQP with embedded credentials
- High-entropy strings in assignments and quoted values (charset-aware threshold: 3.3 for hex, 4.5 otherwise)
- Lines annotated `# crush:ignore-secret` are skipped
- `.env.example` / `.env.sample` / `.env.template` / `.env.dist` scanned for required vs optional env var classification
- Code references scanned: `process.env`, `os.environ`, `std::env::var`, `System.getenv`, `ENV[]`, `Deno.env.get`

Desktop GUI (v0.8+, Linux and Windows):
- Tauri 2 + Svelte cross-platform desktop app
- Real-time run events streamed per-container on `run-event::{run_id}` with tagged payloads
- Working abort — `Stop` kills the actual spawned process tree
- Data directory resolves gracefully: `CRUSH_DATA_DIR` env override → `/var/lib/crush` (if writable) → `dirs::data_dir()` fallback; startup error dialog instead of panic for non-root users
- Batched log replay (last 500 lines, 256KB window) so re-opening a container's log tab doesn't flood the UI
- Dev mode toggle plumbed through to `RunOptions { dev: true }`

Quality-of-life:
- `.crushignore` to extend the fingerprint skip list for big repos
- `--memory`, `--cpus`, `--priority` resource caps via Job Object
- `--watch` for hot-restart on source change
- Job Object guarantees the entire spawned tree dies on `crush` exit
- `crush eject` writes a marked Dockerfile + compose for prod
- `crush update` self-updates from GitHub releases
- `crush detect --json` / `crush ps --format json` / `crush services ps --format json` / `crush history --format json` for tooling

AI (requires `ANTHROPIC_API_KEY`):
- `crush debug <container>` — parse a stack trace, suggest a fix

---

## What's scaffolded but not yet daily-driven

- Linux runtime (`crush-runtime-linux`): scaffolded, not battle-tested
- macOS runtime (`crush-runtime-macos`): scaffolded, not built in CI
- WASM runtime (`crush-runtime-wasm`): scaffolded
- `crush scan` / `crush sbom`: present but limited coverage
- `crush registry` local server: present but underused
- Multi-arch builds (`--platform linux/amd64,linux/arm64`): cross-build works in CI for releases; native cross-compile from Windows is not supported
- `crush compose` full lifecycle: parses compose and starts deps, but doesn't manage the full `up/down/scale` lifecycle yet
- `crush watch` subcommand: on Windows the subcommand exits with a hint pointing at `--watch` (the subcommand uses Linux overlayfs); use `--watch` flag instead
- eBPF networking (`crush-network` ebpf feature): gated behind the `ebpf` feature flag; `crush-ebpf-progs` needs porting to `aya-ebpf` before it's buildable from a clean checkout
- `crush update` checksum verification: self-update works but does not yet verify SHA256 of the downloaded binary before replacing

---

## Install

### Linux (v0.8.4+)

```bash
# One-line install (downloads the latest Linux x86_64 binary)
curl -fsSL https://crush-web-six.vercel.app/install.sh | sh
```

Or grab a `.deb`, `.AppImage`, or the raw binary from the
[releases page](https://github.com/Chidi09/crush/releases/latest) — asset names
are version-pinned (e.g. `Crush_0.8.4_amd64.deb`), so use the install script
above for a version-agnostic command.

### macOS

No prebuilt macOS binary yet (signed bundles coming soon). Build from source:

```bash
cargo install --git https://github.com/Chidi09/crush crush-cli
```

### Windows

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
# Binary at target/release/crush-cli (crush-cli.exe on Windows) — rename to crush
```

Cross-compiling for Windows from Linux (what CI does):
```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64
cargo build --release --target x86_64-pc-windows-gnu -p crush-cli
```

> Note: `cargo build --all-features` is intentionally unsupported — the `ebpf` feature requires nightly + `bpf-linker`. Use `cargo build -p crush-cli` or `cargo check --workspace` (no `--all-features`).

You can also install from source with Cargo (no clone needed):
```bash
cargo install --git https://github.com/Chidi09/crush crush-cli
```

Homebrew, Scoop, and Winget packages are **not** published yet — they'll arrive
once macOS / Linux are first-class. A crates.io release (`cargo install crush-run`)
is planned; the `crush` and `crush-cli` names are already taken by unrelated
crates, so the published name will be `crush-run`.

### Optional: pgvector on Windows

If you have projects that use `pgvector/pgvector:*` images, install Visual Studio Build Tools:

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows10SDK.19041"
```

First `crush` run that needs pgvector clones the source, builds, and installs. Subsequent runs skip the build.

---

## CLI reference

| Command | What it does |
|---|---|
| `crush` | Detect → start deps → build → run |
| `crush build` | Build a Crush image from the current project |
| `crush run <image>` | Run a previously-built image |
| `crush detect` | Print detection (text) or `--json` |
| `crush eject` | Write `Dockerfile` + `docker-compose.yml` from detection |
| `crush update` | Self-update from GitHub releases |
| `crush ps` / `--format json` | List running containers |
| `crush services ps` / `--format json` / `--all-projects` | List native deps (postgres, garnet, …) |
| `crush history` / `--format json` | Recent build outcomes |
| `crush images` / `--format json` | List image manifests |
| `crush pull <ref>` | Pull from an OCI registry |
| `crush push <ref>` | Push to an OCI registry |
| `crush stop <id>` | Stop a container |
| `crush logs <id>` | Stream or tail container logs |
| `crush inspect <id>` | Show container / image details |
| `crush stats` | TUI dashboard (live CPU/mem sparklines) |
| `crush debug <id>` | AI crash diagnosis (needs `ANTHROPIC_API_KEY`) |
| `crush scan <image>` | Vulnerability scan |
| `crush sbom <image>` | Generate SBOM (SPDX-JSON) |
| `crush secrets set/list/export` | Manage encrypted secrets |
| `crush network create/ls` | Container networks |
| `crush volume create/ls` | Persistent volumes |
| `crush compose up/down` | Multi-container applications |
| `crush migrate` | Migrate a Dockerfile to Crushfile |
| `crush watch` | Watch mode (use `--watch` flag on Windows) |
| `crush deploy` | Deploy to a remote host |
| `crush rollback` | Roll back to the previous image |
| `crush system` | System info and diagnostics |
| `crush completions` | Shell completions (bash/zsh/fish/powershell) |

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

Crush is a Cargo workspace. Crates marked **active** are daily-driven; others are scaffolded for the roadmap.

```
crush/
├── crates/
│   ├── crush-cli/             # CLI entry, all subcommands, detection-to-spawn flow  [active]
│   ├── crush-build/           # Stack detection, Dockerfile/compose parsing,         [active]
│   │                          #   secrets & env scanning, build engine, RunEvent types
│   ├── crush-types/           # Shared structs, dirs_or_default, Container, Image    [active]
│   ├── crush-services/        # Native postgres/redis/mysql, pgvector extension      [active]
│   ├── crush-image/           # OCI image store + sled-backed metadata               [active]
│   ├── crush-registry/        # OCI pull (push partial)                              [partial]
│   ├── crush-reliability/     # Restart policies, health checks                      [partial]
│   ├── crush-runtime-windows/ # Job Objects, ConPTY                                  [active for cleanup]
│   ├── crush-runtime-linux/   # Linux namespace isolation                            [experimental]
│   ├── crush-runtime-macos/   # Apple Virtualization framework                       [experimental]
│   ├── crush-runtime-wasm/    # wasmtime + WASI preview 2                            [experimental]
│   ├── crush-compat/          # Dockerfile + compose parser                          [partial]
│   ├── crush-ai/              # Anthropic-backed crash diagnosis                     [active]
│   ├── crush-tui/             # ratatui-based ps/stats dashboard                     [active]
│   ├── crush-network/         # Reverse proxy, port mapping, eBPF (feature-gated)    [partial]
│   ├── crush-volume/          # Bind mounts, named volumes                           [partial]
│   ├── crush-api/             # HTTP API + unix socket surface                       [partial]
│   ├── crush-proto/           # OCI gateway protobuf definitions                     [partial]
│   ├── crush-deploy/          # Remote deployment                                    [partial]
│   └── crush-gui/src-tauri/   # Tauri 2 + Svelte desktop app                        [active]
├── crush-web/                 # Marketing site (Angular SSR)
├── scripts/
│   ├── ci-local.sh            # Local CI: check + lib tests + clippy + fmt
│   └── release-local.sh       # Local release: CLI binary + .deb + AppImage + SHA256SUMS
├── benches/                   # Criterion benchmarks
├── docs/                      # mdBook source
└── extensions/                # VS Code / JetBrains / Neovim integrations
```

### Key design decisions

- **Single static binary** — `crush` runs with no runtime deps. Release profile uses `lto = true`, `codegen-units = 1`, `strip = true`.
- **Async-first** — Tokio throughout.
- **Platform-gated runtimes** — each runtime crate is `#[cfg]`-gated; the Windows-specific `winreg` dep compiles only on Windows.
- **Content-addressable storage** — image store uses SHA-256 digests via `sha2` + `sled`.
- **Tagged serde enums for event contracts** — `RunEvent` uses `#[serde(tag = "kind", rename_all = "kebab-case")]` so the GUI frontend can discriminate events without stringly-typed checks.
- **eBPF excluded from workspace** — `crush-ebpf-progs` is built by `crush-network`'s `build.rs` (needs the BPF target + nightly). It's excluded from `[workspace]` so normal `cargo build` never resolves it.

---

## Contributing

Pick anything from the [issues](https://github.com/Chidi09/crush/issues) or:

- Add detection for a stack we don't handle yet (`crates/crush-build/src/detect.rs` — drop a `try_<stack>(root)` function)
- Extend secrets scanning with new token patterns (`crates/crush-build/src/analysis/secrets.rs`)
- Port `crush-ebpf-progs` to `aya-ebpf` (the crate currently uses the pre-rename `aya-bpf` deps)
- Fix `crush images` displaying "0 B" for image sizes
- Move a `partial` crate to `active` by completing its coverage

How to contribute:
1. Fork, branch from `main`
2. `cargo check --workspace --all-targets` must succeed (no `--all-features`)
3. `cargo test --workspace --lib --exclude crush-ebpf-progs` must pass
4. `cargo clippy --workspace --exclude crush-ebpf-progs -- -D warnings` must pass
5. PR with a clear description

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
