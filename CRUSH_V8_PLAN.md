 Plan: v0.8.0 — Crush GUI

  Design system: exact port from crush-web

  The GUI uses the same tokens, same fonts, same palette. Zero design
  divergence from the website.

  Colors (from styles.css + tailwind.config.ts):
  background  #09090b   (crush-black)
  surface     #1a1a22   (crush-surface)
  border      #2a2a35   (crush-border)
  text        #e8e8ed   (crush-text)
  text-muted  #9a9ab0   (crush-textMuted)
  orange      #e05540   (primary accent — everywhere)
  orange-lt   #f06a55   (hover states)
  green       #4ade80   (running/success)
  red         destructive for errors

  Typography: Geist (sans) + Geist Mono — same stack as the website.

  Component DNA (all inherited directly):
  - Cards: rounded-2xl · border-crush-border/70 · corner orange glow on
  hover (0 0 20px rgba(224, 85, 64, 0.06))
  - Badges/pills: rounded-full border border-crush-orange/20
  bg-crush-orange/5 text-crush-orange uppercase tracking-wider text-xs
  - Monospace panels: bg-crush-black/90 with macOS • • • chrome (red
  #ff5f56, yellow #ffbd2e, green #27c93f)
  - Ambient glow background: blurred crush-orange/3 circle at
  blur-[140px]
  - Selection: bg-crush-orange/30
  - Scrollbar: thin, bg-crush-border, bg-crush-black track

  ---
  Framework: Tauri v2 + Svelte 5

  Why Svelte over React for this:
  - Svelte compiles to zero-runtime JS — smallest possible WebView
  payload
  - Signal-based reactivity (Svelte 5 runes) maps directly to the
  signal() pattern already used in your Angular components
  - Tailwind works identically — copy your tailwind.config.ts and
  styles.css verbatim into the Svelte frontend

  ---
  App structure

  crates/crush-gui/
  ├── src-tauri/
  │   ├── Cargo.toml        # depends on crush-image, crush-build,
  crush-services
  │   ├── tauri.conf.json
  │   └── src/
  │       ├── main.rs
  │       ├── state.rs       # Arc<ImageStore>, Arc<NativeServiceState>
  │       └── commands.rs    # Tauri commands wrapping crush crates
  └── src/                   # Svelte 5 frontend
      ├── app.css            # exact copy of crush-web styles.css
      ├── lib/
      │   ├── components/
      │   │   ├── Sidebar.svelte
      │   │   ├── StatusBadge.svelte
      │   │   ├── TerminalPane.svelte   # reuse macOS chrome pattern from
   web
      │   │   ├── CpuSparkline.svelte
      │   │   └── CopyField.svelte
      │   └── stores/
      │       ├── containers.ts
      │       ├── services.ts
      │       └── images.ts
      └── routes/
          ├── +layout.svelte   # sidebar + main shell
          ├── dashboard/
          ├── containers/
          ├── services/
          ├── images/
          ├── build/
          ├── logs/
          └── settings/

  ---
  Shell layout

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

  Sidebar (48px wide, icon-only, tooltips on hover):
    ◆           ← Crush logomark (SVG from public/logo.svg)
    ─
    ⊞  Dashboard
    ▣  Containers
    ⚙  Services      ← v0.7.0 native services
    ◫  Images
    📋  Logs
    🔨  Build
    ─
    ⋯  Settings      ← pinned to bottom

  Active icon: bg-crush-orange/10 border-l-2 border-crush-orange
  text-crush-orange
  Inactive: text-crush-textMuted hover:text-white
  hover:bg-crush-surface/50

  ---
  Screen 1 — Dashboard

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

  The [Run ▶] button fires the same flow as crush (no args) — stack
  detect → build → prompt → run — with the output streamed into an inline
   TerminalPane component (same macOS chrome as the website's terminal
  component).

  ---
  Screen 2 — Containers

    CONTAINERS                                        [+ New]  [Search]

    NAME              IMAGE           STATUS     CPU    MEM       UPTIME
    ────────────────────────────────────────────────────────────────────
    gazillion-be      python:3.11     ● running  █░░░   184 MB   2h 14m
  [⋯]
    test-nginx        nginx:alpine    ● running  ░░░░    12 MB   3h 02m
  [⋯]
    old-worker        node:20         ○ exited    —       —      exited
  [▸]


  - Status badge: ● running → bg-emerald-500/10 text-emerald-400 (same
  pattern as website's CONTAINER METRICS card)
  - CPU sparkline: 20-point mini bar chart, orange fill
  - MEM bar: thin h-1.5 rounded-full progress bar, same as website card
  - [⋯] action menu: stop / restart / logs / inspect / copy ID

  Clicking a row → right-side drawer slides in (400px):
  ┌─── gazillion-be ─────────────────┐
  │  [Logs]  [Inspect]  [Stats]      │
  │ ─────────────────────────────── │
  │  2026-05-27 14:23:01 INFO ...   │
  │  2026-05-27 14:23:02 INFO ...   │
  │  ...streaming...                │
  └──────────────────────────────────┘

  Drawer log pane: same bg-crush-black/90 font-mono text-[11px] as the
  terminal component on the web.

  ---
  Screen 3 — Services (the v0.7.0 feature, front and centre)

    NATIVE SERVICES                            [+ Start service]

    ┌─ postgres ─────────────────────────────────────────────────────┐
    │  ● running · PostgreSQL 16.3 + pgvector 0.7.4                 │
    │  Port 5432 · ~/.crush/services/data/gazillion-be/postgres/     │
    │                                                               │
    │  DATABASE_URL                                      [Copy ◻]  │
    │  postgresql://postgres:crush@localhost:5432/gazillion-be      │
    │                                                    [Stop ■]  │
    └────────────────────────────────────────────────────────────────┘

    ┌─ garnet (redis-compat) ────────────────────────────────────────┐
    │  ● running · Garnet 1.0.4                                     │
    │  Port 6379                                                    │
    │                                                               │
    │  REDIS_URL                                         [Copy ◻]  │
    │  redis://localhost:6379                                       │
    │                                                    [Stop ■]  │
    └────────────────────────────────────────────────────────────────┘

  [Copy ◻] writes to clipboard and briefly flashes ✓ copied in
  text-crush-green.

  The connection string field uses CopyField.svelte — a code-block styled
   input (read-only) with a copy icon, identical in style to the
  website's code-block component class.

  ---
  Screen 4 — Images

    IMAGES                                   [Pull image...]  [Search]

    ┌────────────────────────────────────────────────────────────────┐
    │  python:3.11-slim        186 MB   3 layers   1d ago   [Delete]│
    │  node:20-alpine           92 MB   4 layers   3h ago   [Delete]│
    │  nginx:alpine             23 MB   2 layers   1d ago   [Delete]│
    └────────────────────────────────────────────────────────────────┘

  Pull dialog (sheet from right side):
  - Input field with border-crush-border bg-crush-surface styling
  - Per-layer progress: small rows with h-1 bg-crush-orange rounded
  progress bars
  - Same pattern as the website's progress bars

  ---
  Screen 5 — Logs

    ┌─ containers/services ──┐  ┌─ log output ───────────────────────┐
    │  ● gazillion-be        │  │ [All] [ERROR] [WARN] [INFO] [Search]│
    │  ● postgres            │  │ ──────────────────────────────────  │
    │  ○ old-worker          │  │ 14:23:01 INFO  Uvicorn running...  │
    │                        │  │ 14:23:02 INFO  Application startup │
    │                        │  │ 14:23:45 ERROR Connection refused  │
    │                        │  │                                    │
    │                        │  │              [AI Diagnose ✦]       │
    └────────────────────────┘  └────────────────────────────────────┘

  [AI Diagnose ✦] button uses the same sparkle SVG icon pattern from the
  website's AI diagnosis feature card. Clicking it:
  1. Sends last N error lines to crush-ai
  2. Shows result in an inline card below the log stream:
    ┌─ Crush Diagnostic ─────────────────────────────────────────────┐
    │ ✦ Missing database environment variable.                       │
    │   Fix: crush secrets set DB_HOST                               │
    └────────────────────────────────────────────────────────────────┘
  Styled exactly like the AI diagnosis mini-card on the website's feature
   section: border-crush-orange/10 text-crush-orange font-bold for the
  header, text-crush-textMuted for body.

  ---
  Screen 6 — Build

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

  Layer bars: orange fill for fresh (bg-crush-orange), muted fill for
  cached (bg-crush-border).

  ---
  Tauri command surface (commands.rs)

  // Container ops
  #[tauri::command] async fn list_containers(state) ->
  Vec<ContainerSummary>
  #[tauri::command] async fn stop_container(id: String, state) ->
  Result<()>
  #[tauri::command] async fn stream_logs(id: String, window, state)  //
  uses window.emit("log-line")

  // Image ops
  #[tauri::command] async fn list_images(state) -> Vec<ImageSummary>
  #[tauri::command] async fn pull_image(reference: String, window, state)
   // window.emit("pull-progress")
  #[tauri::command] async fn remove_image(id: String, state) ->
  Result<()>

  // Native services (v0.7.0)
  #[tauri::command] async fn list_native_services(state) ->
  Vec<RunningService>
  #[tauri::command] async fn stop_native_service(name: String, state) ->
  Result<()>
  #[tauri::command] async fn get_connection_string(name: String, state)
  -> Option<String>

  // Build
  #[tauri::command] async fn run_default(window, state)  // streams
  detect → build → run via window.emit()
  #[tauri::command] async fn list_build_history(state) ->
  Vec<BuildRecord>

  // AI
  #[tauri::command] async fn diagnose_logs(lines: Vec<String>, state) ->
  String

  ---
  Window configuration (tauri.conf.json)

  {
    "app": {
      "windows": [{
        "title": "Crush",
        "width": 1200,
        "minWidth": 900,
        "height": 760,
        "minHeight": 600,
        "decorations": true,
        "transparent": false,
        "theme": "Dark"
      }]
    },
    "bundle": {
      "identifier": "run.crush.app",
      "icon": ["icons/icon.png", "icons/icon.ico"],
      "windows": { "webviewInstallMode": { "type": "downloadBootstrapper"
   } }
    }
  }

  The logo.svg from crush-web/public/ is used directly for icons/ —
  export at 32/128/256/512px.

  ---
  What NOT in scope for v0.8.0

  - Settings screen (v0.9.0)
  - Volume manager screen (v0.9.0)
  - Mobile (not applicable)
  - Light mode (brand is dark-only, matches website)
  - Auto-updater UI (v0.9.0 — Tauri updater plugin exists, wire it then)

  ---
  Shipped artifact

  crush-gui-0.8.0-windows-x64-setup.exe — NSIS installer, ~25MB. Produced
   by tauri build in CI on windows-latest. App installs to
  %LOCALAPPDATA%\crush-gui\ and adds a Start Menu shortcut.