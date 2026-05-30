# Speed/parity plan — v0.7.42 → v0.7.48

Goal: close the remaining gap between `crush` and `docker compose up` on
both *perceived speed* and *UX surface area*. Seven units. Ship in order.

This document is the handoff brief — every unit specifies exact files,
commands, code patterns to follow, and explicit anti-patterns so a fresh
agent can land each PR without reverse-engineering the codebase.

---

## Conventions (read this first — applies to every unit)

### House rules
- **Never** add `Co-Authored-By: Claude` (or any Claude trailer) to commits. House rule.
- Bump `workspace.version` in the root `Cargo.toml` (single source of truth — all crates inherit).
- One unit = one PR = one commit, conventional message format (`feat(scope): …`, `fix(scope): …`).
- Don't push until the build passes on the VPS (see "VPS build" below).
- Don't open a PR — push to `main`. This repo ships directly to release.

### VPS build (don't skip)
The Windows binary is cross-compiled on a Linux VPS reachable via `ssh safe-meet`. Pattern:

```bash
ssh safe-meet "cd ~/crush && git checkout Cargo.lock 2>&1 | tail -1 && git pull --ff-only 2>&1 | tail -3 && source ~/.cargo/env && cargo build --release --target x86_64-pc-windows-gnu -p crush-cli 2>&1 | tail -30"
```

Note: piping cargo through `tail` masks its exit code. **Always read the
last ~30 lines** and confirm you see `Finished \`release\` profile` — do
not trust the wrapper's exit code.

### Release pattern
```bash
ssh safe-meet "cp ~/crush/target/x86_64-pc-windows-gnu/release/crush-cli.exe ~/crush/crush-VERSION-windows-x86_64.exe && cd ~/crush && gh release create vVERSION crush-VERSION-windows-x86_64.exe --title 'vVERSION' --notes 'one-line summary'"
```

If the build failed but you reached `gh release create`, **delete the
release** before re-doing: `git push --delete origin vVERSION`.

### Anti-patterns (apply everywhere)
- Do NOT add `bytes` / `futures` / `walkdir` etc. unless a unit explicitly says to. They already exist or are not needed.
- Do NOT write tests unless asked. The shop ships and validates by hand.
- Do NOT refactor adjacent code "while you're there." One unit = one change.
- Do NOT replace the existing `cmd /c` Windows shell wrapper. It handles `.cmd`/`.bat` PATH shims correctly.
- Do NOT touch `crates/crush-services/src/postgres.rs` lightly — it has Windows-specific behavior around system PostgreSQL.
- Do NOT add features behind cargo feature flags. We ship one binary.
- Do NOT add `Co-Authored-By` (yes, repeating it — agents always forget).

### Where things live
| Concern | File |
|---|---|
| Top-level CLI dispatch + run path | `crates/crush-cli/src/main.rs` |
| Per-stack detection | `crates/crush-build/src/detect.rs` |
| Multi-service detection | `crates/crush-build/src/multiservice.rs` |
| Compose parser + dep classifier | `crates/crush-build/src/service_orchestrator.rs` |
| Dep service drivers (postgres, garnet) | `crates/crush-services/src/` |
| Job Object | `crates/crush-cli/src/job_object.rs` |
| Reverse proxy | `crates/crush-cli/src/proxy.rs` |
| Image build pipeline | `crates/crush-build/src/{pipeline,stages,cache}.rs` |

---

## Unit 1 — Skip image-build on warm runs (v0.7.42)

### Goal
Right now `crush` always prints `✓ crushed to image safe_meet:latest (3.5s · 13 MB)` even when source is unchanged and the dep layer is cached. Kill the entire image-build path on warm runs and go straight to the "run it now?" prompt.

