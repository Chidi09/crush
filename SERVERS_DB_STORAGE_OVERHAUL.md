# Servers · Database · Storage — UI Overhaul Plan

> **Goal.** Not just feature-parity with Dokploy / Supabase / Cloudflare-R2 — **match their
> craft.** What makes those products feel premium isn't the feature list, it's the *finish*:
> motion, density, real data-viz, skeleton loading, optimistic updates, a command palette,
> hover-cards, and a hundred micro-interactions. This plan front-loads a **design system** and a
> deep **component library**, then applies them across all five management surfaces —
> **Servers, Database, Storage, Native Services, and the Dashboard.**
>
> The differentiator stays: these surfaces manage the servers you SSH into and the DBs/buckets
> Crush runs — so we get *live, correlated* moments no read-only console can (a request pulsing a
> DB row, a deploy animating across a timeline). Those are the showpieces.

Legend — Effort: ▁ ≤½d · ▃ 1–2d · ▅ 3–5d · ▇ 1wk+. Priority: **P0** foundation · **P1** headline ·
**P2** strong · **P3** stretch. Anchors read from the current tree.

Current state (measured): `servers/+page.svelte` 101 LOC (launcher), `servers/[alias]` 275 LOC
(static health + flat lists), `database/+page.svelte` 3363 LOC (tabs + FK detect + cell-edit + RLS),
`storage/+page.svelte` 1314 LOC (S3 browser). Backends: `servers.rs` 446, `database.rs` 882,
`storage.rs` 544.

---

## Phase V — Visual system & design language (the foundation everything inherits) · P0

### V1 — Design tokens · ▃
A single tokens layer so every surface is consistent and themeable.
- **Color:** semantic surfaces (`bg`, `surface`, `surface-raised`, `overlay`), borders
  (`border`, `border-strong`), text (`text`, `muted`, `faint`), the crush orange accent
  (`rgba(224,85,64,…)`), a **status palette** (success/warn/error/info/neutral) and a dedicated
  **8-hue data-viz palette** (charts must not reuse status colors).
- **Elevation:** 4-step shadow scale + a subtle inner-glow for raised cards (the Supabase/Linear
  "lifted panel" feel).
- **Radius / spacing:** an 8-pt spacing scale and a radius scale (`sm/md/lg/pill`).
- **Typography:** a type scale (display→caption) with the mono family reserved for data/IDs/SQL.
- **Motion tokens:** durations (`fast 120ms`, `base 200ms`, `slow 320ms`) + easings
  (`standard`, `emphasized`, `spring`) so animation is consistent, not ad-hoc.
**Anchors.** Extend the existing `var(--color-crush-*)` set in the global stylesheet.
**Acceptance.** Every new component reads from tokens; swapping a token re-themes globally.

### V2 — Theming + density + reduced-motion · ▃
Dark (default) **and** light theme; a **density toggle** (comfortable / compact — power users
managing 200 rows want compact); honor `prefers-reduced-motion`. Persist choice.
**Acceptance.** Toggle theme/density live; reduced-motion disables non-essential animation.

### V3 — Motion & interaction principles · ▁ (doc + helpers)
- Enter/exit with Svelte `transition:`/`crossfade` (rows, panels, toasts) — content *arrives*,
  doesn't pop. Layout shifts animate (FLIP) where it reads as continuity.
- Hover/active states on everything interactive (lift, border-glow, icon nudge).
- **Skeletons, not spinners** for first paint; **optimistic UI** for edits (apply instantly, roll
  back on error with a toast).
**Acceptance.** No bare spinners on initial load; edits feel instant.

---

## Phase 0 — Component library (build once, compose everywhere) · P0

**Data-viz**
- **0.1 Sparkline** ▁ — inline SVG line.
- **0.2 AreaChart** ▃ — gradient-filled time-series with hover tooltip + crosshair (host CPU/RAM,
  query latency). The "real chart" feel, still dependency-free SVG.
- **0.3 DonutGauge / RadialMeter** ▁ — disk/RAM as a ring with animated sweep.
- **0.4 UptimeBars** ▁ — statuspage-style 90-segment health history bar.
- **0.5 Heatmap** ▃ — activity/load heatmap (requests over time, table write volume).

**Surfaces & structure**
- **0.6 DataGrid** ▅ — the workhorse: virtualized rows, sticky + **frozen first column**, column
  sort/resize/reorder, **range cell selection + copy (TSV/CSV/JSON)**, inline edit slots, zebra,
  focus ring, pagination footer. Caller injects cell renderers (FK chips, type editors).
