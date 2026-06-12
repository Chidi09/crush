# Crush Runtime

Welcome to the documentation for **Crush** v0.8.1: a from-scratch container runtime written entirely in Rust.

## What is Crush?

Crush is a daemonless, native runtime that detects your project's stack and runs it with dependencies already up — no Docker daemon, no WSL2, no VMs. For production you `crush eject` to a real Dockerfile + compose and deploy anywhere.

### What's actually shipped

- **Zero-config stack detection** — language, framework, port, monorepo structure. Crushfile > compose > Dockerfile > heuristics. Files annotated `# crush:eject` are excluded so ejected Dockerfiles never loop back.
- **Native deps** — PostgreSQL, Garnet (Redis-compat), MySQL started natively by parsing `docker-compose.yml` / `application.yml` / `.env`. Per-service env merge for monorepos.
- **Warm-run optimization** — skips `pnpm install` when `node_modules` is newer than the lockfile; skips image repack when source fingerprints match.
- **Secrets scanning** — runs before execution. Detects AWS / GCP / GitHub / OpenAI / GitLab / npm / Vercel / Cloudflare tokens; DB connection strings; high-entropy values. Escape hatch: `# crush:ignore-secret`.
- **Env classification** — reads `.env.example` / `.env.sample` / `.env.template` and code references (`process.env`, `os.environ`, `std::env::var`, etc.) to distinguish required from optional variables.
- **Desktop GUI** (v0.8+) — Tauri 2 + Svelte cross-platform app. Real-time run events, working abort, batched log replay, dev mode toggle.
- **Windows Job Objects** — the entire spawned process tree is killed on exit. No orphan processes holding ports between runs.
- **AI crash diagnosis** — `crush debug <container>` sends the stack trace to Claude and suggests a fix (requires `ANTHROPIC_API_KEY`).

### What's scaffolded but not yet daily-driven

- Linux / macOS / WASM runtimes: crates exist, not battle-tested
- eBPF networking (`ebpf` feature flag): needs `crush-ebpf-progs` ported to `aya-ebpf`
- `crush scan` / `crush sbom`: present but limited coverage
- `crush update` checksum verification: self-update works but SHA256 of the downloaded binary is not yet verified

## Getting Started

- [Installation](user_guide/installation.md)
- [Quickstart Guide](user_guide/quickstart.md)
- [CLI Reference](user_guide/commands.md)
- [Architecture Overview](architecture/README.md)