### Files
- `crates/crush-cli/src/main.rs` (single touch around line ~1567 where `engine.execute_layered_build` is called)
- `crates/crush-build/src/cache.rs` (add a small "content fingerprint" helper if one doesn't already exist)

### Algorithm
1. Compute a **project content hash**: SHA-256 over sorted `(path, mtime_ns)` tuples for every tracked source file (the same exclusion list as `latest_mtime()` in main.rs:998 — skip `node_modules`, `target`, `.next`, etc.).
2. Look up the previous hash from `<data_dir>/cache/last-image/<project_name>.hash`.
3. If they match AND a cached `outcome.digest` is on disk, **skip `execute_layered_build` entirely**. Print: `✓ image fresh — skipping pack (--repack to force)`.
4. If they don't match, run the build as today and write the new hash.

### Code shape
```rust
let project_hash = crush_build::project_fingerprint(&project_root)?;
let hash_path = data_dir.join("cache").join("last-image").join(format!("{project_name}.hash"));
let prev_hash = std::fs::read_to_string(&hash_path).ok();

let outcome = if prev_hash.as_deref() == Some(&project_hash) && !cli.repack {
    println!("   {} image fresh — {} {}",
        "✓".green().bold(),
        "skipping pack".dimmed(),
        "(--repack to force)".dimmed());
    crush_build::BuildOutcome { was_cached: true, digest: prev_hash.unwrap(), size_bytes: 0, duration_ms: 0 }
} else {
    let o = engine.execute_layered_build(&project_root, &stack).await?;
    let _ = std::fs::create_dir_all(hash_path.parent().unwrap());
    let _ = std::fs::write(&hash_path, &project_hash);
    o
};
```

### CLI surface
Add `#[arg(long, help = "Force re-packing the image even if sources unchanged")] repack: bool` to `Cli` (around line 65).

### Anti-patterns
- Do NOT git-diff-based hashing. Many projects have uncommitted work.
- Do NOT hash file contents. Mtimes are enough and ~100x faster.
- Do NOT touch `BuildPipeline` / `BuildEngine` internals. Skip them, don't refactor.
- Do NOT cache by image digest — the digest is the *output* of the build, so it can't gate the build itself.

### Done when
- Warm `crush` in safe-meet skips image-build step (no `crushed to image` line, or a faster `image fresh` line).
- Editing any `.ts`/`.go`/`.py` triggers rebuild.
- `crush --repack` always rebuilds.

### Commit message
`feat(cache): skip image pack on warm runs via content fingerprint`

---

## Unit 2 — Parallel multi-service builds (v0.7.43)

### Goal
Today multi-service spawn loop runs each sub-service's build serially. Run them in parallel, capped at `min(num_cpus, 4)` concurrent.

### Files
- `crates/crush-cli/src/main.rs` (multi-service block, around line 1567–1690)

### Algorithm
1. Collect `(sub, build_cmd)` pairs for every sub-service that needs to build.
2. Spawn each build as a `tokio::task` guarded by a `tokio::sync::Semaphore::new(min(num_cpus, 4))`.
3. `futures::future::join_all` (already in deps now). If any task errors, kill the rest and bail.
4. Per-service output already has `[name]` prefix coloring — interleaving is acceptable.

### Code shape
```rust
use tokio::sync::Semaphore;
let sem = Arc::new(Semaphore::new(std::thread::available_parallelism().map(|p| p.get().min(4)).unwrap_or(2)));

let mut handles = Vec::new();
for sub in &stack.services {
    if let Some(build_cmd) = compute_build_cmd(sub) {  // existing freshness-check logic
        let sem = sem.clone();
        let sub = sub.clone();
        let sub_path = PathBuf::from(&sub.path);
        let dep_env = dep_env.clone();
        let filter = filter.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            run_build(&sub, &build_cmd, &sub_path, &dep_env, &filter).await
        }));
    }
}
let results = futures::future::join_all(handles).await;
if results.iter().any(|r| matches!(r, Ok(Err(_)) | Err(_))) {
    anyhow::bail!("one or more sub-service builds failed");
}
```

### Anti-patterns
- Do NOT model inter-service dependency order. Turbo/nx/pnpm handle that *inside* each sub-service's own build command. Our job is only to run sub-services concurrently, not their internal stages.
- Do NOT remove the `[name]` line prefixes. Interleaved output is the whole point of having them.
- Do NOT make the semaphore size configurable yet. Hardcode `min(cpus, 4)`. Add a flag only if someone asks.
- Do NOT use `rayon` — we're in tokio land.

### Done when
- NCIC (go backend + vite frontend) builds in `max(backend, frontend)` time, not sum.
- safe-meet (single root service, even if it's turbo internally) is unchanged.
- If backend fails, frontend gets killed and the error surfaces cleanly.

### Commit message
`feat(build): build sub-services in parallel with semaphore-capped concurrency`

---

## Unit 3 — Parallel dep starts + background binary fetch (v0.7.44)

### Goal
Today postgres and redis start sequentially in the compose section (main.rs:1394). Start them concurrently. Separately: when we need to download Garnet on first run, kick the download off the moment we *detect* we need it (during compose parse), not after detection finishes.

### Files
- `crates/crush-cli/src/main.rs` (compose section, line ~1394)
- `crates/crush-services/src/binary_cache.rs` (maybe add a "prefetch hint" if not already exposed)

### Algorithm

**Part A: parallel starts**
```rust
let dep_futures: Vec<_> = parsed.dep_services.iter()
    .map(|dep| {
        let dep = dep.clone();
        let pname = project_name.clone();
        let dd = data_dir.clone();
        async move {
            let res = start_dep_service_smart(&dep, &pname, &dd).await;
            (dep, res)
        }
    })
    .collect();
let results = futures::future::join_all(dep_futures).await;
for (dep, res) in results {
    // same print logic as today, just batched after the join
}
```

Per-service "starting … ok" lines arrive out of order — that's fine. Keep the existing line format.

**Part B: background prefetch**
At the very top of `Commands::Default` (after detection but before compose parsing), kick off a `tokio::spawn` that pre-warms the Garnet binary cache if `redis_compat` is on the dep image list. By the time `start_dep_service_smart` runs, the binary is already on disk.

```rust
tokio::spawn(async move {
    // Best-effort prefetch; ignore failure. The real start path
    // will redownload on a miss.
    let _ = crush_services::redis_compat::prefetch(cache_dir).await;
});
```

Add a `pub async fn prefetch(cache_dir: PathBuf) -> Result<()>` to `redis_compat.rs` that calls into BinaryCache without spawning the server.

### Anti-patterns
- Do NOT parallelize *across* compose files (we only ever have one).
- Do NOT prefetch postgres — it uses the system install on Windows, no binary to fetch.
- Do NOT block startup on prefetch. It's fire-and-forget.

### Done when
- safe-meet: postgres + redis "starting … ok" lines appear within ~50ms of each other instead of serially.
- First-run cold redis prefetch is observable in `<data_dir>/cache/binaries/garnet/` before crush hits the dep start phase.

### Commit message
`feat(deps): start dep services in parallel + background-prefetch garnet`

---

## Unit 4 — `crush ps` / `logs` / `stop` / `restart` (v0.7.45)

### Goal
Match the day-to-day docker-compose verbs. Surface what's running, tail its logs, stop one or all, restart one.

### Files
- `crates/crush-cli/src/main.rs` (add subcommands + handlers)
- `crates/crush-services/src/state.rs` (or wherever `save_service_state` / `save_native_state` live — already exists)

### State that already exists
`ServiceState` and `NativeServiceState` (see `save_service_state` / `save_native_state` in main.rs around line 1428/1440). Read those for `ps`.

### Commands

**`crush ps`** — list running deps:
```
NAME       KIND      PORT     UPTIME    PID
postgres   native    5432     1h 12m    18342
redis      garnet    6379     1h 12m    18987
```
Read `<data_dir>/services/<project>.state.json` and `native.state.json`. Show PID, ports from the saved state. Compute uptime from `started_at`. For "still running?" check: on Windows use `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)` + `GetExitCodeProcess`; on Linux send `kill -0 pid`. Drop entries where the process is gone (cleanup as a side effect).

**`crush logs <name> [-f]`** — tail a service's log file.
Currently we don't centrally capture dep service logs. Two options:
- (A) Postgres writes to its data dir's `pg_log/`. Garnet writes to wherever we launched it from. Tail those files.
- (B) Change `start_dep_service_smart` to redirect stdout/stderr to `<data_dir>/services/logs/<project>/<name>.log`.
- **Pick (B).** Simpler, uniform across deps. Apply only to *new* spawns (existing running ones predate this — `crush logs` won't have history for them; document that).

**`crush stop [<name>]`** — stop one or all deps.
- No arg → stop everything in the project's saved state, then delete the state file.
- With arg → stop that one, remove from state, save.
- Implementation: for native services, `kill` the PID (on Windows: `TerminateProcess` via `windows-sys`; we already have it). For containers, shell out to the detected backend (`docker stop <cname>` etc.) — code probably already exists, grep for `running_containers`.

**`crush restart [<name>]`** — `stop` then re-run detection + start for that service.
Skip implementing this for now if it gets messy. Defer to next unit.

### Subcommand shape
```rust
#[command(about = "Show running services for this project")]
Ps,
#[command(about = "Tail logs for a running service")]
Logs { name: String, #[arg(short, long)] follow: bool },
#[command(about = "Stop one or all running services")]
Stop { name: Option<String> },
```

### Anti-patterns
- Do NOT introduce a daemon for this. The state files are enough.
- Do NOT delete log files on `stop`. Keep them for post-mortem.
- Do NOT cap log file size. Add log rotation later if it's actually a problem.
- Do NOT change the format of existing state JSON. Add fields; don't rename.

### Done when
- `crush ps` after starting safe-meet shows postgres + redis with uptime.
- `crush logs postgres` prints the latest 50 lines.
- `crush logs postgres -f` follows.
- `crush stop` kills both, removes state, second `crush ps` shows nothing.
- `crush stop postgres` stops just postgres.

### Commit message
`feat(cli): add ps / logs / stop subcommands for service lifecycle`

---

## Unit 5 — Job Object resource limits (v0.7.46)

### Goal
Add `--memory <bytes>` and `--cpus <fraction>` flags. Wire them into the existing Windows Job Object so the app and all its descendants die if they exceed the limit. Catches OOMs and runaway CPU locally instead of in prod.

### Files
- `crates/crush-cli/src/job_object.rs`
- `crates/crush-cli/src/main.rs` (add flags + pass into `init()`)

### Code shape

```rust
// job_object.rs
pub struct Limits { pub memory_bytes: Option<u64>, pub cpu_percent: Option<u8> }

pub fn init_with_limits(limits: Limits) { /* … */ }

// inside imp::create() set the new fields on JOBOBJECT_EXTENDED_LIMIT_INFORMATION:
// JOB_OBJECT_LIMIT_PROCESS_MEMORY  | JOB_OBJECT_LIMIT_JOB_MEMORY
// info.ProcessMemoryLimit = memory_bytes
// info.JobMemoryLimit = memory_bytes

// For CPU, separate call:
// SetInformationJobObject(h, JobObjectCpuRateControlInformation, &cpu_info, ...)
// cpu_info.ControlFlags = JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP
// cpu_info.Anonymous.CpuRate = percent * 100  (1% = 100)
```

### CLI shape
```rust
#[arg(long, value_parser = parse_size, help = "Memory cap (e.g. 4G, 512M). Process tree dies on exceed.")]
memory: Option<u64>,
#[arg(long, value_parser = parse_cpu_fraction, help = "CPU cap (e.g. 0.5, 2). 1.0 = one core.")]
cpus: Option<f32>,
```

Parser helpers: `parse_size("4G")` → bytes; `parse_cpu_fraction("0.5")` → percent (50). Both reject invalid.

### Anti-patterns
- Do NOT add Linux equivalents in this unit. Cgroups need a separate design pass.
- Do NOT swallow "limit exceeded" silently — log a clear message in the Ready panel: `↳ memory cap: 4 GB (process tree killed on exceed)`.
- Do NOT default to any limit. Off unless user specifies.

### Done when
- `crush --memory 256M` in a project that allocates 300MB during build kills the whole tree.
- `crush --cpus 0.5` keeps total CPU under 50% on a single-core measurement.
- No flags = current behavior, no overhead.

### Commit message
`feat(limits): --memory and --cpus flags via Job Object resource controls`

---

## Unit 6 — Watch mode (`crush --watch`) (v0.7.47)

### Goal
Re-run the affected sub-service when sources change. Like `docker compose watch`, but smarter: don't restart unrelated services.

### Files
- `crates/crush-cli/src/main.rs` (new code path under `--watch`)
- `crates/crush-cli/Cargo.toml` (add `notify = "6"` — battle-tested file watcher)

### Algorithm
1. After successful Ready panel, if `cli.watch`, set up a `notify` recommended watcher rooted at `project_root`.
2. Debounce events (200ms collapse window).
3. On change burst, figure out which sub-service the changed paths belong to (walk up to the nearest `package.json`/`go.mod`/`pom.xml` — already detected in `stack.services[].path`).
4. Kill that sub-service's `tokio::process::Child`, re-run its build (with the freshness check from Unit 1 to skip unnecessary work), respawn.
5. Other sub-services untouched.

### Patterns to ignore in watcher
Reuse the same skip list from Unit 1 / `latest_mtime`:
- `node_modules`, `target`, `.next`, `dist`, `build`, `.turbo`, `.venv`, `__pycache__`, `.git`, `.cache`, `out`, `bin`, `_build`, `.gradle`, `.mvn`, `vendor`, `deps`

### Anti-patterns
- Do NOT use a polling watcher even on Windows. `notify` defaults to inotify/ReadDirectoryChangesW — keep it.
- Do NOT restart on every single keystroke save. 200ms debounce minimum.
- Do NOT restart on changes to files in the skip list. Especially `node_modules` (yarn/npm churns it constantly).
- Do NOT try to do hot-module-reload yourself. We're orchestrating processes, not bundling.
- Do NOT extend watch mode to deps (postgres, redis). Sources only.

### Done when
- `crush --watch` in NCIC, edit a `.go` file → only the Go service restarts, vite stays up.
- Edit a `.tsx` file → only vite restarts, Go stays up.
- Save in `node_modules` (e.g. via pnpm install) → no restart.
- `Ctrl+C` cleans up everything via the existing Job Object path.

### Commit message
`feat(watch): --watch reruns affected sub-service on source change`

---

## Unit 7 — `crushd` daemon (v0.7.48 — bigger lift)

### Goal
Make `crush` itself instant by moving the warm state into a long-lived background daemon. `crush ps`, `crush stop`, etc. become thin clients that IPC to `crushd` over a named pipe / unix socket.

### Don't start this unit until 1–6 have been used in anger for a week.
If the cold-start time after 1+2+3 lands feels fine, skip this entirely. Talk to the user before committing.

### High-level
- `crates/crush-daemon/` new crate.
- Binary `crushd` started on demand (or via a tray app, future).
- Named pipe `\\.\pipe\crush` on Windows, `~/.crush/sock` on Unix.
- Protocol: line-delimited JSON, `{ "cmd": "ps" }` → `{ "result": [...] }`.
- `crush` CLI checks if daemon is up; if yes, send over pipe; if no, fall back to current in-process logic. **Backwards-compatible from day one.**

### Things to keep in the daemon (warm state)
- Image content-fingerprint cache (so `crush` doesn't re-walk the filesystem)
- Running dep service state (replaces the current state-files-on-disk)
- Compose parse cache keyed by `(path, mtime)`
- Stack detection cache keyed by `(project_root, mtime of key files)`

### Anti-patterns
- Do NOT make the daemon required. Always-graceful-fallback.
- Do NOT use gRPC. JSON over a named pipe is enough.
- Do NOT autostart on system boot. Lazy-start on first `crush` invocation.
- Do NOT proxy log streams through the daemon — they get big fast. Daemon hands the CLI a file path; CLI tails directly.
- Do NOT design a plugin API. We don't need one yet.

### Done when
- `crush ps` after a fresh terminal takes <50ms.
- Killing `crushd` (Task Manager) doesn't break `crush` — it just gets slower again.
- `crushd --version` prints the same version as `crush --version`.

### Commit message
`feat(daemon): introduce crushd for warm state and sub-50ms ps`

---

## What we are explicitly NOT doing

These ideas came up during planning and were rejected. Don't add them
back without a conversation:

| Idea | Why not |
|---|---|
| Hardlink-based dep cache (pnpm-style global store) | pnpm/uv/cargo/go-mod already do this. Reinventing it = a worse pnpm. |
| Shadow building (background prebuild) | Freshness cache (already shipped) covers the actual win. The background CPU drain is real, the payoff isn't. |
| UDS / named-pipe IPC for app-to-db | Loopback TCP is microseconds; drivers don't transparently support UDS. Breaks parity instead of improving it. |
| Strict env scrubbing | Many Windows tools need PATH/TEMP/USERPROFILE or they crash. The right answer is the env-diff lint (deferred). |
| Reading `infra/nginx.conf` to seed proxy routes | The inferred routes already work for monorepos. Add only if a user with a non-standard layout asks. |
| Per-stack feature flags | One binary, simple shipping. |
| Switching to gRPC for anything | We don't have a multi-language consumer. JSON-over-pipe is fine. |
| Test suites for the new code | This shop validates by hand. Don't write tests unless asked. |

---

## Suggested sequencing

If picking off one at a time:
- **Today**: Unit 1 (huge perceived speed win, ~2 hours)
- **Tomorrow**: Units 2 + 3 (parallel everything, ~half day combined)
- **Day 3**: Unit 4 (ps/logs/stop, ~3 hours)
- **Day 4**: Unit 5 (resource limits, ~1 hour)
- **Week 2**: Unit 6 (watch mode, ~half day; spend time on the path-to-service mapping)
- **Maybe never**: Unit 7 (daemon, only if 1–6 isn't enough)

Each unit is independently mergeable. Don't queue them.
