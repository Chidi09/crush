# Crush — Reliability & Docker-Parity Plan

> **Mission frame.** Crush is *native-first, Docker-backwards-compatible*: the goal is to
> run a **full-scale Docker experience on Windows/macOS/Linux without running Docker
> locally**. Every item below is judged against that frame. "Over-engineering" is fine
> when it buys determinism, safety, or parity — but where the naïve suggestion is
> actually *fragile*, this plan specifies the **correct** design instead of the literal one.
>
> Workflow: an implementing agent builds these phases; the maintainer validates + ships.
> Each task lists **Goal / Design / Anchors (real files & symbols) / API surface /
> Acceptance / Effort**. Anchors were read from the current tree — keep them honest.

Legend — Effort: ▁ small (≤½ day) · ▃ medium (1–2 days) · ▅ large (3–5 days) · ▇ epic (1wk+).
Priority: **P0** kills a known bug class · **P1** core to the vision · **P2** valuable · **P3** polish.

---

## Phase R1 — Run-loop resilience
The dev inner loop (`crates/crush-build/src/run.rs`). This is where users live; fragility here is felt daily.

### R1.1 — EADDRINUSE port takeover  · P0 · ▃
**Goal.** A bound port must never just fail the run. Identify the holder and offer
*Kill & takeover* or *Use next free port*.

**Design.**
- Add a preflight in the spawn path: before launching an app/service, probe the target
  port. If bound, resolve the owning PID + process name cross-platform:
  - Linux: parse `/proc/net/tcp` + `/proc/net/tcp6` (hex local-addr/port → inode) →
    walk `/proc/*/fd` symlinks matching `socket:[inode]`.
  - Windows: `GetExtendedTcpTable(AF_INET, TCP_TABLE_OWNER_PID_ALL)` via `windows` crate
    (already a dep for the Job Object code at `run.rs:239`).
  - macOS: `lsof -nP -iTCP:<port> -sTCP:LISTEN -t` fallback (no stable syscall).
- Emit a new event and let the surface decide (auto in CLI with a flag, interactive in GUI).

**Anchors.** `run.rs` spawn region (after `Spawning` at line 77, before `PortBound` at 95).
New helper module `crates/crush-build/src/portcheck.rs`. The Windows path reuses the
`windows` crate already imported for `JobObjectExtendedLimitInformation` (run.rs:239).

**API surface.** New `RunEvent` variants (enum at `run.rs:28`):
```rust
PortConflict { port: u16, pid: u32, process: String },   // emitted on detect
PortReassigned { from: u16, to: u16 },                    // when "use next free" chosen
```
New `RunOptions` field: `on_port_conflict: PortConflictPolicy { Prompt, Kill, Reassign, Fail }`.
GUI: a modal in the run view; CLI: `--port-conflict=kill|reassign|fail` (default `prompt`
in GUI, `fail` in non-interactive CLI).

**Acceptance.** Unit test the `/proc/net/tcp` hex parser against a known fixture; integration
test that occupying a port then running with `Reassign` binds the next free port and emits
`PortReassigned`. Manual: occupy :3000 with `python -m http.server 3000`, run a Next app,
confirm prompt + takeover.

### R1.2 — Wire RestartManager into the dev loop  · P1 · ▃
**Goal.** PM2-grade resilience: a dev server that crashes restarts automatically, *visibly*,
with a cap — without masking real errors.

**Correct design (not the naïve one).** Auto-restart that silently hides a crash loop is
worse than failing. So:
- Default dev (`RunOptions.dev == true`) uses `RestartPolicy::OnFailure { max_retries: 5 }`
  unless the user passes `--restart`.
- On non-zero exit: emit `Restarting { reason, attempt, delay_ms }`, sleep
  `RestartManager::backoff_delay()`, relaunch. On clean exit (code 0) or explicit Stop,
  do **not** restart (respect `explicitly_stopped`).
- **Crash-loop guard:** if `max_retries` is hit within a short window, stop and emit a
  terminal `Warning { message: "crashed N times in Ms — stopping; see logs" }` so the loop
  surfaces rather than spins forever.