- **0.7 Drawer/Sheet, Modal, ContextMenu (right-click), Tooltip, HoverCard** ▃ — HoverCard is key:
  hovering an FK shows a mini preview of the referenced row.
- **0.8 CommandPalette (⌘/Ctrl-K)** ▃ — jump to any server/app/table/bucket, run actions
  (deploy, new query, reset sandbox) without the mouse. (Aligns with the API-studio/Cmd-K idea.)
- **0.9 ToastHost, Skeleton, ProgressRing, StatusPill (pulsing), Badge, SegmentedControl, Tabs,
  Breadcrumb, Timeline, Identicon/Avatar, EmptyState (illustrated)** ▃.

**Content viewers**
- **0.10 JsonTree** ▃ — collapsible JSON/JSONB viewer + editor (DB cells, container inspect,
  bucket policy).
- **0.11 CodeBlock** ▁ — syntax-highlit SQL/JSON/shell (lightweight highlighter, themed).
- **0.12 LogStream** ▃ — virtualized live log view with severity coloring, search/filter,
  wrap toggle, follow-tail, pause — used by servers + runs.
- **0.13 DiffView** ▃ — side-by-side/inline diff (schema diff, env diff, branch API diff).
- **0.14 TerminalPane** ▃ — `xterm` + fit addon, themed to our palette, bound to a PTY stream.

**Architecture primitives (the ones Svelte gives you nothing for — build deliberately)**
- **0.15 OverlayStack + FocusManager** ▅ — a global z-ordered overlay registry that Modal /
  Drawer / ContextMenu / Tooltip / HoverCard / CommandPalette all register into. Provides
  **layered Escape** (Esc closes only the top-most layer), **per-layer focus trap + focus
  restoration** to the triggering node, scroll-lock, and the **hover-card safe-triangle**
  (pointer-trajectory check so a card stays open when the cursor heads toward it). **This is the
  single highest-value primitive — half the premium feel and most of the keyboard correctness hang
  off it, and unlike the rest, the framework gives us nothing here.**
- **0.16 ContextBus** ▁ — a tiny "where am I" store (active table / server / bucket) so ⌘K and
  context menus can mutate their placeholder + actions to the current location.

> This library *is* the visual quality. Each page below is mostly composition + the page-specific
> showpiece.

---

## Phase S — Servers (Dokploy parity, with finish)

### S1 — Live, beautiful resource telemetry · P1 · ▃
Host header = **DonutGauges** for CPU/RAM/disk + **AreaCharts** with gradient fill streaming on a
~3s sample; **UptimeBars** for reachability history. Per-container CPU/mem as mini sparklines in
each card. Numbers count-up animate; thresholds tint green→amber→red; values update with a soft
pulse, not a flicker.
**Backend gap:** add `cpu_pct` to `ServerHealth` (sample `/proc/stat` delta over SSH — pure parse,
tested). Client ring-buffers `ServerContainerStat.cpu/mem` strings → numbers.
**Anchors.** `commands/servers.rs` `server_health`, `server_container_stats`;
`routes/servers/[alias]/+page.svelte`.
**Acceptance.** Host + container metrics render as live gradient charts/gauges with smooth updates.

### S2 — App-centric dashboard · P1 · ▅
Master/detail: left = **app cards** (status pill w/ pulse, ports, uptime, mini CPU/RAM sparkline,
last-deploy chip); right = detail with `Tabs` (Overview · Logs · Env · Domains · Terminal).
Selecting animates (crossfade). Host picker (`servers/+page.svelte`) gains live status dots +
mini metrics so you triage at a glance.
**Acceptance.** Reads as a dashboard of apps with motion + live status, not a flat list.

### S3 — Deployment timeline + deploy/redeploy/rollback · P1 · ▅
A **Timeline** of deployments (who/when/commit/status) with a **DiffView** of env/image between
releases; one-click redeploy/rollback over SSH streaming progress into `LogStream`, with a
**status transition animation** (blue→green swap visualized). Reuse the blue-green engine.
**Anchors.** `crush-build/deploy_targets.rs`, `crush-deploy/bluegreen.rs` + `ssh.rs` (`SshBlueGreen`).
**Acceptance.** Redeploy from the GUI; the timeline grows and rollback restores the prior release.

### S4 — Env editor + domains/SSL · P2 · ▃
Env tab: a polished key/value editor (masked secrets, paste-`.env`, diff-on-save). Domains tab:
map domain→app, issue TLS via our gateway/ACME, show **cert status with an expiry ring**.
**Acceptance.** Edit env (diff shown), add a domain, watch cert status.

