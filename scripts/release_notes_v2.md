## Crush v0.2.0

The runtime actually works now. v0.2.0 is a foundational correctness release — every command that previously printed placeholder output has been replaced with a real implementation.

---

### Installation

**Windows**
```
curl -LO https://github.com/Chidi09/crush/releases/download/v0.2.0/crush-0.2.0-windows-x86_64.exe
```

Run the binary directly. For global install with PATH, upgrade to [v0.3.0](https://github.com/Chidi09/crush/releases/tag/v0.3.0).

---

### What's new

#### Runtime fixes
- **Container entrypoint:** `crush run` was spawning `/sbin/init` with the container ID as an argument instead of the image's actual entrypoint. Fixed — the correct command is now loaded from the container config on disk.
- **PID persistence:** Container PIDs are now written to disk on start and restored on daemon restart, so `crush ps` correctly shows running containers after a crash or reboot.
- **cgroup v2 CPU limits:** Docker-style `cpu_shares` (1–262144) are now correctly converted to cgroup v2 `cpu.weight` (1–10000) before being written to the kernel. The old code passed raw values that could trigger `EINVAL`.
- **Debug logging removed:** The container runner was writing a step-by-step trace to `/tmp/crush_pre_exec_debug.log` on every container start. Stripped.

#### Build pipeline
- `crush build` and `crush default` now actually execute the detected build command (`npm install`, `cargo build --release`, `pip install`, etc.) inside the project directory. Previously this step stored a placeholder string and returned immediately.
- If a `Crushfile` is present in the project root, its `[[stages]]` are parsed and used directly instead of auto-generated ones.

#### CLI command completions
- `crush logs --follow` tails the live log file instead of streaming only the initial snapshot.
- `crush inspect` loads and displays real container/image data from disk instead of a stub.
- `crush health` runs the container's configured health check command with the configured timeout.
- `crush volume` subcommands (`ls`, `create`, `rm`, `inspect`) are wired to the `VolumeManager`.
- `crush daemon` starts the real API server (Unix socket on Linux, named pipe on Windows).
- `crush docker-context` writes a valid Docker CLI context to `~/.docker/contexts/` so `docker` commands are routed to Crush.
- `crush ps` shows human-readable uptime ("Up 3 hours") and dynamically sized columns.
- `crush export` produces a standards-compliant OCI tarball with `oci-layout`, blobs, and `index.json`.
- `crush pull` and `crush build` accept a `--platform` flag (e.g. `linux/arm64`) for multi-arch image handling.

#### Windows runtime
- Real PID tracking — `get_pid()` no longer returns a hardcoded `4242`.
- `exec()` spawns a process inside the existing Job Object.
- `delete()` removes the Job Object handle and cleans up the container state directory.
- `crush run` on Windows now dispatches to `WindowsRuntime` instead of the Linux runner path.

---

### Assets
| File | Platform |
|------|----------|
| `crush-0.2.0-windows-x86_64.exe` | Windows x86-64 |

Linux and macOS binaries can be self-compiled: `cargo build --release -p crush-cli`