**Anchors.** `RestartManager` is already complete (`crates/crush-reliability/src/restart.rs`:
`new`, `should_restart`, `backoff_delay`, `record_attempt`, `reset`). It is only wired to the
CLI `--restart` flag today (`crush-cli/src/main.rs:2456`). Wrap the child-wait in `run.rs`
(the `Exited { code }` emission at line 105) with the manager.

**API surface.** New `RunEvent::Restarting { reason: String, attempt: u32, delay_ms: u64 }`.
GUI shows a non-alarming "restarting…" chip with the attempt counter.

**Acceptance.** Test: a script that exits 1 three times then 0 restarts exactly 3× with
growing backoff, then settles. Test: explicit Stop during backoff cancels the restart.

### R1.3 — Lockfile-hash DepsFresh (replace mtime heuristic)  · P0 · ▁
**Goal.** Kill the "reinstalls every run" / "didn't reinstall after pull" bug class for good.

**Why it's broken now.** Freshness is decided by mtime comparison —
`node_deps_fresh()` (`run.rs:604`) and the per-stack checks (`run.rs:521-566`) compare
`node_modules`/`.venv`/`target` mtime ≥ lockfile mtime. `git pull`/`checkout`/`touch`
reorders mtimes and lies in both directions.

**Design.** Content-hash the lockfile and persist it.
- On successful install, write `.crush/deps-state.json`:
  `{ "<lockfile-relpath>": { "sha256": "...", "installed_at": <unix> } }`.
- Freshness = `sha256(current lockfile) == stored sha256` **and** deps dir exists.
- Keep the mtime check only as a cheap fast-path *negative* (if dir missing → not fresh);
  never as a positive freshness signal.
- Covers: `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `bun.lockb`, `poetry.lock`,
  `requirements.txt`, `Cargo.lock`, `go.sum`, `pom.xml`/`build.gradle*`.

**Anchors.** Replace bodies of `node_deps_fresh` (`run.rs:604`) and the freshness arms at
`run.rs:521-566`; add `crates/crush-build/src/depstate.rs`. Emits the existing
`RunEvent::DepsFresh` (`run.rs:118`) — no new event needed.

**Acceptance.** Test: install → freshness true; mutate one byte of the lockfile → false;
`touch -d future node_modules` → still false (mtime can't fake it). Manual: `git pull` that
changes a lock triggers reinstall; a no-op run does not.

### R1.4 — Resource-exhaustion sentinels  · P2 · ▃
**Goal.** Warn when a spawned dev process leaks (e.g. Vite balloons to multiple GB).

**Correct design (debounced, not noisy).** A naïve "you're using 4GB" toast is noise.
Instead:
- Sample RSS/CPU of the spawned tree every ~5s (reuse the Job Object on Windows for
  accurate tree accounting; `/proc/<pid>/statm` on Linux; `proc_pidinfo`/`ps` on macOS).
- Fire **once** per threshold crossing with hysteresis: warn at >X% of system RAM
  (default 60%) *and* sustained for ≥2 samples; re-arm only after it drops below X−10%.
- `OomMonitor` already exists (`crates/crush-reliability/src/oom.rs`: `new`, `poll`,
  `OomEvent`) — generalize it from container-only to a host-process sampler.

**API surface.** `RunEvent::ResourceWarn { service: String, rss_bytes: u64, pct_ram: u8 }`.
Action button in GUI → restart that service (reuses R1.2 restart path).

**Acceptance.** Test the hysteresis state machine (cross up → one event; stay high → no
repeat; drop + re-cross → new event). Threshold configurable, default sane.

### R1.5 — Crash diagnosis (assist, not auto-patch)  · P2 · ▃
**Goal.** When a dev process dies, surface a probable cause + fix fast.

**Correct design.** Auto-patching code from parsed stderr is a foot-gun — keep the human in
the loop.
- On non-zero exit, capture the trailing stderr ring buffer (already streamed as `Stderr`,
  `run.rs:21`), run lightweight local pattern matchers first (port in use → point at R1.1;
  `MODULE_NOT_FOUND` → suggest install; `EACCES`; Prisma `.prisma/client` missing →
  `prisma generate`; Python `ModuleNotFoundError`). These are deterministic, no AI needed.
- If no rule matches and the user opts in, send the trace to the existing AI copilot
  (`crush-ai`) for a suggestion. Present as a **suggestion with a diff preview**; "Apply"
  writes the patch only on explicit click and always leaves it in the working tree (never
  commits).

**Anchors.** Reuse the `Stderr` stream + `crush-ai` crate (copilot). New
`commands/diagnose.rs` in the GUI; rule table in `crates/crush-build/src/diagnose.rs`.

**Acceptance.** Each built-in rule has a fixture stderr → expected suggestion. AI path is
behind an explicit toggle and never edits files without confirmation.

---

## Phase R2 — Deterministic environments (the heart of "Docker without Docker")
A container's value is a *pinned* environment. To match it natively, Crush must control the
toolchain — this is **core to the vision**, not over-engineering.

### R2.1 — Toolchain manager: auto-fetch exact runtimes  · P1 · ▇
**Goal.** A project pinned to Node 20.10.0 / Python 3.11.x / Java 21 runs on that exact
runtime even if the host has something else — zero "wrong version" drift, no global mutation.

**Design.** New crate `crates/crush-toolchain`.
- **Detect** (already half-done): `detect.rs` already reads version files into
  `hints.runtime_version` (`detect.rs:739,748`). Extend to `.nvmrc`, `.node-version`,
  `package.json#engines.node`, `.python-version`, `.tool-versions` (asdf/mise), `.sdkmanrc`,
  `go.mod` toolchain line, `rust-toolchain.toml`.