### S5 — Embedded terminal · P2 · ▃
`TerminalPane` (0.14) over `server_exec_pty` (host or `docker exec`), themed to our palette.
Replaces the current external-terminal shell-out. **Acceptance.** Live shell in-app.

### S6 — Container inspect (rich) · P3 · ▃
`server_container_inspect` (`docker inspect` over SSH → typed) rendered with `JsonTree` + a pretty
summary (ports, mounts, networks, restart policy, health). **Acceptance.** Full container detail.

---

## Phase D — Database (Supabase-level + craft)

### D1 — The virtual relational grid · P1 · ▅
`DataGrid` (0.6) on the data tab: server-paginated via `db_browse_table` (built), sort→`ORDER BY`,
per-column filter→`WHERE`, **frozen PK column**, **range select + copy as CSV/JSON**, zebra, cell
focus ring, sticky header, row-count badge. Type-colored cells (numbers right-aligned, NULL in
faint italic, bool as ●/○, timestamps humanized with a tooltip of the raw value).
**Anchors.** `database/+page.svelte` data tab; `commands/database.rs db_browse_table`.
**Acceptance.** Large table browses smoothly with spreadsheet ergonomics.

### D2 — FK navigation + relationship hover-cards · P1 · ▃
FK cells render as **chips** ("12 → users") that navigate to the referenced row; **hovering** pops
a `HoverCard` previewing that row inline (no click needed). FK metadata already queried (~line 383).
**Acceptance.** Hover an FK → see the referenced row; click → jump to it filtered.

### D3 — Type-aware editors · P1 · ▃
Editors from column type (`information_schema.columns` ~line 317): enum→dropdown (`pg_enum`),
bool→toggle, json/jsonb→`JsonTree`, timestamp→picker, nullable→NULL toggle, numeric→stepper.
Optimistic apply; destructive routes through the `db_estimate_impact` dry-run. **Acceptance.**
Type-correct inline editing with instant feedback.

### D4 — ERD diagram with auto-layout + minimap · P2 · ▅
Graph from FK metadata: tables as draggable nodes, FKs as edges, auto-layout, minimap, zoom/pan;
click a node → open in grid; highlight a table's relationships on hover. Lightweight SVG, no dep.
**Acceptance.** Explore the schema visually; node→grid navigation works.

### D5 — Query workbench polish · P2 · ▃
SQL editor with `CodeBlock` highlighting + autocomplete of table/column names; **result-set
mini-charts** (auto bar/line when result is numeric); **EXPLAIN visualizer** (query plan as a
collapsible tree with cost heat). **Acceptance.** Run a query → optional chart + a readable plan tree.

### D6 — Table editor (DDL via UI) · P2 · ▃
Forms generating DDL (create/alter/add-drop column, FK, index) with a **SQL preview** before run;
typed confirmation for destructive. **Acceptance.** Add a column via form; grid reflects it.

### D7 — Row history + saved queries + export · P3 · ▃
Per-row change timeline (where audit data exists); saved snippets + history in `.crush/db/`
(atomic-save like `depstate.rs`); export grid to CSV/JSON. **Acceptance.** Save/re-run a snippet; export.

---

## Phase O — Storage (R2 / Supabase-Storage polish)

### O1 — Folder breadcrumbs + tree · P1 · ▃
Browse `/`-delimited keys as folders (client-derive, or add a `delimiter` param to
`storage_list_objects`); breadcrumb + collapsible folder tree. **Acceptance.** Drill in/up via tree+crumbs.

### O2 — Masonry grid + thumbnails + lightbox · P1 · ▃
List/grid toggle; **image thumbnails** via `storage_get_presigned_url`; a **lightbox** with zoom +
EXIF; previews for text/json/pdf/video/audio via `storage_read_object_preview`. File-type icons +
size/age. **Acceptance.** Grid of thumbnails; click → lightbox/preview.

### O3 — Drag-and-drop upload with progress rings · P1 · ▃
DnD zone with a drag-ghost; per-file `ProgressRing`; large files use the multipart path
(R6.1 in `RELIABILITY_AND_PARITY_PLAN.md`) with pause/abort. **Acceptance.** Drop many files, see progress.

### O4 — Details panel + bulk actions · P2 · ▃
Right `Drawer`: size, type, modified, ETag, public/private toggle, **copy public URL**, presigned
link, metadata edit (`storage_get/set_object_metadata`). Bulk select → delete/make-public.
**Acceptance.** Inspect/edit an object; bulk-delete a selection.

