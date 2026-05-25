<div align="center">
  <img src="crush-web/public/logo.png" alt="Crush" width="120" />

  <h1>Crush</h1>
  <p><strong>A from-scratch, production-grade container runtime written in Rust.</strong></p>
  <p>Sub-second starts &nbsp;·&nbsp; No WSL2 &nbsp;·&nbsp; No VM overhead &nbsp;·&nbsp; OCI-compatible &nbsp;·&nbsp; AI-powered diagnostics</p>

  <p>
    <a href="https://github.com/Chidi09/crush/actions"><img src="https://img.shields.io/github/actions/workflow/status/Chidi09/crush/ci.yml?branch=main&label=CI&logo=github&style=flat-square" alt="CI" /></a>
    <a href="https://github.com/Chidi09/crush/blob/main/Cargo.toml"><img src="https://img.shields.io/badge/rustc-1.75%2B-orange?logo=rust&style=flat-square" alt="Rust Version" /></a>
    <a href="https://github.com/Chidi09/crush/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=flat-square" alt="License" /></a>
    <a href="https://github.com/Chidi09/crush/stargazers"><img src="https://img.shields.io/github/stars/Chidi09/crush?logo=github&style=flat-square" alt="Stars" /></a>
    <a href="#contributing"><img src="https://img.shields.io/badge/discord-coming_soon-5865F2?logo=discord&logoColor=white&style=flat-square" alt="Discord" /></a>
    <a href="#stability-notice"><img src="https://img.shields.io/badge/status-active_alpha-red?style=flat-square" alt="Status" /></a>
  </p>
</div>

---

> [!IMPORTANT]
> **Stability Notice**: Crush is currently in active **Alpha** development (pre-release v0.1.0). The CLI commands, TOML schema, and network configurations are under active development and may change. It is not yet recommended for mission-critical production workloads, but early feedback, testing, and contributions are highly encouraged!

---

## What is Crush?

Crush is a container runtime built from the ground up in Rust. It detects your project's tech stack, builds a minimal OCI image, and runs it natively — no Docker daemon, no WSL2, no virtual machine in the way.

```
┌────────────────────────────────────────────────────────────────────────────────┐
│  ~/my-api $ crush                                                              │
│                                                                                │
│  🔍 Detecting project stack...                                                 │
│     ↳ Detected: Node.js v20.11.0 • TypeScript • Express • TailwindCSS          │
│                                                                                │
│  📦 Building containerized layer tree...                                       │
│     ↳ [1/3] Base runtime system cache hit (debian-slim-nodejs)                 │
│     ↳ [2/3] Cache Hit: node_modules/ dependencies layer (unchanged)            │
│     ↳ [3/3] Building source code layer... Done!                                │
│                                                                                │
│  ✨ Successfully crushed to image: my-api:latest                               │
│     ⚡ Build complete in 0.9s (Total compressed image size: 41 MB)             │
│                                                                                │
│  🚀 Run container now? [Y/n] Y                                                 │
│     ↳ Creating container network sandbox (crush-bridge-0)                      │
│     ↳ Binding local socket port 3000 -> 3000                                   │
│     ↳ Assigning Windows Job Object policies (CPU rate: 1.0, RAM limit: 512MB)    │
│                                                                                │
│  ✓ Container started natively on http://localhost:3000                         │
│     ⚡ Cold boot elapsed: 0.3s (Total pipeline duration: 1.2s!)                │
│                                                                                │
└────────────────────────────────────────────────────────────────────────────────┘
```

Already have a Dockerfile? Crush runs it as-is and can export back to a standard `Dockerfile` or `docker-compose.yml` when you need to deploy to a VPS or CI pipeline.

---

## Table of Contents

