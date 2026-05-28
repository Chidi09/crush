# Plan: dev/prod parity + perf wins (v0.7.41 → v0.7.45)

Three shipping units in order of ROI. Each is independently mergeable.

---

## Unit A — Built-in reverse proxy (`v0.7.41`)

### Goal
Make `localhost:5173` (frontend) + `localhost:8080` (backend) look like
production: one origin, real path routing. Kills CORS-only-in-dev bugs
and "API calls work locally but break behind nginx" surprises.

### Data model
Add to `InferredStack` (and per `SubService`):
```rust
pub struct ProxyRoute {
    pub path_prefix: String,   // "/api", "/" (catch-all)
    pub target: String,        // "http://localhost:8080"
    pub strip_prefix: bool,    // default false; true if upstream expects bare paths
}

pub struct ProxyConfig {
    pub bind_port: u16,        // default 8000
    pub routes: Vec<ProxyRoute>,
}
```

Sources, in priority order:
1. `crush.toml` `[proxy]` table (explicit user override)
2. `infra/nginx.conf` / `nginx/*.conf` parsed for `location` blocks (rough; only proxy_pass directives)
3. Inferred from multi-service detection: `/api/*` → backend, `/` → frontend
4. Skip entirely if only one service

### Implementation
- New crate `crush-proxy` (or module under `crush-cli`). Hyper 1.x server + reqwest client; ~250 LOC.
- Spawn proxy task in `main.rs` after sub-services are bound, before Ready panel.
- Wait for upstreams to be reachable before binding (avoid 502s on first request).
- Ready panel surfaces the proxy URL as primary: `↳ http://localhost:8000 (proxy)`, individual ports as `(direct)`.
- `--no-proxy` flag to disable for users who don't want it.
- Bind retry: if `:8000` is taken, try `:8001..8010`.

### Edge cases
- WebSocket upgrade (Next HMR, Vite HMR): proxy must handle `Upgrade: websocket`. Hyper supports it but needs explicit handling — non-trivial.
- Cookies: pass `Set-Cookie` through unchanged. `Host` header rewrites are off by default.
- Large request bodies (uploads): stream, don't buffer.
- Don't proxy if user already has nginx running on :8000.

### Done when
- safe-meet: `crush` exposes a single `localhost:8000` that routes `/api/*` to api:4000 and the rest to web:3000.
- HMR still works through the proxy (websocket upgrade verified).
- `crush --no-proxy` keeps the current per-port Ready panel.

---

## Unit B — Env diff + lint (`v0.7.42`)

### Goal
Catch "works on my machine because $env:FEATURE_X is set" before it hits
CI. **No scrubbing** — just a loud warning at startup.

### Implementation
- New helper `env_audit(project_root) -> EnvAudit` in `crush-build`.
  - Parses `.env`, `.env.local`, `.env.production`, `.env.development` (cli.dev picks the variant).
  - Greps source files for `process.env.X`, `os.environ['X']`, `os.Getenv("X")`, `std::env::var("X")`, `env('X')` (PHP/Laravel).
  - Returns `{declared_in_dotenv: Set, read_by_code: Set, present_in_shell: Set}`.
- At startup, print:
  - `⚠ env vars read by code but not in .env: FOO, BAR (inherited from shell — won't exist in prod)`
  - `⚠ env vars in .env but not read by code: BAZ (dead config)`
- `--strict-env` flag: instead of warning, fail with non-zero exit.
- `--no-env-lint` to silence.

### Edge cases
- Don't false-positive on stdlib vars (`PATH`, `HOME`, `TEMP`, `USERPROFILE`, `SystemRoot`, `APPDATA`, `LOCALAPPDATA`, `PROGRAMFILES`, `LANG`, `LC_*`, `TZ`, `SHELL`, `USER`, `LOGNAME`).
- Multi-service: per-service `.env` files (`apps/api/.env`, etc.) — audit each separately.
- Computed env keys (`process.env[someVar]`) — skip silently, can't statically detect.

### Done when
- Running `crush` in a project where code reads `SUPABASE_URL` but `.env` is missing it prints a clear warning at the top.
- `crush --strict-env` exits 1 in that case.

---

## Unit C — Parallel multi-service builds (`v0.7.43`)

### Goal
Cut wall-clock build time in multi-service projects by ~40%. Today we
build backend → frontend → ... sequentially.

### Implementation
- Refactor the multi-service spawn block in `main.rs` (~line 1567).
- Build step becomes:
  ```rust
  let build_tasks: Vec<_> = stack.services.iter()
      .filter_map(|sub| build_command_for(sub))
      .map(|(sub, cmd)| tokio::spawn(run_build(sub, cmd)))
      .collect();
  let results = futures::future::join_all(build_tasks).await;
  if results.iter().any(|r| r.is_err()) { bail!("one or more builds failed"); }
  ```
- Per-service build output already has `[name]` prefix coloring — interleaving is fine.
- Add a top-of-block summary line: `⚙ building 3 services in parallel...`.

### Edge cases
- Dep ordering: in a turbo monorepo, `apps/api` depends on `packages/shared`. We don't currently model this. Two options:
  - (A) Run each sub-service's own build command (which delegates to turbo/nx — they handle internal order). Already what we do.
  - (B) Add a `depends_on: [shared]` field to `SubService`. Defer to v0.7.44 if (A) proves insufficient.
- Resource contention: 4 parallel `cargo build`s on a laptop will OOM. Cap at `min(num_cpus, 4)` concurrent builds via a semaphore.
- Output interleaving: turbo's own prefix collides visually with ours (`web:build:` inside `[web]` line). Strip turbo's prefix when it matches the service name.

### Done when
- NCIC (backend Go + frontend Vite) builds in `max(go, vite)` time instead of `go + vite`.
- safe-meet still works (single root service, no behavior change).

---

## Bonus track (v0.7.44+)

### Job Object resource limits
- Extend `job_object::init()` to accept `Option<Limits { memory: u64, cpu_percent: u8 }>`.
- New `--memory 4G` and `--cpus 2` flags.
- Use `JobObjectExtendedLimitInformation.ProcessMemoryLimit` and CPU rate control (`JobObjectCpuRateControlInformation`).
- ~30 LOC, surfaces OOMs locally before they fire in prod.

### Real port-binding observation
- After spawning a service, poll `GetExtendedTcpTable` (Windows) / `/proc/net/tcp` (Linux) filtered by the child's PID.
- If the bound port differs from what we guessed, update the Ready panel.
- Kills the "no response on :8080 after 10s — app may be on a different port" warning when the app bound :3001.

### Persistent service daemon (`crushd`)
- Single-binary daemon that keeps dep-service state, image cache index, port assignments, and proxy routes warm.
- `crush` becomes a thin client that IPCs to `crushd` over a named pipe.
- Big lift (probably 2-3 weeks). Defer until A/B/C land and we measure cold-start time.

---

## Sequencing
- v0.7.41: Unit A (reverse proxy)
- v0.7.42: Unit B (env diff)
- v0.7.43: Unit C (parallel builds)
- v0.7.44: Job Object limits + port observation (bonus, small)
- v0.7.45+: revisit daemon if cold-start is still the bottleneck

## Out of scope (deliberate)
- Hardlink-based dep cache (pnpm/uv already do this)
- Shadow building (covered by freshness cache)
- IPC over TCP via UDS/named pipes (no real-world payoff, breaks parity)
- Rewriting nginx config syntax (too sharp; the inferred routes path is enough)