---

## Phase N — Native Services (the local Supabase/Render service rack)

Current: `routes/services/+page.svelte` 344 LOC — a "start a service" button row + per-project
groups with a connection-string copy, logs, stop. Functional but utilitarian. Backend has
`list_native_services`, `start_native_service`, `stop_native_service`; `NativeServiceSummary` carries
`kind/port/pid/connection_string/data_dir/started_at/console_url`.

### N1 — Service cards with live vitals · P1 · ▃
Each running service = a rich card: `TechIcon` for its kind, **status pill (pulsing)** from a live
`is_alive` probe, port, uptime, **mini CPU/RAM sparkline** of the service process, copyable
connection string (masked, click-to-reveal), and quick links — **"Open in DB Studio" / "Open in S3
Studio"**, console (`console_url`, e.g. MinIO), Logs (`LogStream`), Restart, Stop.
**Anchors.** `routes/services/+page.svelte`; `commands/*` (add per-pid CPU/RAM sample + `is_alive`).
**Acceptance.** Running services read as live cards with vitals and one-click studio/console/logs.

### N2 — Service catalog (spin up anything) · P1 · ▃
Replace the button row with a **catalog grid**: Postgres, MySQL/MariaDB, Redis, MongoDB, MinIO —
each with an icon, blurb, and (where it applies) a **version picker** (ties to the toolchain idea).
"Add service" → starts it, optionally attached to a project. Visual provisioning state.
**Acceptance.** Pick a service + version from the catalog and it starts with progress feedback.

### N3 — Grouping, health, and data management · P2 · ▃
Group by project / scratch with collapsible sections; per-service **data actions** (snapshot/reset
via the DB snapshot + sandbox engines we built), data-dir size, and a health timeline (`UptimeBars`).
**Acceptance.** Snapshot/reset a service's data from its card; see health history.

---

## Phase H — Dashboard polish (the home that frames everything)

Current: `routes/dashboard/+page.svelte` 745 LOC — project detection, run, deploy targets, branch
switcher, services summary, tunnel. Solid bones; needs visual hierarchy + a "control center" feel.

### H1 — Overview hero · P2 · ▃
A top band of **at-a-glance gauges/sparklines**: disk-usage **DonutGauge** (from `SystemInfo.
disk_breakdown`), counts for running services / active runs / images, and host CPU/RAM if local.
Count-up animation, themed.
**Anchors.** `routes/dashboard/+page.svelte`; existing `SystemInfo`/`disk_breakdown`.
**Acceptance.** The dashboard opens with a crisp system overview, not just a project card.

### H2 — Live run + activity feed · P2 · ▃
Active runs as **live status cards** (state pill, port, uptime, mini metric) wired to the run store;
a **recent-activity timeline** (runs, deploys, snapshots) using the `Timeline` component.
**Acceptance.** In-progress runs and recent activity are visible and update live.

### H3 — Quick actions + ⌘K affordance · P3 · ▁
A quick-actions grid (Run, New API request, Start a service, New sandbox, Deploy) and a visible
**⌘K** hint that opens the command palette (Phase 0.8). Polished empty state when no project is open.
**Acceptance.** Common actions are one click/keystroke away; empty state is inviting.

---

## Phase X — The showpieces only Crush can do (visual "wow") · P2
- **Live request→DB correlation:** fire a request (API studio) → the affected DB row **pulses**
  in the grid + matching **log lines highlight** in `LogStream`, all in one view.
- **Sandbox reset animation:** "Reset data" visibly snaps the grid back to the seeded baseline.
- **Deploy choreography:** the blue→green swap animates across the deployment timeline with health
  turning the new release live.
These are the screenshots that sell it — none are possible without Crush running the stack.

---

## Cross-cutting polish (applies to all)
- **⌘K command palette** + keyboard shortcuts on every surface (`j/k` rows, `/` filter, `e` edit).
- **Toasts** for every async action (success/undo/error-with-retry); **optimistic** edits.
- **Skeletons** on first load; **empty states** illustrated, never a bare "no data".
- **Right-click context menus** (copy id, open related, delete) everywhere data lives.
- **Responsive + a11y**: keyboard-navigable grids, focus rings, ARIA, `prefers-reduced-motion`.
- Consistent iconography (`Icon.svelte`/`TechIcon.svelte`), one motion budget, theme + density.

## Appendix M — Micro-interaction spec (free-to-spec vs. needs-a-primitive)

