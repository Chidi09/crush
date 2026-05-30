# Plan: v0.8.0 — Crush GUI

> **Status:** plan, not implementation. Read top-to-bottom before touching code.
> **House rules** and **scope cuts** at the end matter more than the screen mocks.

---

## North star

A Tauri 2 app that does what `crush` (CLI) does today, with a real UI for:
- Running the current project (the `crush` no-arg flow)
- Watching containers, native services, images
- Streaming logs with AI diagnosis
- Inspecting build history

**Not** a docker-desktop clone. **Not** a code editor. **Not** a settings panel
(that's 0.9). The GUI is a *visualisation + control* layer on top of the same
crates the CLI uses — zero shelling out to `crush.exe`.

---

## Design system: exact port from crush-web

The GUI uses the same tokens, same fonts, same palette. Zero design
divergence from the website.

Colors (from `styles.css` + `tailwind.config.ts`):
```
background  #09090b   (crush-black)
surface     #1a1a22   (crush-surface)
border      #2a2a35   (crush-border)
text        #e8e8ed   (crush-text)
text-muted  #9a9ab0   (crush-textMuted)
orange      #e05540   (primary accent — everywhere)
orange-lt   #f06a55   (hover states)
green       #4ade80   (running/success)
red         destructive for errors
```

Typography: Geist (sans) + Geist Mono — same stack as the website. Self-host
both as woff2 in `static/fonts/` — no Google Fonts CDN call (offline first).

Component DNA (all inherited directly from `crush-web`):
- Cards: `rounded-2xl border-crush-border/70` · corner orange glow on hover (`0 0 20px rgba(224, 85, 64, 0.06)`)
- Badges/pills: `rounded-full border border-crush-orange/20 bg-crush-orange/5 text-crush-orange uppercase tracking-wider text-xs`
- Monospace panels: `bg-crush-black/90` with macOS `• • •` chrome (red `#ff5f56`, yellow `#ffbd2e`, green `#27c93f`)
- Ambient glow background: blurred `crush-orange/3` circle at `blur-[140px]`
- Selection: `bg-crush-orange/30`
- Scrollbar: thin, `bg-crush-border`, `bg-crush-black` track

---

## Framework: Tauri 2 + SvelteKit 2 + Svelte 5

**Why Svelte 5 (runes) over React for this:**
- Compiles to near-zero JS runtime — small WebView payload, fast paint
- `$state`, `$derived`, `$effect` runes map 1:1 to the signal pattern in your
  Angular components — no mental translation
- Tailwind v4 works identically — copy `tailwind.config.ts` + `styles.css`
  verbatim into the Svelte frontend
- Vite-driven dev with HMR over the Tauri dev server

**Why SvelteKit specifically:** built-in router, file-based routing matches the
screen list, `+layout.svelte` gives us the persistent sidebar shell for free.
Use SSR=`false` everywhere (this is a single-user desktop app, not a website).

**Why Tauri 2 not 1:** v2 has the unified `tauri-plugin-shell`, fixed event
streaming back-pressure, much better Windows installer story.

**Why not Wails / Electron / egui:**
- Electron: 100MB+ download, not acceptable
- Wails (Go): would need to rewrite all our Rust crates' surface — defeats the point
- egui: pixel-perfect parity with crush-web's CSS-rich design is hard; we'd
  have to redesign the design system inside immediate-mode constraints

---

## Cold-start performance budget

| Phase | Budget | Notes |
|---|---|---|
| Window decoration paint | 100ms | Tauri's own |
| First frame of shell (sidebar + empty main) | 300ms | Show skeleton before data |
| First payload of data (containers, services) | 600ms | List existing state, no detection |
| Hydration done | 1s total | Anything later is a regression |

**Concrete:** the sidebar + ambient orange glow + "Loading…" placeholders must
render before the first `invoke()` resolves. Don't block first paint on data.

---

## App structure

```
crates/crush-gui/
├── src-tauri/
│   ├── Cargo.toml             # depends on crush-image, crush-build,
│   │                          # crush-services, crush-ai, NOT crush-cli
│   ├── tauri.conf.json
│   ├── build.rs               # tauri-build
│   ├── icons/                 # 32/128/256/512 .png + .ico from crush-web/public/logo.svg
│   └── src/
│       ├── main.rs            # Tauri::Builder, register all commands
│       ├── state.rs           # AppState { store, services, ai }
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── containers.rs
│       │   ├── services.rs
│       │   ├── images.rs
│       │   ├── run.rs         # the `crush` no-arg flow
│       │   ├── logs.rs        # streaming with channel
│       │   ├── build.rs       # build history
│       │   └── ai.rs          # diagnose_logs
│       ├── events.rs          # typed event payloads + emitter helpers
│       └── platform/
│           ├── mod.rs
│           ├── windows.rs     # SetPriorityClass, file association
│           └── unix.rs        # XDG paths, .desktop file
└── src/                       # SvelteKit frontend
    ├── app.css                # exact copy of crush-web styles.css
    ├── app.html
    ├── lib/
    │   ├── tauri.ts           # typed invoke() + listen() wrappers
    │   ├── components/
    │   │   ├── Sidebar.svelte
    │   │   ├── StatusBadge.svelte
    │   │   ├── TerminalPane.svelte   # macOS chrome from web
    │   │   ├── CpuSparkline.svelte
    │   │   ├── CopyField.svelte
    │   │   ├── Drawer.svelte
    │   │   └── EmptyState.svelte
    │   └── stores/
    │       ├── containers.svelte.ts   # $state-based store
    │       ├── services.svelte.ts
    │       └── images.svelte.ts
    └── routes/
        ├── +layout.svelte             # sidebar + main shell
        ├── +page.svelte               # → redirect to /dashboard
        ├── dashboard/+page.svelte
        ├── containers/+page.svelte
        ├── services/+page.svelte
        ├── images/+page.svelte
        ├── build/+page.svelte
        └── logs/+page.svelte
```

**Top-level `Cargo.toml` workspace member** added: `crates/crush-gui/src-tauri`.

---

## Backend reuse: what calls what

The GUI's `src-tauri/` is a thin wrapper around the existing crates. No
business logic in commands beyond marshalling.

| Tauri command | Backing crate function (already exists or referenced) |
|---|---|
| `list_containers` | walk `data_dir/containers/*/container.json`, see `crush-cli/src/main.rs:3067-3145` (the existing `Commands::Ps` flow) |
| `stop_container` | `crush-runtime-windows`/`-linux` per platform |
| `stream_logs(id)` | tail `data_dir/containers/{id}/stdout.log` + emit events |
| `list_images` | `ImageStore::list_images()` from `crush-image` |
| `pull_image(ref)` | `crush-registry::pull` — wire its progress callback to event emit |
| `remove_image(id)` | `ImageStore::remove_image()` |
| `list_native_services` | `crush-services::load_native_state()` for all projects in `data_dir/services/native/*.json` |
| `stop_native_service(name)` | `crush-services::PostgresDriver::stop` / `RedisCompatDriver::stop` |
| `get_connection_string(name)` | derive from the running service config (already in state.json) |
| `run_default(project_path)` | the entire `Commands::Default` arm in `crush-cli/src/main.rs:1488` — needs extraction (see refactor below) |
| `list_build_history` | `data_dir/build-history.json` (new file we start writing in 0.7.72) |
| `diagnose_logs(lines)` | `crush-ai::AiEngine::diagnose` |

### Required refactor before GUI work begins

The `Commands::Default` flow in `crush-cli/src/main.rs:1488-2710` is one
giant function. It cannot be called from Tauri as-is. **Extract it into
`crush-build/src/run.rs`** as:

```rust
pub struct RunHandle {
    pub events: tokio::sync::mpsc::Receiver<RunEvent>,
    pub abort: tokio::sync::oneshot::Sender<()>,
}

pub enum RunEvent {
    Detected { stack: InferredStack },
    DepStarted { name: String, image: String, native: bool },
    DepFailed { name: String, error: String },
    ImageFresh,
    ImagePacked { digest: String, size_bytes: u64, duration_ms: u64 },
    BuildStarted { command: String },
    BuildOutput { line: String, stream: Stream },   // Stream = Stdout | Stderr
    BuildFinished { duration_ms: u64 },
    Spawning { command: String, port: u16 },
    AppOutput { line: String, stream: Stream },
    PortBound { port: u16, urls: Vec<(String, String)> },  // (label, url)
    Exited { code: i32 },
}

pub async fn run_project(
    project_root: PathBuf,
    options: RunOptions,
) -> Result<RunHandle>;
```

The CLI's `Commands::Default` becomes a thin loop over `RunHandle::events`
that prints them with the current colour formatting. The GUI's `run_default`
command emits the same events as `tauri::Window::emit` calls.

**Why this matters:** without this refactor we'd duplicate ~1200 lines of
detection/spawn/probe logic. Worse, the duplication would silently drift.

This refactor is a v0.7.72 task — do it BEFORE starting GUI work. It's also
useful for daemon mode (deferred Unit 7 of SPEED_PLAN) and `crush --json` for
scripting.

---

## State management (src-tauri)

```rust
// src-tauri/src/state.rs
pub struct AppState {
    pub data_dir: PathBuf,
    pub store: Arc<ImageStore>,
    pub ai: Arc<AiEngine>,
    pub runs: Arc<RwLock<HashMap<RunId, RunHandle>>>,
    pub log_tailers: Arc<RwLock<HashMap<String, LogTailerHandle>>>,
}
```

- `runs`: `crush run_default` invocations the GUI started — keyed by a
  Uuid so the frontend can refer back, abort, or attach output.
- `log_tailers`: per-container `tail -f`-style file watchers — opened on
  first `subscribe_logs(container_id)` call, refcounted, closed when no
  frontend listener remains.

**No mutex on `ImageStore`:** it already has internal locking via the
sqlite-like `Database` field. Same for `BinaryCache`.

**State is `tauri::State<AppState>`** — no `Mutex<AppState>` wrapper. Use
`Arc<RwLock<T>>` per field where mutation is needed.

---

## Event protocol

All events use **kebab-case names** and **typed payloads** serialised via
serde. Frontend defines a matching TypeScript discriminated union in
`src/lib/tauri.ts`.

```
run-event::{run_id}::{kind}   — RunEvent payloads
log-line::{container_id}      — LogLine { ts, stream, text }
pull-progress::{image}        — PullProgress { layer, current_bytes, total_bytes }
service-state-changed         — broadcast when any native service starts/stops
container-state-changed       — broadcast when a container transitions
```

Frontend `listen()` once per page mount, `unlisten()` on unmount via
`onDestroy`. SvelteKit's `$effect.root()` cleanup handles lifecycles.

**Back-pressure:** emit channels are bounded (cap = 1024). When the UI
falls behind, drop oldest log lines and emit a `log-line-dropped` warning
event with a count. Never block the producer (the build / app).

---

## Tauri command surface (signatures)

```rust
// containers
#[tauri::command] async fn list_containers(state: State<'_, AppState>)
    -> Result<Vec<ContainerSummary>, String>;
#[tauri::command] async fn stop_container(id: String, state: State<'_, AppState>)
    -> Result<(), String>;
#[tauri::command] async fn subscribe_logs(
    container_id: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String>;

// images
#[tauri::command] async fn list_images(state: State<'_, AppState>)
    -> Result<Vec<ImageSummary>, String>;
#[tauri::command] async fn pull_image(
    reference: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String>;  // returns image id
#[tauri::command] async fn remove_image(id: String, state: State<'_, AppState>)
    -> Result<(), String>;

// native services
#[tauri::command] async fn list_native_services(state: State<'_, AppState>)
    -> Result<Vec<NativeServiceSummary>, String>;
#[tauri::command] async fn stop_native_service(name: String, project: String, state: State<'_, AppState>)
    -> Result<(), String>;
#[tauri::command] async fn get_connection_string(name: String, project: String, state: State<'_, AppState>)
    -> Result<Option<String>, String>;

// run
#[tauri::command] async fn run_project(
    project_path: String,
    window: Window,
    state: State<'_, AppState>,
) -> Result<String, String>;  // returns run_id (Uuid)
#[tauri::command] async fn abort_run(run_id: String, state: State<'_, AppState>)
    -> Result<(), String>;

// build
#[tauri::command] async fn list_build_history(limit: Option<usize>, state: State<'_, AppState>)
    -> Result<Vec<BuildRecord>, String>;

// ai
#[tauri::command] async fn diagnose_logs(
    lines: Vec<String>,
    state: State<'_, AppState>,
) -> Result<DiagnosisResult, String>;

// platform helpers
#[tauri::command] async fn pick_project_directory(window: Window) -> Result<Option<String>, String>;
#[tauri::command] async fn open_url(url: String) -> Result<(), String>;
#[tauri::command] async fn reveal_in_explorer(path: String) -> Result<(), String>;
```

All errors are stringified at the Tauri boundary — no
`Result<_, anyhow::Error>` because anyhow doesn't serialise cleanly.

---

## Shell layout

```
┌────┬─────────────────────────────────────────────┐
│    │                                             │
│ 48 │                                             │
│ px │           main content area                │
│    │                                             │
│ s  │                                             │
│ i  │                                             │
│ d  │                                             │
│ e  │                                             │
│ b  │                                             │
│ a  │                                             │
│ r  │                                             │
│    │                                             │
└────┴─────────────────────────────────────────────┘
```

Sidebar (48px wide, icon-only, tooltips on hover):
```
  ◆           ← Crush logomark (SVG from public/logo.svg)
  ─
  ⊞  Dashboard
  ▣  Containers
  ⚙  Services      ← v0.7.0 native services
  ◫  Images
  📋  Logs
  🔨  Build
  ─
  ⋯  Settings      ← pinned to bottom (placeholder route in 0.8, fills in 0.9)
```

Active icon: `bg-crush-orange/10 border-l-2 border-crush-orange text-crush-orange`
Inactive: `text-crush-textMuted hover:text-white hover:bg-crush-surface/50`

---

## Screen 1 — Dashboard

```
  crush                                     ● 2 running  [crush ▶]

  ┌─ Current project ────────────────────────────────────────────┐
  │  gazillion-be-staging                                        │
  │  ↳ Python 3.11 · FastAPI                         [Run ▶]    │
  └──────────────────────────────────────────────────────────────┘

  ┌─ Services ───────────────┐  ┌─ Quick stats ───────────────────┐
  │  postgres  ● 5432  2h    │  │  containers running    2        │
  │  garnet    ● 6379  2h    │  │  images cached        14        │
  │  [Stop all]              │  │  disk used           2.1 GB     │
  └──────────────────────────┘  └─────────────────────────────────┘

  ┌─ Recent builds ──────────────────────────────────────────────┐
  │  gazillion-be   0.9s  ● cached   2m ago                      │
  │  my-api         1.2s  ○ fresh    1h ago                      │
  └──────────────────────────────────────────────────────────────┘
```

The **[Run ▶]** button fires `run_project(cwd)` — same flow as `crush` with
no args — and slides up a `TerminalPane` from the bottom (250px) streaming
the run events. The pane has a `[Stop]` button that calls `abort_run(id)`.

**Current project detection:** the GUI doesn't run inside a project. We pick
it from:
1. Last-used project (LocalStorage)
2. `tauri::Manager` cwd
3. Empty state with `[Open project…]` calling `pick_project_directory`.

---

## Screen 2 — Containers

```
  CONTAINERS                                        [+ New]  [Search]

  NAME              IMAGE           STATUS     CPU    MEM       UPTIME
  ────────────────────────────────────────────────────────────────────
  gazillion-be      python:3.11     ● running  █░░░   184 MB   2h 14m [⋯]
  test-nginx        nginx:alpine    ● running  ░░░░    12 MB   3h 02m [⋯]
  old-worker        node:20         ○ exited    —       —      exited  [▸]
```

- Status badge: `● running` → `bg-emerald-500/10 text-emerald-400` (same
  pattern as website's CONTAINER METRICS card)
- CPU sparkline: 20-point mini bar chart, orange fill — driven by polling
  `container_stats(id)` every 1s while the row is visible
- MEM bar: thin `h-1.5 rounded-full` progress bar, same as website card
- `[⋯]` action menu: stop / restart / logs / inspect / copy ID

Clicking a row → right-side drawer slides in (400px):
```
┌─── gazillion-be ─────────────────┐
│  [Logs]  [Inspect]  [Stats]      │
│ ─────────────────────────────── │
│  2026-05-27 14:23:01 INFO ...   │
│  2026-05-27 14:23:02 INFO ...   │
│  ...streaming...                │
└──────────────────────────────────┘
```

Drawer log pane: same `bg-crush-black/90 font-mono text-[11px]` as the
terminal component on the web.

**Polling vs push:** containers update every 2s via a background `setInterval`
that calls `list_containers`. Stats poll every 1s only for the currently-open
drawer. Stop polling when window blurs.

---

## Screen 3 — Services (the v0.7.0 feature, front and centre)

```
  NATIVE SERVICES                            [+ Start service]

  ┌─ postgres ─────────────────────────────────────────────────────┐
  │  ● running · PostgreSQL 17.10 + pgvector 0.8.0                │
  │  Port 5432 · ~/.crush/services/data/gazillion-be/postgres/     │
  │                                                               │
  │  DATABASE_URL                                      [Copy ◻]  │
  │  postgresql://solexpay:solexpay@localhost:5432/gazillion_be   │
  │                                                    [Stop ■]  │
  └────────────────────────────────────────────────────────────────┘

  ┌─ garnet (redis-compat) ────────────────────────────────────────┐
  │  ● running · Garnet 1.1.9                                     │
  │  Port 6379                                                    │
  │                                                               │
  │  REDIS_URL                                         [Copy ◻]  │
  │  redis://localhost:6379                                       │
  │                                                    [Stop ■]  │
  └────────────────────────────────────────────────────────────────┘
```

`[Copy ◻]` writes to clipboard and briefly flashes `✓ copied` in `text-crush-green`.
The connection string field uses `CopyField.svelte` — a code-block styled
input (read-only) with a copy icon.

**Multi-project services:** the list shows services for every project that
has running native services, grouped by project name. Headers per group.

---

## Screen 4 — Images

```
  IMAGES                                   [Pull image...]  [Search]

  ┌────────────────────────────────────────────────────────────────┐
  │  python:3.11-slim        186 MB   3 layers   1d ago   [Delete]│
  │  node:20-alpine           92 MB   4 layers   3h ago   [Delete]│
  │  nginx:alpine             23 MB   2 layers   1d ago   [Delete]│
  └────────────────────────────────────────────────────────────────┘
```

Pull dialog (sheet from right side):
- Input field with `border-crush-border bg-crush-surface` styling
- Per-layer progress: small rows with `h-1 bg-crush-orange` rounded
  progress bars driven by `pull-progress::{image}` events
- Same pattern as the website's progress bars

---

## Screen 5 — Logs

```
  ┌─ containers/services ──┐  ┌─ log output ───────────────────────┐
  │  ● gazillion-be        │  │ [All] [ERROR] [WARN] [INFO] [Search]│
  │  ● postgres            │  │ ──────────────────────────────────  │
  │  ○ old-worker          │  │ 14:23:01 INFO  Uvicorn running...  │
  │                        │  │ 14:23:02 INFO  Application startup │
  │                        │  │ 14:23:45 ERROR Connection refused  │
  │                        │  │                                    │
  │                        │  │              [AI Diagnose ✦]       │
  └────────────────────────┘  └────────────────────────────────────┘
```

`[AI Diagnose ✦]` uses the sparkle SVG from the website's AI diagnosis card.
Clicking:
1. Collects last 200 lines visible in pane
2. Calls `diagnose_logs(lines)`
3. Shows result inline below the log stream:
```
┌─ Crush Diagnostic ─────────────────────────────────────────────┐
│ ✦ Missing database environment variable.                       │
│   Fix: crush secrets set DB_HOST                               │
└────────────────────────────────────────────────────────────────┘
```
Styled exactly like the AI diagnosis mini-card: `border-crush-orange/10
text-crush-orange font-bold` for the header, `text-crush-textMuted` for body.

**Virtualised list:** logs render with `svelte-virtual-list` (or hand-rolled).
At 1000+ lines the naive DOM becomes the bottleneck.

---

## Screen 6 — Build

```
  BUILD HISTORY

  ┌─ gazillion-be · 0.9s · cached ─────────────────────────────────┐
  │  2m ago                                                        │
  │  ████████████████████████████░░  deps (cached)                │
  │  ████  source (fresh)                                         │
  │  Analysis: 0 findings                                         │
  └────────────────────────────────────────────────────────────────┘

  ┌─ my-api · 14.2s · fresh ───────────────────────────────────────┐
  │  1h ago                                                        │
  │  ████████████████████████████████  deps (fresh 12.1s)         │
  │  ██  source                                                    │
  │  Analysis: 2 findings  [View →]                               │
  └────────────────────────────────────────────────────────────────┘
```

Layer bars: orange fill for fresh (`bg-crush-orange`), muted fill for
cached (`bg-crush-border`).

**Where does this data come from?** A new file `data_dir/build-history.json`
that the CLI's run flow appends to on every build outcome. Cap at 200
entries, rolling. Added in v0.7.72 alongside the run-flow refactor.

---

## Window configuration

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Crush",
  "version": "0.8.0",
  "identifier": "run.crush.app",
  "app": {
    "windows": [{
      "title": "Crush",
      "width": 1200,
      "minWidth": 900,
      "height": 760,
      "minHeight": 600,
      "decorations": true,
      "transparent": false,
      "theme": "Dark",
      "center": true
    }],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"],
    "windows": {
      "webviewInstallMode": { "type": "downloadBootstrapper" },
      "nsis": { "displayLanguageSelector": false }
    }
  }
}
```

The `logo.svg` from `crush-web/public/` is used directly for `icons/` —
export at 32 / 128 / 256 / 512 px. `tauri icon` automates this from one PNG.

---

## CI build

`.github/workflows/gui-release.yml`:

```yaml
name: GUI release
on:
  push:
    tags: ['gui-v*']
jobs:
  build:
    runs-on: windows-latest   # mac/linux follow in 0.9
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: pnpm/action-setup@v3
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm, cache-dependency-path: crates/crush-gui/pnpm-lock.yaml }
      - run: cd crates/crush-gui && pnpm install --frozen-lockfile
      - run: cd crates/crush-gui && pnpm tauri build
      - uses: softprops/action-gh-release@v2
        with:
          files: crates/crush-gui/src-tauri/target/release/bundle/nsis/*.exe
```

VPS-build pattern from SPEED_PLAN doesn't apply here — Tauri needs the host's
WebView + bundler, which we don't have on the Linux VPS.

---

## Testing strategy

**What's worth testing:**
- The extracted `run_project` function in `crush-build` — unit tests against
  fixture project dirs (Node, Python, Java) verifying event sequence
- Connection-string formatters in `crush-services` — pure functions
- `commands/images.rs` Tauri commands — mock `ImageStore` trait

**What's NOT worth testing (skip):**
- Svelte components — burns time for low value, design is changing
- Tauri command serialisation — Tauri owns this, framework-level
- Full end-to-end run flows — flaky, slow, and `crush detect` already
  exercises detection
- AI diagnosis output — non-deterministic

No Playwright. No screenshot diffs. The TUI worked without those.

---

## Anti-patterns

| Don't | Why |
|---|---|
| Shell out to `crush.exe` from the GUI | Defeats the point — re-spawns a process, can't stream cleanly, version mismatches |
| Duplicate detection logic in `commands/run.rs` | Use the extracted `run_project` — single source of truth |
| Use Tauri's `Mutex<AppState>` wrapper | `Arc<RwLock<…>>` per field, locks scoped to fields, not the whole state |
| `unsafe_eval` or `unsafe_inline_script` CSP | Inline event handlers from generated SVG can be replaced with class-based handlers |
| Bundle Node.js runtime | Tauri bundles its own WebView; pnpm is dev-only |
| Persist state to multiple files | Use `data_dir/gui-state.json` for UI state, the existing `data_dir/...` for runtime state. Don't sprinkle |
| Add a settings screen | Out of scope for 0.8.0 — link to a docs URL until 0.9 |
| Make the sidebar resizable | Fixed 48px, icons-only — matches the design system |
| Auto-update via Tauri updater | Skip in 0.8.0 — the CLI's existing `crush update` flow will offer GUI updates when present (v0.9) |

---

## Phased delivery

### v0.7.72 — Refactor enabler (week before GUI work)
**Goal:** extract `run_project` so the GUI can call it.

Tasks:
1. Move `Commands::Default` body (`crush-cli/src/main.rs:1488-2710`) into
   `crush-build/src/run.rs` as `run_project(root, opts) -> RunHandle`.
2. Define `RunEvent`, `Stream`, `RunOptions` types in `crush-build/src/run.rs`.
3. Rewrite `Commands::Default` as a 30-line consumer of `RunEvent` that
   prints the same coloured output as today. **Behaviour identical.**
4. Add `data_dir/build-history.json` append on every build outcome.
5. Tag `v0.7.72`. CLI users see no change.

### v0.8.0-alpha — Spike (1 week)
**Goal:** prove the toolchain works end-to-end.

Tasks:
1. `cargo new` the `crush-gui/src-tauri` crate, register in workspace.
2. `pnpm create svelte@latest` in `crates/crush-gui/`, copy `app.css` from
   `crush-web/styles.css`, copy `tailwind.config.ts`.
3. Implement Dashboard screen + `list_containers` + `list_native_services`.
4. Window opens, sidebar renders, dashboard shows real data.
5. **No build, no run, no logs yet** — just lists.

### v0.8.0-beta — Run + logs (1 week)
1. Implement Containers screen with drawer + log stream
2. Implement Services screen with copy fields + stop buttons
3. Implement Dashboard `[Run ▶]` → `TerminalPane` streaming `RunEvent`s

### v0.8.0 — Ship (1 week)
1. Implement Images screen + pull dialog
2. Implement Build history screen
3. Implement Logs screen with AI diagnose button
4. NSIS installer wiring, icon export, signing (skip code signing for 0.8.0 — Windows SmartScreen will warn but it works)

---

## What NOT in scope for v0.8.0

- Settings screen (v0.9.0)
- Volume manager screen (v0.9.0)
- Compose-file editor (v0.9.0 maybe; might just stay CLI-only)
- Mobile (not applicable — the GUI is a desktop tool)
- Light mode (brand is dark-only, matches website)
- Auto-updater UI (v0.9.0 — Tauri updater plugin exists, wire it then)
- macOS / Linux builds (v0.9.0 — Windows-first since that's where the pain is)
- Multi-window (single window, single project at a time)
- Localisation (English only)
- Code signing (causes SmartScreen warnings; deferred until we have a signing cert)
- TUI parity for `crush ps` / `crush stats` — the TUI stays, GUI doesn't replace it. They serve different workflows.

---

## Shipped artifact

`crush-gui-0.8.0-windows-x64-setup.exe` — NSIS installer, ~25 MB. Produced by
`tauri build` in CI on `windows-latest`. App installs to `%LOCALAPPDATA%\Crush\`
and adds a Start Menu shortcut. Includes WebView2 downloader bootstrapper for
machines without Edge WebView2 installed (~99.5% have it on Win10/11 already).

`crush update` (the CLI command) gains a `--gui` flag in v0.7.73 that fetches
the GUI installer separately and runs it elevated.

---

## Open questions to settle BEFORE writing code

1. **Single binary or two?** — CLI ships as one .exe today. Do we ship the GUI
   as a separate .exe (current plan) or bundle the CLI inside the GUI? Plan
   says separate; reconsider if disk-size matters less than install simplicity.

2. **Daemon-or-not?** — does the GUI launch a background `crushd` process to
   share state with the CLI, or does each tool re-read `data_dir/` files?
   Current plan: re-read files (no daemon). Trade-off: GUI doesn't see live
   stats from CLI runs unless we poll the filesystem. For 0.8.0 this is fine
   — most state is filesystem-backed (container state, services state).

3. **AI quota** — the GUI's `[AI Diagnose ✦]` button is a one-click trigger.
   Easy to spam. Throttle to one diagnosis per 30s per container? Or trust
   the user. Default: trust, no throttle, add only if abuse appears.

4. **GUI version sync with CLI** — does GUI 0.8.0 require CLI 0.7.72+? Yes,
   because of the refactor. The GUI installer checks and prompts to install
   the CLI if missing. CLI installer is unchanged.