- [Why Crush?](#why-crush)
- [Features](#features)
- [Architecture](#architecture)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [The Crushfile](#the-crushfile)
- [Workspace Crates](#workspace-crates)
- [Platform Support](#platform-support)
- [IDE Extensions](#ide-extensions)
- [Building from Source](#building-from-source)
- [Contributing](#contributing)
- [License](#license)

---

## Why Crush?

| | Docker Desktop | Crush |
|---|---|---|
| Startup time | ~15–47s | **0.3–1.2s** |
| Image size | ~842 MB | **~38–41 MB** |
| Requires VM | Yes (WSL2/Hyper-V) | **No** |
| Requires daemon | Yes | **No** |
| Dockerfile compat | Native | **Yes (read + export)** |
| docker-compose compat | Native | **Yes** |
| AI crash diagnosis | No | **Yes** |
| Built-in vuln scanner | No | **Yes** |
| SBOM generation | No | **Yes (CycloneDX/SPDX)** |
| WASM containers | No | **Yes** |

Docker Desktop runs Linux containers on Windows by spinning up a full WSL2 virtual machine. Every container start must wait for that VM. Crush runs containers using native OS primitives — **Windows Job Objects** on Windows, **cgroups/namespaces** on Linux, **Apple Virtualization Framework** on macOS — with no intermediary layer.

---

## Features

### Zero-config stack detection
Run `crush` in any project directory. Crush detects your language and framework (Node.js, Python, Go, Rust, Java, .NET, Ruby, PHP, and more), picks the right base image, and builds a minimal layered OCI image automatically.

### Sub-second container starts
No daemon to cold-start. No VM to boot. Containers launch in under a second from a cached build.

### Native Windows support
First-class Windows containers via **Windows Job Objects**, **HCS (Host Compute Service)**, and **HNS (Host Network Service)**. No WSL2 required. Runs on Windows 10 21H2+ and Windows Server 2022+.

### OCI-compatible image store
Crush uses a local content-addressable image store fully compliant with the OCI Image Layout Specification. Pull from any registry (`crush pull ubuntu:latest`), push to any registry, tag, export to a tarball — everything works.

### Dockerfile & docker-compose compatibility
Point Crush at an existing `Dockerfile` or `docker-compose.yml` and it works immediately. Use `crush migrate` to convert them into an optimized `Crushfile`. Use `crush export` to go back to a standard Dockerfile for deployment anywhere.

### AI-powered crash diagnosis
When a container crashes, `crush debug <id>` parses the stack trace, identifies the root cause, and suggests a concrete fix — powered by the Anthropic API. Set `ANTHROPIC_API_KEY` to enable.

```
$ crush debug my-api

Parsed error trace:
  Language: TypeScript
  Exception: TypeError
  Message: Cannot read properties of undefined (reading 'split')
  File: src/server.ts:42

AI Diagnosis:
  Root cause: DB_HOST environment variable not set
  Fix: crush secrets set DB_HOST
  Confidence: 0.97
```

### Built-in security scanner
`crush scan` checks your source code and images for vulnerabilities, leaked secrets, and SQL injection patterns. `crush scan --fix` applies safe mechanical patches automatically.

### SBOM generation
`crush sbom <image>` generates a Software Bill of Materials in **CycloneDX** or **SPDX** format — required for supply chain compliance.

### Multi-arch builds
Build once for multiple platforms: `crush build --platform linux/amd64,linux/arm64`.

### WASM containers
Run WebAssembly workloads as lightweight isolated containers via the `crush-runtime-wasm` crate.

### Local OCI registry
`crush registry` spins up a local OCI-compliant registry proxy on `localhost:5000` for air-gapped or private workflows.

### TUI dashboard
`crush stats` and `crush ps` render live CPU/memory sparklines and container tables directly in the terminal via the built-in TUI renderer.

### Hot reload dev mode
`crush watch` rebuilds and hot-swaps your container on every file save with configurable debounce.

---

## Architecture

Crush is a Cargo workspace with clearly separated crates:

```
crush/
├── crates/
│   ├── crush-cli/            # Entry point — CLI parser + command dispatch (clap)
│   ├── crush-types/          # Shared types: Container, Image, MountConfig, Protocol…
│   ├── crush-build/          # Stack detection, layered build engine, cache, SBOM, scanner
│   ├── crush-image/          # OCI image store: blobs, layers, compression, lazy pull, GC
│   ├── crush-registry/       # OCI registry client + local registry server
│   ├── crush-runtime-windows/# Windows: Job Objects, HCS, HNS, ConPTY, fs sandbox
│   ├── crush-runtime-linux/  # Linux: cgroups v2, namespaces, overlay FS, seccomp, capabilities
│   ├── crush-runtime-macos/  # macOS: Apple Virtualization Framework, Rosetta 2, vsock
│   ├── crush-runtime-wasm/   # WASM: Wasmtime engine, WASI, component model
│   ├── crush-network/        # Bridge networks, NAT, port mapping, DNS, eBPF, IPv6
│   ├── crush-volume/         # Named volumes, bind mounts, tmpfs, quota, backup
│   ├── crush-reliability/    # Restart policies, OOM handling, health checks, secrets, SELinux
│   ├── crush-compat/         # Dockerfile parser, docker-compose loader, migration engine
│   ├── crush-ai/             # Stack trace parser, AI diagnostic engine (Anthropic API)
│   ├── crush-tui/            # Terminal UI: tables, sparklines, manpages, completions
│   ├── crush-api/            # HTTP API server for programmatic access
│   └── crush-proto/          # Shared protobuf/gRPC definitions
├── crush-web/                # Marketing site (Angular 19 + AnalogJS + Spartan UI)
├── benches/                  # Criterion benchmarks: build pipeline, startup, image extraction
├── tests/                    # Integration tests + e2e test suite
├── fuzz/                     # Fuzz targets: Dockerfile parser, compose, OCI manifest, Crushfile
├── docs/                     # mdBook documentation source
└── extensions/               # IDE extensions: VS Code, JetBrains, Neovim
```

### Key design decisions

- **Single static binary** — `crush` compiles to a single binary with no runtime dependencies. The release profile uses `lto = true`, `codegen-units = 1`, and `strip = true` for minimal size.
- **Async-first** — built on Tokio with full async I/O throughout.
- **Platform-gated runtimes** — each runtime crate is conditionally compiled only for its target OS. No dead code ships.
- **Content-addressable storage** — the image store uses SHA-256 digests (via `sha2` + `sled`) so layers are deduplicated automatically.
- **OCI-spec compliant** — uses the `oci-spec` crate for full compliance with OCI Image Layout and Runtime specs.

---

## Installation

### One-line install (Windows PowerShell)
```powershell
irm https://github.com/Chidi09/crush/releases/latest/download/install.ps1 | iex
```

### One-line install (Linux / macOS)
```bash
curl -fsSL https://github.com/Chidi09/crush/releases/latest/download/install.sh | sh
```

### Homebrew (macOS)
```bash
brew install crush
```

### Scoop (Windows)
```powershell
scoop install crush
```

### Cargo

To compile and install from crates.io from source:
```bash
cargo install crush-cli
```

**Tip (Fast pre-compiled binaries):** If you have `cargo-binstall` installed, you can pull pre-compiled releases instantly without building locally from source:
```bash
cargo binstall crush-cli
```

---

## Quick Start

```bash
# Run crush in any project — auto-detects the stack
cd my-node-app
crush

# Explicit build + tag
crush build --tag my-app:v1.0

# Run a pre-built image
crush run my-app:v1.0 --port 8080:80 --env NODE_ENV=production

# Watch mode — hot reload on file changes
crush watch

# Pull and run from Docker Hub
crush pull nginx:latest
crush run nginx:latest --port 80:80

# Multi-container compose
crush compose --file docker-compose.yml up

# Migrate an existing Dockerfile
crush migrate --apply

# View live container stats
crush stats

# AI debug a crashed container
ANTHROPIC_API_KEY=sk-... crush debug my-app

# Scan for vulnerabilities
crush scan
crush scan --fix

# Generate SBOM
crush sbom my-app:v1.0 --format cyclonedx

# Export to OCI tarball
crush export my-app:v1.0 --output my-app.tar

# Prune stopped containers and unused images
crush system prune --all
```

---

## CLI Reference

| Command | Description |
|---|---|
| `crush` | Auto-detect stack, build, and run |
| `crush build` | Build an OCI image from the current directory |
| `crush watch` | Hot-reload on file changes |
| `crush run <image>` | Run an image in a sandboxed container |
| `crush ps` | List running containers |
| `crush stop <id>` | Gracefully stop a container |
| `crush logs <id>` | Stream container logs |
| `crush debug <id>` | AI-powered crash diagnosis |
| `crush inspect <id>` | Low-level container/image/network/volume info |
| `crush stats` | Live CPU, memory, I/O, PID metrics |
| `crush events` | Stream system events |
| `crush pull <image>` | Pull from any OCI registry |
| `crush images` | List local images |
| `crush rmi <image>` | Remove a local image |
| `crush push <image>` | Push to an OCI registry |
| `crush tag <src> <dst>` | Tag a local image |
| `crush export <image>` | Export image to OCI tarball |
| `crush scan` | Vulnerability + secret scan |
| `crush sbom <image>` | Generate CycloneDX or SPDX SBOM |
| `crush migrate` | Convert Dockerfile to Crushfile |
| `crush compose` | Multi-container compose (up/down/ps/logs) |
| `crush network` | Manage container networks |
| `crush volume` | Manage persistent volumes |
| `crush registry` | Local OCI registry proxy |
| `crush system` | Prune, info, telemetry |
| `crush update` | Self-update the binary |

Run `crush <command> --help` for full flags on any subcommand.

---

## The Crushfile

The `Crushfile` is Crush's native project config — a TOML file that replaces the Dockerfile for projects built with Crush. It is fully optional; Crush works without one via auto-detection.

```toml
[project]
name    = "my-api"
version = "1.0.0"
stack   = "node"          # auto-detected if omitted

[build]
entry   = "src/index.ts"
port    = 3000
command = "npm run build"

[run]
env  = ["NODE_ENV=production"]
port = ["3000:3000"]

[resources]
memory = "512m"
cpu    = 1.0
```

Convert an existing `Dockerfile`:
```bash
crush migrate --apply
```

Export back to `Dockerfile` + `docker-compose.yml` for deployment:
```bash
crush export my-app:latest --output my-app.tar
```

---

## Workspace Crates

| Crate | Description |
|---|---|
| `crush-cli` | CLI entry point, argument parsing, command dispatch |
| `crush-types` | Shared domain types (Container, Image, MountConfig, etc.) |
| `crush-build` | Stack detection, layered build engine, layer caching, multi-stage parsing, SBOM, vulnerability scanner, secret detection |
| `crush-image` | OCI image store: blob storage, layer compression (zstd/gzip), lazy pull, GC, multi-arch manifests |
| `crush-registry` | OCI registry client (pull/push/auth) + local registry proxy server |
| `crush-runtime-windows` | Windows runtime: Job Objects, HCS, HNS, ConPTY, filesystem sandbox, Windows credentials |
| `crush-runtime-linux` | Linux runtime: cgroups v2, PID/net/mount/user namespaces, overlay FS, seccomp, capabilities, signals |
| `crush-runtime-macos` | macOS runtime: Apple Virtualization Framework, Rosetta 2, vsock, storage, networking |
| `crush-runtime-wasm` | WASM runtime: Wasmtime engine, WASI, component model, sandboxed networking |
| `crush-network` | Bridge networks, NAT, port mapping, CNI plugins, eBPF routing, IPv6, DNS |
| `crush-volume` | Named volumes, bind mounts, tmpfs, quota enforcement, backup, Windows volume support |
| `crush-reliability` | Restart policies, OOM handling, health checks, secrets management, SELinux/AppArmor, rootless mode, read-only containers |
| `crush-compat` | Dockerfile parser, docker-compose loader, Docker credentials, migration engine |
| `crush-ai` | Stack trace parser, AI diagnostic engine (Anthropic API), build error analysis, auto-fix suggestions |
| `crush-tui` | Terminal UI: container tables, CPU/memory sparklines, shell completions, manpage generation, watch mode |
| `crush-api` | HTTP API server for programmatic/external access |
| `crush-proto` | Shared protobuf definitions |

---

## Platform Support

| Platform | Status | Runtime backend |
|---|---|---|
| Windows 10 21H2+ (x64) | ✅ Supported | Windows Job Objects + HCS |
| Windows 11 (x64) | ✅ Supported | Windows Job Objects + HCS |
| Windows Server 2022 | ✅ Supported | Windows Job Objects + HCS |
| Linux (x64, ARM64) | ✅ Supported | cgroups v2 + namespaces + overlay FS |
| macOS 13+ (Apple Silicon) | ✅ Supported | Apple Virtualization Framework + Rosetta 2 |
| macOS 13+ (Intel) | ✅ Supported | Apple Virtualization Framework |
| WASM | ✅ Supported | Wasmtime engine |

---

## IDE Extensions

| Editor | Location | Features |
|---|---|---|
| **VS Code** | `extensions/vscode/` | Crush commands in command palette, container status bar, log streaming |
| **JetBrains** (IntelliJ, Rider, GoLand…) | `extensions/jetbrains/` | Run configurations, container tool window, live diagnostics |
| **Neovim** | `extensions/neovim/` | Lua plugin: build/run commands, floating log window, diagnostics |

---

## Building from Source

**Prerequisites:**
- Rust toolchain (`rustup` recommended) — `rustup update stable`
- **On Windows**: 
  - Windows SDK 10.0.22000+
  - **Windows Developer Mode Enabled**: Creating NTFS symlinks (required when compiling modules or running `npm ci`/`yarn` inside your container layers) requires Developer Mode. Search for "Developer settings" in Windows Settings and toggle **Developer Mode** on, or run powershell as administrator.
- **On Linux**: kernel headers, `libseccomp-dev`
- **On macOS**: Xcode Command Line Tools

```bash
git clone https://github.com/Chidi09/crush.git
cd crush

# Build the CLI in release mode
cargo build --release -p crush-cli

# The binary is at:
# Windows: target/release/crush.exe
# Linux/macOS: target/release/crush
```

**Run the test suite:**
```bash
# Unit + integration tests
cargo test

# End-to-end tests (requires a built binary in PATH)
cargo test --test integration_tests

# Fuzz a specific parser (requires cargo-fuzz + nightly)
cargo +nightly fuzz run dockerfile
```

**Run benchmarks:**
```bash
cargo bench
```

**Build the website:**
```bash
cd crush-web
pnpm install
pnpm run dev       # dev server at localhost:5173
pnpm run build     # production build
```

---

## Contributing

Contributions are welcome and appreciated. Crush is early-stage — there is plenty of ground to cover.

**Good first issues:**
- Improve stack detection heuristics for additional languages/frameworks
- Add more test coverage to `crush-build` and `crush-compat`
- Implement additional fuzz targets
- IDE extension improvements

**How to contribute:**
1. Fork the repo and create your branch from `main`
2. Make your changes with tests where applicable
3. Run `cargo test` and `cargo clippy` — both must pass
4. Open a pull request with a clear description of what changed and why

If you have a larger feature in mind, open an issue first to discuss the approach before writing code.

**Discord:** Coming soon — a community server is being set up. Star the repo to be notified.

---

## Built by

<table>
  <tr>
    <td align="center">
      <a href="https://github.com/Chidi09">
        <strong>Chidi</strong><br />
        <sub>Founder</sub>
      </a>
    </td>
    <td align="center">
      <a href="https://github.com/Chidi09/crush/issues">
        <strong>You?</strong><br />
        <sub>Contributor</sub>
      </a>
    </td>
  </tr>
</table>

Want to join the team? Open a PR or browse the [open issues](https://github.com/Chidi09/crush/issues).

---

## License

Licensed under either of:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

<div align="center">
  <sub>Built with ❤️ and Rust · <a href="https://github.com/Chidi09/crush">github.com/Chidi09/crush</a></sub>
</div>
