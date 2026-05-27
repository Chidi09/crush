## Crush v0.4.0

Three major capability upgrades: **cloud deployment** (Crushfile → production server in one command), **eBPF-based per-container metrics** (network + disk I/O from the kernel), and a **smarter stack detector** that reads your actual dependency files instead of guessing from filenames.

---

### Installation

**Windows — first-time install**
```
curl -LO https://github.com/Chidi09/crush/releases/download/v0.4.0/crush-0.4.0-windows-x86_64.exe
crush-0.4.0-windows-x86_64.exe install
```

**Upgrading from v0.3.x**
```
crush update
```

---

### What's new

#### `crush deploy` — one-command cloud deployment

Add a `[deploy]` section to your Crushfile and run `crush deploy`. Crush builds the image, exports an OCI tarball, provisions the server (or reuses an existing one), and starts the container remotely.

```toml
[deploy]
provider = "hetzner"
region = "nbg1-dc3"
server_type = "cx21"

[deploy.hetzner]
api_token = "${HETZNER_API_TOKEN}"
ssh_key_name = "my-key"
```

```
crush deploy           # build + provision + deploy
crush deploy --status  # show URL, server, deployed-at
crush deploy --logs    # tail container logs after deploy
crush deploy --destroy # remove the server
```

**Supported providers:** Hetzner Cloud, DigitalOcean, AWS EC2, GCP Compute Engine, Fly.io, and any server accessible over SSH.

For all cloud providers, Crush SSHes in to install itself and run the container — no Docker, no registry, no Kubernetes.

**Deployment state** is persisted to `~/.crush/deployments/<project>.json` so subsequent `crush deploy` calls update in-place rather than creating new servers.

#### eBPF metrics — network + disk I/O per container

When running on a Linux kernel ≥ 5.4 with BTF, the `crush stats` TUI now shows real per-container network and disk I/O sourced from the kernel via two new eBPF programs:

- `crush_net_ingress` / `crush_net_egress` — `cgroup_skb` programs attached to each container's cgroup, counting bytes in and out.
- `crush_block_rq_complete` — `tracepoint/block/block_rq_complete` counting bytes read and written per cgroup when block requests complete.

The stats view gains four new sparklines: **NET IN**, **NET OUT**, **DISK R**, **DISK W**, color-coded green < 1 MB/s, yellow 1–10 MB/s, red > 10 MB/s.

On kernels without BTF or eBPF support, the same columns fall back to `/proc/<pid>/net/dev` and `/proc/<pid>/io` for approximate per-process I/O rates.

#### Detector overhaul — signal scoring with real dep-tree parsing

The stack detector no longer relies purely on file-existence heuristics. It now reads your actual dependency manifests and scores signals:

- **Node.js:** reads `dependencies`, `devDependencies`, and `peerDependencies` from `package.json`. Config files (e.g. `next.config.ts`) score 10 pts, direct deps (e.g. `"next"` in deps) score 8 pts, start-script patterns score 4–5 pts. Winner is the framework with the highest score.
- **Python:** parses `requirements.txt` line by line and `pyproject.toml` (`[project.dependencies]` for PEP 621, `[tool.poetry.dependencies]` for Poetry). FastAPI, Flask, Django, Tornado, aiohttp, Starlette, Litestar are all detected from the actual installed packages.
- **Ruby:** parses `Gemfile` declarations (`gem 'rails'`, `gem "sinatra"`, etc.) instead of inferring from directory structure.
- **PHP:** parses `composer.json` `require` and `require-dev` sections instead of reading the file as a raw string.

When a framework is found as a **direct dependency** (not just a config file), confidence is raised to **0.99** — meaning crush is as certain as it can be without running the code.

---

### Assets
| File | Platform |
|------|----------|
| `crush-0.4.0-windows-x86_64.exe` | Windows x86-64 |

Linux and macOS binaries can be self-compiled: `cargo build --release -p crush-cli`
