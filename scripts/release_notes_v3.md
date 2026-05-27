## Crush v0.3.0

Three quality-of-life upgrades: **PATH self-installation**, a smarter **project detector**, and a readable **inspect** command.

---

### Installation

**Windows — first-time install**
```
curl -LO https://github.com/Chidi09/crush/releases/download/v0.3.0/crush-0.3.0-windows-x86_64.exe
crush-0.3.0-windows-x86_64.exe install
```
Copies the binary to `%LOCALAPPDATA%\crush\bin\crush.exe` and adds it to your user PATH — no admin rights required, takes effect in any new terminal immediately.

**Upgrading from v0.2.0**
```
crush update
```
Downloads the new binary and automatically re-runs the PATH install so your shell entry stays current.

---

### What's new

#### Global PATH self-installation
- New `crush install` subcommand — copies the binary to a stable location and writes the directory to the Windows user PATH registry key (`HKCU\Environment`), then broadcasts `WM_SETTINGCHANGE` so new terminals see it without logging off. No admin rights needed.
- On Linux, installs to `~/.local/bin/crush` with executable permissions.
- `crush update` now calls `crush install` after a successful download, keeping the PATH entry up to date automatically.
- New installer scripts: `scripts/install.ps1` (PowerShell) and `scripts/install.sh` (bash).

#### Smarter project detector
- **Bug fix — Go frameworks:** Previously checked for a directory named `gin` in the project root (wrong). Now correctly scans `go.mod` for import paths like `github.com/gin-gonic/gin`, `github.com/labstack/echo`, `github.com/gofiber/fiber`, `github.com/go-chi/chi`, `google.golang.org/grpc`.
- **Bug fix — Node package manager:** Build commands always said `npm` even for pnpm/yarn projects. Now checks for `pnpm-lock.yaml` and `yarn.lock` first and outputs the correct `pnpm run build` or `yarn build`.
- **New frameworks:** SvelteKit, Astro, SolidStart, Qwik, Laravel, Symfony.
- **New Python toolchains:** `uv` (`uv sync --no-dev`), `pdm` (`pdm install --prod`), conda (`environment.yml`).
- **Versioned base images:** Instead of always pulling `ubuntu:22.04`, the build pipeline now resolves an optimised base image from the detected runtime version — `node:20-alpine`, `python:3.11-slim`, `golang:1.22-alpine`, `eclipse-temurin:21-jre-alpine`, etc.
- `crush detect --json` outputs machine-readable JSON for scripting and CI pipelines.

#### Readable inspect output
- `crush inspect` now prints a structured, human-readable report by default — container state, ports table, mounts table (bind vs tmpfs, ro/rw), cgroup resource limits, health check status, and restart policy.
- Raw JSON is still available: `crush inspect <id> --format json`
- Same formatting applied to image inspection: architecture, digest, entrypoint, layers, environment variables.

---

### Assets
| File | Platform |
|------|----------|
| `crush-0.3.0-windows-x86_64.exe` | Windows x86-64 |

Linux and macOS binaries can be self-compiled: `cargo build --release -p crush-cli`