- **Resolve + download** to a content-addressed cache `~/.crush/runtimes/<lang>/<version>/`:
  - Node: official `nodejs.org/dist/vX/node-vX-<os>-<arch>.{tar.gz,zip}` + checksum verify.
  - Python: `python-build-standalone` (indygreg) prebuilt, or shell out to `uv python install`
    when present (don't reinvent — `uv` is the better way and is fast/static).
  - Java: Adoptium Temurin API.
  - Go: `go.dev/dl`.
- **Activate** by prepending the resolved `bin/` to the spawned process `PATH` (env-scoped to
  that run only — never touch global state, never write shims to the user's shell).
- Verify integrity via published SHA256; cache is immutable once written.
- **Offline:** if the exact version isn't cached and there's no network (see R7.3), fall back
  to host runtime with a loud `Warning` naming the mismatch — never hang on a download.

**Anchors.** `detect.rs` (`runtime_version` plumbing at lines 18, 521, 739). New
`crush-toolchain` crate consumed by `run.rs` right before `Spawning` (line 77) to compute the
augmented `PATH`. CLI `crush toolchain ls|install|which`.

**API surface.** `RunEvent::ToolchainResolved { lang: String, version: String, cached: bool }`
and `ToolchainFetching { lang, version, pct: u8 }` for download progress.

**Acceptance.** With host Node 18 and an `.nvmrc` of 20.x, the spawned process reports
`node -v` = 20.x. Checksum mismatch aborts the install. Second run is cache-hit (no download).
Offline + uncached → warning + host fallback, no hang.

**Why this over a literal "download a runtime" hack:** content-addressed cache + checksum +
run-scoped PATH gives container-grade reproducibility without the global-mutation mess of
nvm/pyenv. Leaning on `uv` for Python is less code and more correct than a bespoke builder.

### R2.2 — `crush doctor`  · P1 · ▃
**Goal.** One command that proves the host can actually build/run the project — the biggest
native-vs-Docker liability.

**Design.** A check registry, each = `{ name, probe() -> Status, fix() -> Option<Action> }`:
- Toolchain present & matches pin (delegates to R2.1).
- Native build tools for common native deps: C/C++ toolchain (`cc`/MSVC Build Tools),
  `make`, Python headers (for `node-gyp`), `pkg-config`, libpq (for `pg`/`pgvector`).
- Lockfile ↔ manifest sync (R2.3).
- Port availability for declared services; disk headroom; required CLIs (git, the package
  manager the lockfile implies).
- Output a table (CLI) / panel (GUI) with ✓/✗ and a one-click/`--fix` remediation
  (`xcode-select --install`, winget MSVC Build Tools, `apt-get install build-essential`,
  `pnpm install`, etc.). Fixes are explicit, never silent.

**Anchors.** New `crates/crush-doctor` + `commands/doctor.rs` (GUI) + `crush doctor` (CLI).

**Acceptance.** On a box missing a C compiler, `crush doctor` flags it and `--fix` installs it
(platform-appropriate). Re-run shows green.

### R2.3 — Lockfile ↔ manifest drift  · P2 · ▁
**Goal.** Catch `package.json` changed but lockfile stale (and vice-versa) before a confusing
build failure.

**Design.** Cheap structural check folded into R1.3 + R2.2: parse manifest deps, confirm each
resolves in the lockfile and versions satisfy ranges. On drift: warn + offer auto-sync
(`<pm> install --lockfile-only` / `pnpm install`). Runs as a run-preflight and inside doctor.

**Anchors.** `crush-doctor` shared with R1.3 `depstate.rs`. Emits `RunEvent::Warning`.

**Acceptance.** Add a dep to `package.json` without updating the lock → drift warning with a
working auto-sync.

---

## Phase R3 — Production parity (Docker-backwards-compatible)
This is Pillar 2 of the vision: prove "what runs locally runs in the container" — **natively**.

### R3.1 — `crush run --simulate-prod`  · P1 · ▅
**Goal.** Run the *real* production topology — the OCI image + its service graph — to prove
parity, **without requiring a Docker daemon**.

**Correct design (native execution, not "shell out to docker").**
- Build the real OCI image (already supported — image build + export are working;
  see image-export findings). 
- Execute it via Crush's **native OCI runtime** (`crates/crush-runtime`) rather than
  `docker run`. If the runtime can't yet honor a needed feature, fall back to
  `docker compose up` *only when a Docker engine is detected at the edge* — interop, not a
  dependency.
- Read the ejected `docker-compose.yml` (we already generate it via `crush eject`) as a
  **dependency graph**: bring up backing services (Postgres/Redis/etc.) using the native
  service drivers (`crush-services`) keyed by `native_driver_for(image)`, wire env/links,
  then start the app image, then point the L7 gateway (`crush-build/src/gateway.rs`) at it.
- Health-gate with the existing checker before declaring "prod-parity up".

**Anchors.** `crush-runtime` (native OCI exec), `crush-services` (`native_driver_for`,
`ServiceDriver`), `crush eject` (compose generation), `gateway.rs:run_l7_gateway` (L7 route).
New `crates/crush-build/src/simulate.rs` orchestrating the compose-graph run. CLI flag on
`crush run`.

**API surface.** `RunEvent::SimulateProd { phase: "building"|"services"|"app"|"healthy" }`.

**Acceptance.** A sample app with Postgres dependency: `crush run --simulate-prod` builds the
image, starts native Postgres, runs the image against it natively, gateway serves it, health
check passes — with **no Docker daemon running**. Document any feature that still needs the
Docker fallback.

### R3.2 — Config-drift detection on eject  · P2 · ▃
**Goal.** Flag when native state and the ejected compose diverge (e.g. a Redis dep added
natively but the compose wasn't re-ejected).

**Design.** On `crush eject` (and as a doctor check), diff the *detected* service set
(`detect.rs` deps + running native services) against services declared in the existing
`docker-compose.yml`. Report added/removed/changed (image tag, ports, env keys). Offer
"re-eject".

**Anchors.** `crush eject` codepath + `detect.rs`. Emits a structured diff to GUI.

**Acceptance.** Add a native Redis dep, re-run eject check → drift reported with a one-click
re-eject that produces a compose containing redis.

### R3.3 — Auto-snapshot before migrations  · P1 · ▁
**Goal.** A corrupting migration is always one click from undo.

**Design.** Cheap, because the snapshot engine exists (`crush db snapshot/restore`,
task #13). Wrap migration invocations: when a run's command matches
`prisma migrate (dev|deploy|reset)`, `drizzle-kit push|migrate`, `flyway migrate`,
`alembic upgrade`, `rails db:migrate`, `knex migrate`, `sequelize db:migrate`, take
`crush db snapshot --tag auto-pre-migrate-<ts>` against the target DB first. Surface a
one-click restore in the GUI DB panel. Retain last N auto snapshots (prune older).

**Anchors.** Command detection in `run.rs` (or a wrapper in `crush-cli`); snapshot engine
in `crush-cli/commands/db.rs`. GUI: list auto snapshots in `routes/database`.

**Acceptance.** Running a Prisma migrate creates a tagged snapshot first; a deliberately
bad migration is recoverable via one-click restore.

---

## Phase R4 — Gateway / TLS hardening
`crates/crush-build/src/gateway.rs` (real ACME already implemented: `CertResolver`,
`acme_worker`, self-signed for local).

### R4.1 — ACME circuit breaker  · P1 · ▃
**Goal.** Never get the user's IP rate-limit-banned by Let's Encrypt when DNS isn't pointed.

**Design.** Per-domain failure state persisted to `certs_dir/acme-state.json`:
`{ host: { failures: u32, next_attempt_at: unix } }`. On failure, exponential cooldown
(e.g. 15m → 1h → 6h → 24h cap); reset on success. The `acme_worker` (`gateway.rs:299`,
currently a flat hourly loop at line 324) consults `next_attempt_at` and skips domains in
cooldown. Pre-flight: before ordering, do a cheap DNS A/AAAA resolve + optional self-reach
check; if the domain doesn't resolve to us, don't even spend an ACME order.

**Anchors.** `acme_worker` (`gateway.rs:299-324`), `obtain_acme_cert`. Self-signed already
serves meanwhile, so cooldown never breaks TLS.

**Acceptance.** Point a domain at nothing → after 3 failures the worker backs off (assert via
the persisted state + log), self-signed keeps serving, no tight retry loop.

### R4.2 — Trusted local CA (mkcert-style)  · P2 · ▃
**Goal.** Zero-warning HTTPS for `*.local`/dev domains.

**Design.** We already mint per-domain self-signed leafs (so TLS doesn't fail). Add a stable
**local root CA**: generate once (`rcgen`) at `~/.crush/ca/`, sign local leafs from it, and
install the root into the OS trust store on demand (`crush trust install` →
`security add-trusted-cert` macOS / `certutil -addstore Root` Windows / update-ca-certificates
+ NSS for Linux/Firefox). Browser then trusts local domains without warnings.

**Anchors.** `gateway.rs` self-signed path (`self_signed_pem`, `certified_key_from_pem`,
`is_public_fqdn`). New `crates/crush-build/src/localca.rs` + `crush trust install|uninstall`.

**Acceptance.** After `crush trust install`, `https://app.crush.local` loads with a valid
lock in Chrome/Safari. Uninstall cleanly removes it.

### R4.3 — Graceful connection draining  · P3 · ▁
**Goal / correct framing.** The naïve suggestion ("spawn a new listener and drain on every
domain add") is **unnecessary** — the gateway already hot-swaps domain→port and certs behind
`Arc<RwLock>` under a single long-lived listener (`gateway.rs:106,114,220`; domains watcher at
421-430), so adding a domain never drops connections. The only real gap is **draining on
gateway shutdown/restart**: stop accepting, let in-flight requests finish (bounded timeout),
then exit. Implement that and nothing more.

**Acceptance.** Add a domain under active load → zero dropped connections (already true; add a
test). On shutdown, in-flight requests complete within the drain window.

---

## Phase R5 — DB studio safety
`crates/crush-gui/src-tauri/src/commands/database.rs`.

### R5.1 — Default LIMIT + streaming for grid views  · P0 · ▃
**Goal.** `SELECT *` on a huge table must never OOM the backend/GUI.

**Why it's broken now.** `query_postgres` (`database.rs:194`) runs `client.simple_query(sql)`
which buffers all rows; no cap. MySQL path (`query_mysql:257`) same risk; Mongo already
`.limit(limit)` (`database.rs:474`).
**Design.**
- Separate two modes: **grid/browse** (auto-capped) vs **raw SQL editor** (user owns it, but
  warn on unbounded `SELECT`). For browse, wrap as `SELECT * FROM (<user sql>) _crush LIMIT
  1001` and detect the 1001st row to show "more rows exist". For very large reads, use a
  server-side cursor (`DECLARE _crush CURSOR FOR ... ; FETCH 1000`) over the extended-query
  protocol instead of `simple_query`, streaming chunks to the frontend.
- The grid sends `{ page, page_size }`; default page_size 1000, hard max configurable.

**Anchors.** `query_postgres` (`database.rs:194`), `query_mysql` (257), `db_run_query` (321).
Add a `db_browse_table` command distinct from `db_run_query`.

**Acceptance.** Browsing a 10M-row table returns ≤1001 rows and a "more" indicator; memory
flat. Raw editor still runs arbitrary SQL but warns on unbounded selects.

### R5.2 — Transactional dry-run for destructive statements  · P2 · ▃
**Goal.** Before `DELETE`/`UPDATE`/`DROP`/`TRUNCATE` without a clear guard, show exact impact.

**Design.** Parse the statement type. For `DELETE`/`UPDATE` on Postgres/MySQL, run inside
`BEGIN; <stmt>; ` then read affected row count (Postgres: `<stmt> RETURNING *` count or
`GET DIAGNOSTICS`; simplest portable: `SELECT count(*) FROM (...)` of the predicate), then
`ROLLBACK;` — never committing the probe. Prompt "This will affect exactly N rows. Proceed?"
and only then run for real. `DROP`/`TRUNCATE` get a typed-confirmation modal (type the table
name).

**Anchors.** `db_run_query` (`database.rs:321`) gains a pre-exec analysis step; new
`db_estimate_impact` command. Reuse the existing destructive-confirm UX hook.

**Acceptance.** `DELETE FROM users WHERE ...` reports the precise count before committing;
canceling rolls back with zero changes; the probe never mutates data.

### R5.3 — Connection pruning — **N/A, documented**
Not applicable: `database.rs` opens a connection **per call** and drops it
(`tokio_postgres::connect` at line 195; redis multiplexed conn per call at 354). There is no
persistent pool to leak, so there are no orphaned connections to prune. *If* a pool is
introduced later for perf, add idle-timeout eviction then — not before.

---

## Phase R6 — Storage robustness
`crates/crush-gui/src-tauri/src/commands/storage.rs` (currently single `put_object`,
`storage_upload_object:225`).

### R6.1 — Multipart upload for large objects + abort-on-failure  · P2 · ▃
**Goal.** Reliably upload large files (single PUT caps at 5GB on S3 and buffers memory).

**Design.** Above a threshold (default 64MB), use `create_multipart_upload` →
`upload_part` (8–16MB parts, bounded concurrency) → `complete_multipart_upload`. On any
fatal error or cancel, **immediately** `abort_multipart_upload` so no partial data lingers
(cost protection). Stream parts from disk; never buffer the whole file.

**Anchors.** `storage_upload_object` (`storage.rs:225`) and `storage_upload_bytes` (250) gain
a multipart branch. `make_s3_client` (65) already builds the client.

**Acceptance.** A 1GB upload to MinIO/R2 completes via multipart with flat memory; killing it
mid-flight triggers an abort (verify no in-progress upload remains via `list_multipart_uploads`).

### R6.2 — Resume-on-wake  · P3 (stretch) · ▃
**Gated on R6.1.** Persist `{ upload_id, bucket, key, parts:[{n, etag}] }` to
`~/.crush/s3-state/<hash>.json` after each completed part. On reconnect, `list_parts` and
resume from the first missing part instead of restarting. Only build once users actually hit
large cross-network uploads — for local MinIO this rarely matters.

**Acceptance.** Drop the network mid-upload; on resume only the missing parts transfer.

### R6.3 — Zombie multipart sweeper  · P3 · ▁
**Gated on R6.1.** A periodic background task: `list_multipart_uploads`, `abort` any
initiated by Crush older than 24h. Cheap insurance against surprise R2/S3 bills.

**Acceptance.** A stale upload older than the cutoff is aborted on the next sweep.

---

## Phase R7 — UX & resilience polish

### R7.1 — Cmd/Ctrl+K command palette  · P2 · ▃
**Goal.** Jump anywhere: a project, a container's logs, a DB table in studio, run/deploy.
**Design.** A global Svelte overlay (`crush-gui`) indexing nav routes + dynamic entities
(projects, services, buckets, tables) with fuzzy match; actions dispatch existing Tauri
commands. Keybind registered app-wide. **Anchors.** New `lib/CommandPalette.svelte` +
`Sidebar.svelte` route registry; entity providers call existing list commands.
**Acceptance.** Ctrl+K → type a table name → opens DB studio focused on it; type "deploy" →
runs the deploy action.

### R7.2 — Dependency DAG visualizer  · P3 · ▃
**Goal.** Live graph of detected services (Frontend → API → Postgres/Redis); edge flashes red
on unhealthy. **Design.** Build the graph from `detect.rs` deps + running-service health;
render with a lightweight SVG/force layout (no heavy dep). Poll health from existing status
commands. **Acceptance.** Stopping Postgres turns its node + edges red within a poll cycle.

### R7.3 — Offline guard  · P2 · ▁
**Goal / correct framing.** Not a "serve from cache" rebuild (we're already local) — just
**don't hang on the network when offline**. Add a fast reachability probe (TCP connect to a
known host w/ ~1s timeout, cached briefly). When offline: skip update checks, skip cloudflared
tunnel init with a clear message, and make R2.1 fall back to host runtime instead of waiting
on a download timeout. **Anchors.** Update-check path, `tunnel.rs` init, `crush-toolchain`.
**Acceptance.** With networking disabled, cold start has no multi-second DNS stalls; tunnel
init returns a clear "offline" instead of hanging.

---

## Suggested sequencing (by leverage, not by phase number)
1. **R1.3 lockfile-hash** + **R5.1 grid LIMIT** + **R1.1 port takeover** — three P0s that each
   patch a *real, visible* failure in the current code. Small/medium, ship first.
2. **R3.3 auto-snapshot** + **R2.2 `crush doctor`** + **R4.1 ACME breaker** — high safety per
   line of code; snapshot engine already exists.
3. **R1.2 restart** + **R5.2 dry-run** + **R6.1 multipart** + **R7.3 offline guard** — core
   robustness.
4. **R2.1 toolchain manager** + **R3.1 simulate-prod** — the two epics that *define* the
   "Docker experience without Docker" promise. Biggest scope; do them deliberately.
5. **R4.2 local CA**, **R7.1 Cmd+K**, **R1.4 sentinels**, **R1.5 diagnosis**, **R3.2 drift**,
   **R7.2 DAG**, **R6.2/6.3** — polish & stretch.

## Cross-cutting requirements (apply to every task)
- **Validate the Windows cross-compile** (`cargo check --target x86_64-pc-windows-gnu`) for any
  task touching `#[cfg(windows)]` (R1.1, R1.4, R2.1, R4.2) — the 1.0.3 `sysinfo` regression
  came from host-only validation.
- New `RunEvent` variants are append-only; update the GUI `RunEvent` handler + `tauri.ts`.
- Each task ships with unit tests for its pure logic (parsers, state machines, hashing,
  backoff) and a manual acceptance note.
- Match the existing design language (Svelte 5 runes, `Icon.svelte`/`TechIcon.svelte`,
  existing card/panel styles). No stubs, no `todo!()`, no `panic!("not implemented")`.
