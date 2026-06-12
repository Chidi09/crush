# Crush GUI data contract

Stable paths and schemas the GUI (v0.8) reads. Pinned as of v0.7.72.
Treat anything in here as a **breaking-change boundary**: changing it
requires bumping a contract version and migrating both ends.

---

## Data directory

```
%LOCALAPPDATA%\Crush\               (Windows)
$XDG_DATA_HOME/crush/               (Linux/macOS, defaults to ~/.local/share/crush)
```

Resolve from Rust via `crush_image::dirs::data_dir()`. Don't hard-code.

---

## State files

### `build-history.json`

Schema: `Vec<crush_build::run::BuildRecord>`. Newest entry pushed at end;
GUI should `.reverse()` for newest-first display. Cap = 200 entries.

Written by `crush` (default run flow) on every build outcome.
Read by:
- `crush history` (text or JSON via `--format json`)
- GUI Build screen

```ts
type BuildRecord = {
  timestamp_ms: number;
  project_path: string;
  project_name: string;
  language: string;       // "python (FastAPI)", "node (Vite)", ...
  framework: string;      // "FastAPI" — parsed from language; may be ""
  duration_ms: number;
  was_cached: boolean;
  size_bytes: number;
  digest: string;         // OCI digest "sha256:..."
  success: boolean;
};
```

### `services/native/<project_name>.json`

Schema: `crush_services::NativeServiceState`. One file per project that
has native deps (postgres / garnet / mysql).

```ts
type NativeServiceState = {
  project: string;
  services: Array<{
    name: string;          // "db", "redis", ...
    pid: number;
    port: number;
    data_dir: string;
    kind: "Postgres" | "RedisCompat" | "MySQL";
  }>;
  started_at: number;     // unix seconds
};
```

Read by:
- `crush services ps` (and `--format json`)
- TUI Compose tab (auto-loads on `crush ps`)
- GUI Services screen

### `containers/<container_id>/container.json`

Schema: `crush_types::Container`. One per container, written by the runtime
on create / status change.

```ts
type Container = {
  id: string;
  name: string;
  image: string;
  status: "Running" | "Stopped" | "Paused" | "Created" | "Creating";
  pid?: number;
  created_at: number;
  started_at?: number;
  ports: Array<[number, number]>;
  mounts: Array<{ source: string; target: string }>;
  memory_limit_bytes?: number;
  cpu_shares?: number;
  // ...health fields
};
```

Read by:
- `crush ps` (and `--format json`)
- TUI Containers tab
- GUI Containers screen

### `containers/<container_id>/stdout.log` and `stderr.log`

Plain UTF-8 line-buffered text. Append-only during container lifetime.
GUI tails these via a polling watch (notify-rs in src-tauri).

---

## CLI commands the GUI can shotgun-call (read-only)

These are safe to invoke from the GUI as raw subprocess calls **only**
for spike/prototype work. The production path is direct crate calls
from `crush-gui/src-tauri` — but the JSON shapes match either way.

```
crush detect --json
crush ps --format json
crush ps --all --format json
crush services ps --format json
crush services ps --format json --all-projects
crush images --format json
crush history --format json --limit 50
```

All emit a single JSON document to stdout. Exit code 0 on success.

---

## Run flow types (defined, not yet implemented)

The GUI's "Run project" button needs to consume a typed event stream.
Types live in `crush_build::run`:

```rust
// crates/crush-build/src/run.rs
pub enum RunEvent { Detected, DepStarted, ImageFresh, ImagePacked,
                    BuildStarted, BuildOutput, BuildFinished, Spawning,
                    AppOutput, PortBound, Exited, Warning, ... }
pub struct RunOptions { dev, rebuild, repack, no_proxy, watch,
                         memory_bytes, cpu_fraction, priority, assume_yes }
```

The matching `run_project(project_root, data_dir, options) -> RunHandle`
function is **not yet implemented** — the body lives inline in
`crush-cli/src/main.rs` Commands::Default (~lines 1488-2710).

Extracting it is the GUI agent's first task. See CRUSH_V8_PLAN.md §
"Required refactor before GUI work begins" for the exact split.

**Until then:** the GUI can spike the run flow by spawning `crush` as a
subprocess and parsing its colour-stripped output. This is throwaway —
delete it as soon as `run_project()` lands.

---

## Anti-fragile rules

1. **Treat every state file as eventually inconsistent.** A container.json
   may reference a pid that's already dead; `services/native/<x>.json`
   may show a service that crashed without cleanup. Always probe the OS
   (kill(pid, 0), TCP connect on port) before reporting "running".

2. **Never write to state files from the GUI.** The CLI owns writes.
   The GUI sends commands (`stop_container`, `stop_native_service`) and
   re-reads after they return.

3. **Polling cadence:** 2s for the lists (containers, services, images),
   1s for stats on the currently-open drawer only. Stop polling on
   window blur.

4. **Atomic updates:** all writes use `write_to <tmp>; rename <tmp> <real>`.
   Reads must tolerate the brief absence of the file during rename.

5. **Schema versioning:** if you need to change a struct shape, add an
   `#[serde(default)]` field instead. Breaking changes require a
   `schema_version` bump on the file and a migration step in `crush-cli`.