The craft details that separate "functional" from premium. Svelte's built-ins (`animate:flip`,
`svelte/motion` `Spring`/`Tween`, `transition:`, CSS) make most of these nearly free — so they're
**spec, not research.** The few that need real architecture route through the Phase-0 primitives.

| Detail | Implementation (our stack) | Effort | Where |
|---|---|---|---|
| **200ms skeleton gate** (no flicker under 150ms) | delayed `loading` flag via `setTimeout`; skip skeleton if data beats it | ▁ | free / Skeleton (0.9) |
| Optimistic edit + rollback | local apply now, revert + ToastHost on error | ▁ | free |
| Kinetic list insertion (carve gap, slide in, glow→neutral) | `animate:flip` + enter `transition:` | ▁ | free / DataGrid, LogStream |
| No numeric jitter | `font-variant-numeric: tabular-nums` | ▁ | free / V1 token |
| Optimistic counting tween | `svelte/motion` `Tween` (spring) | ▁ | free |
| `active:scale-98` press feedback | CSS `:active` transform + fast easing token | ▁ | free / V1 |
| Secret peek + auto-remask countdown | reveal + shrinking CSS line + timer → remask | ▁ | free |
| Copy unmasked without revealing | clipboard write of raw value + checkmark anim | ▁ | free |
| Hold-to-confirm ring (snap-back on release) | pointer events + `Spring` ring fill; cancel→spring to 0 | ▃ | free (destructive actions) |
| Sticky-header scroll ingress | IntersectionObserver sentinel at `y=0` → fade border+blur | ▁ | free |
| Stale-telemetry degradation layer | stream-stall flag → desaturate chart + "cached" label | ▃ | free / AreaChart (0.2) |
| Synchronized skeleton shimmer | one shared keyframe on root (or single container gradient) | ▁ | free / Skeleton |
| Elastic toast stack + proximity-freeze | ToastHost: 3D stack via transforms; hover pauses all timers | ▃ | free / ToastHost (0.9) |
| Dual-ring focus glide | `box-shadow`/outline transition from V1 tokens | ▁ | free / V1 |
| **Layered Escape + focus restoration** | overlay registry, top-most-first close, focus return | ▅ | **primitive 0.15** |
| **Hover-card safe-triangle** | pointer-trajectory check before dismiss | ▃ | **primitive 0.15** |
| **Context-mutating ⌘K placeholder** | ContextBus → CommandPalette placeholder/options | ▁ | **primitive 0.16** |

**Build order within craft:** ship the *free* row early (they're spec-level and lift perceived
quality immediately — the **200ms gate + kinetic insertions** give the biggest perceived-perf jump);
treat **0.15 OverlayStack/FocusManager** as the one real engineering investment, since the layered
Escape, focus restoration, and safe-triangle all depend on it.

## Sequencing
1. **Phase V + Phase 0** — design system + component library (the multiplier; do not skip).
2. **S1 + S2** — servers worst→live app dashboard (biggest visible jump).
3. **D1 + D2 + D3** — the relational grid + FK hover/nav + type editors (the Supabase feel).
4. **N1 + N2** — native-services live cards + catalog (cheap, high-visibility, reuses Phase-0).
5. **S3 + O1 + O2 + O3** — deploy timeline + storage folders/thumbnails/lightbox/drag-drop.
6. **H1 + H2** — dashboard overview hero + live activity (ties the rooms together).
7. **Phase X showpieces**, then **D4–D7, S4–S6, O4, N3, H3** — ERD, workbench, env/domains/
   terminal, details, service data mgmt, ⌘K.

## Backend additions (Rust; pure parts unit-tested)
`server_health.cpu_pct` sampler · `server_app_redeploy`/`rollback` · `server_app_env_get/set` ·
`server_domains` · `server_exec_pty` (stream) · `server_container_inspect` (parse) ·
`pg_enum` value query + DDL builders (DB) · optional `delimiter` on `storage_list_objects`.

## Cross-cutting requirements
- Reuse/extend the design tokens; the Phase-0 library is the consistency backbone.
- Live pollers/streams clean up in `$effect`/`onDestroy`; no leaked timers.
- New Tauri commands registered in `lib.rs` + wrappers in `tauri.ts`; new events safe-defaulted.
- `cargo check --target x86_64-pc-windows-gnu` for any `#[cfg(windows)]` paths.
- No stubs/`todo!()`/`panic!`; new Rust parsers get unit tests; new pages keep svelte-check clean.
