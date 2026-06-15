# Crush → Local PaaS ("local Dokploy") — Implementation Plan

**Audience:** an implementing agent. **Reviewer/deployer:** the validating agent (me) — I review each phase's diff, run the build/tests, and cut the release. **Do not** run `scripts/release.sh` or `git push ci` yourself; open the work as commits on `main` and hand off for validation + release.

## Ground rules (read first)

- **Commits:** solo on `main` as the repo user. **Never** add `Co-Authored-By`/Claude/Anthropic trailers. Don't push `ci` (that's releases only).
- **Server box is disk-tight** (~75G, often >90% full). Run `cargo clean` before large builds; never leave multi-GB junk. Don't touch the gunicorn/uvicorn services on :8000/:8001.
- **Verify everything:** `cargo build -p <crate>` for Rust; `cd crates/crush-gui && node_modules/.bin/svelte-check --tsconfig ./tsconfig.json` for the frontend. Baseline svelte-check noise that is NOT your fault: `vite.config.ts` "Cannot find name 'process'" and `TechIcon.svelte` "Cannot find module 'simple-icons'" (env-only; both pre-exist). Any *other* error is yours.
- **No scaffolding / no stubs.** Build complete, working features. The user judges by what's visible in the **installed GUI** — every backend command must be wired to a visible, usable UI in the same phase.
- **Windows-first.** The user runs the Windows GUI installer (`Crush_x.y.z_x64-setup.exe`, built by AppVeyor from the origin tag). Linux/macOS are secondary. GUI changes only reach the user via a new installer (a release) — the validator handles that.

## Architecture cheat-sheet (where things live)

- **GUI desktop app:** `crates/crush-gui` — Tauri 2 + SvelteKit (Svelte 5 runes: `$state`/`$derived`/`$effect`).
  - Rust commands: `crates/crush-gui/src-tauri/src/commands/*.rs`, registered in `src-tauri/src/lib.rs` (`invoke_handler![...]`). Shared state: `src-tauri/src/state.rs` (`AppState`).
  - Frontend: `src/routes/<page>/+page.svelte`; nav in `src/lib/components/Sidebar.svelte`; API wrappers in `src/lib/tauri.ts`; icons in `src/lib/components/Icon.svelte` (lucide-style stroke paths) and brand icons in `src/lib/components/TechIcon.svelte` (simple-icons; verify slugs exist before importing — a bad named import breaks the build).
- **Engine/library:** `crates/crush-build` — detection (`detect.rs`), run flow (`run.rs`), plus the feature modules already added: `ssh.rs`, `deploy_targets.rs`, `tunnel.rs`, `gateway.rs`, `mailbox.rs`, `lint.rs`, `bluegreen` (in `crush-deploy`).
- **Deploy providers:** `crates/crush-deploy` (`SshProvider`, `bluegreen`, provider trait).
- **Native services:** `crates/crush-services` (Postgres/Redis/Mongo/MinIO drivers).
- **Remote ops pattern (already used by `commands/servers.rs`):** shell out to the system `ssh` in BatchMode and parse stdout. Reuse `ssh_exec(host, cmd)` there — extract it to a shared helper if more files need it.

### Already shipped (baseline you build ON — don't redo)
Server detail view (`/servers/[alias]`): health (CPU/mem/disk/uptime/OS) + `docker ps` with restart/stop/logs. SSH Servers panel. Deploy-platform detection (`deploy_targets.rs`) + dashboard "Deploys to" + one-click deploy. Git-aware run (branch switch + dirty badge). Tunnel (always-available). DB snapshots (`crush db`). Native service start from GUI. Mail catcher. Blue-green deploy.

---

## Phase 1 — Quick wins (do first; each is small + independently shippable)

### 1a. Smarter branch picker (default `main`)  — task #36
- File: `src/routes/dashboard/+page.svelte` (branch `<select>`).
- Order branches: `main`/`master` first, then the current branch, then the rest alphabetically. Keep `b.is_remote` filtered out.
- Helper in the script: `sortedBranches = $derived([...])`. No backend change.
- **Accept:** dropdown lists main first; current still selected; switching unchanged.

### 1b. Accurate polyglot stack labels — task #34
- Symptom: a Go + React/Vite project shows a generic label / "latest".
- Files: `crates/crush-build/src/detect.rs` (label construction) and/or how the GUI/`run.svelte.ts` derives the language string. The deployment record's `language`/`framework` is what the Deployments list shows.
- Fix: when multiple runnable stacks are detected (monorepo `services` with different `runtime_type`s, or a root with both `go.mod` and a `package.json`+vite), produce a composite label like `Go · React (Vite)` instead of one generic runtime. Never emit a bare version/"latest" as the label.
- Add a unit test in `detect.rs` for the Go+Vite case.
- **Accept:** `crush detect` and the GUI show the real multi-stack label; test passes.

### 1c. Env var editor in the GUI
- New Tauri command(s) in a `commands/env.rs`: `read_env(path) -> Vec<{key,value,secret:bool}>` (parse `.env`; reuse the masking logic in `crush-build/src/env.rs` for secret detection) and `write_env(path, entries)` (atomic write, preserve comments where feasible — at minimum round-trip key=value lines).
- UI: a section on the dashboard project card (or a small modal) listing env vars with edit/add/remove; secrets masked with a reveal toggle. Save calls `write_env`.
- **Accept:** edit a var, save, re-open → persisted in the project's `.env`.

---

## Phase 2 — Server management depth (extends `/servers/[alias]`)

### 2a. Streaming logs (follow)
- Backend: `server_container_logs_follow(host, id)` that spawns `ssh host docker logs -f --tail 200 <id>` and streams lines to the frontend via a Tauri event channel (mirror `commands/logs.rs` `subscribe_logs`/`unsubscribe_logs` pattern + `events.rs`). Track the child in `AppState` so it can be cancelled; tree-kill on unsubscribe.
- UI: in the logs panel, a "Follow" toggle that subscribes and appends lines live.
- **Accept:** logs stream live; toggling off stops the remote process (verify no orphan `ssh`).

### 2b. Native (non-docker) service visibility
- Many servers run apps via systemd/pm2, not docker (health shows `has_docker=false`).
- Backend: `server_services(host)` → `systemctl list-units --type=service --state=running --no-pager --plain` (parse) and/or `pm2 jlist` if present. Return a unified `{name, status, kind}` list.
- UI: when no docker, show this list with `restart` (e.g. `sudo systemctl restart <unit>` — gate behind a confirm; surface permission errors).
- **Accept:** on a systemd host, running services list + restart works or shows a clear permission message.

### 2c. Container terminal
- Backend: `server_container_exec(host, id)` → open a terminal running `ssh -t host docker exec -it <id> sh` (reuse the `ssh_connect`/`open_terminal` pattern; `-t` for a PTY).
- UI: an "exec" button per container.
- **Accept:** clicking opens an interactive shell in the container.

### 2d. Per-container monitoring
- Backend: `server_container_stats(host)` → `docker stats --no-stream --format '{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}'`. Optionally a small polling loop for sparklines.
- UI: CPU%/mem columns (or mini bars) in the container table.
- **Accept:** stats populate and refresh.

---

## Phase 3 — Deployments page = "local Vercel"  — task #35

Revamp `src/routes/deployments/+page.svelte` from a flat run-history list into a project-centric dashboard.

- **Per-project card** (group runs by project — already grouped): show the project's **favicon/icon** (reuse `TechIcon` for the stack; for a real favicon, optionally fetch from the running dev server `http://localhost:<port>/favicon.ico` like the dashboard does, else stack icon), **current platform** (from `detect_deploy_targets` + the live cloud deployment overlay already wired via `list_cloud_deployments`), **default branch = `main`** unless the user changed it (persist a per-project branch choice in localStorage or a small state file), status, last-run time.
- **Actions** on each card: Run, Deploy (reuse the one-click deploy command from `deploy_targets`), open live URL, view logs.
- Keep the existing run-history accessible (expand), but lead with the live/actionable view.
- **Accept:** each project shows icon + platform + branch (defaulting to main) + working Run/Deploy/Visit; looks like a deployment dashboard, not a log list.

---

## Phase 4 — Domains / TLS (the big one; do last, design-review with validator first)

- Goal: point a domain at a deployed app, with TLS — the Traefik-equivalent, but in crush's native spirit.
- Likely shape: extend the blue-green **gateway** (`crates/crush-build/src/gateway.rs`) into an HTTP reverse proxy that maps hostnames→upstreams, with ACME (Let's Encrypt) via a Rust ACME crate (e.g. `instant-acme`) and on-disk cert cache. A `crush gateway` already exists for L4 blue-green; this is the L7 + TLS evolution.
- Config: a `domains` table the GUI edits (host → project/port), persisted under the data dir; the gateway hot-reloads it.
- **This phase needs a written sub-design + validator sign-off before coding** — TLS/ACME + public exposure has real security/ops weight. Flag it as an obstacle to design carefully, not skip.
- **Accept:** add a domain in the GUI → it serves the app over HTTPS (test with a DuckDNS/test domain; do not require the user to buy one).

---

## Phase 5 — Databases-as-a-service polish

- crush already has native services + `crush db snapshot/restore`. Surface in the GUI:
  - Scheduled backups (a `crush db snapshot` cron-equivalent; reuse the cron tooling if present).
  - Per-DB monitoring + logs (already partly in the Services page).
  - Restore-from-snapshot button in the Services/DB view.
- **Accept:** schedule a backup, see snapshots listed, restore from the GUI.

---

## Per-phase definition of done (every phase)
1. Rust builds (`cargo build -p crush-gui` and any touched crate).
2. `svelte-check` shows only the 2 baseline env errors.
3. New backend logic has unit tests where pure (parsers, label builders, env round-trip).
4. The feature is **visible and usable in the GUI** (a registered command alone is not "done").
5. A short note in `crush-web/src/app/pages/changelog.page.ts` is NOT added by you — the validator adds it at release time.
6. Commit on `main` with a clear message; hand off. The validator runs the full build, then `scripts/release.sh patch` as a **harness-tracked background job** (lesson: backgrounded-and-untracked release builds get their process tree killed and stall).

## Handoff protocol
- Work one phase at a time; commit per phase. Leave the tree compiling.
- In each commit body, list: files touched, new Tauri commands (so the validator confirms they're registered in `lib.rs`), and how you verified.
- Do not bump versions, edit release scripts, or push to `ci`/origin tags. The validator validates + releases.

---

# APPENDIX A — Exhaustive feature backlog (nothing omitted)

Every item below is a discrete, shippable task. `[have]` = already exists, surface/connect it. `[partial]` = exists but incomplete. `[new]` = build from scratch. Reuse existing code wherever marked. Group ≈ a GUI screen/section.

## A1. Applications (deploy a project)
- [ ] [partial] **Deploy app** — one-click deploy already runs the inferred CLI (`deploy_targets.rs`); add a guided "Create Application" flow: pick project dir → choose source → choose build → deploy.
- [ ] [new] **Stop app** (running deployment): for crush/SSH deploys, `crush deploy --stop` (add it) or `docker stop`; for PaaS, the provider CLI.
- [ ] [new] **Delete app**: tear down (remove container/service + state record in `~/.crush/deployments`).
- [ ] [have] **Terminal in container** — Phase 2c (`docker exec -it sh`).
- [ ] **Source providers** (selector): `[have]` Git/GitHub (git-aware run + remotes), `[new]` Docker image source (run a prebuilt image), `[new]` "Raw" (paste a compose/Dockerfile).
- [ ] **Build types** (selector, with auto-detect default): `[have]` crush-native detection (no Dockerfile), `[have]` Dockerfile, `[new]` Nixpacks (`nixpacks build`), `[new]` Heroku buildpacks (`pack build`), `[new]` Paketo buildpacks. Detect availability of each CLI (`commands/deploy.rs::cli_available` pattern) and only offer installed ones.
- [ ] [partial] **Environment variables** — Phase 1c editor; per-app + per-deploy scoping.
- [ ] [partial] **Monitoring** CPU/mem/disk/**network** — server-level done; add per-app and **network** (`docker stats` includes NetIO; for host network use `cat /proc/net/dev` or `ifstat`). NOTE: network was in Dokploy's list — don't skip it.
- [ ] [partial] **Real-time logs** — Phase 2a streaming; also stream **build logs** while a deploy/build runs (not just runtime logs).
- [ ] **Deployments view per app**: history of deploys with status + per-deploy build log; **cancel queued deployments** (`[new]` a deploy queue in `AppState`: when multiple deploys are requested, queue them; allow cancelling not-yet-started ones; never kill an in-progress one).
- [ ] **Domain management** — Phase 4: add / delete / **auto-generate** a domain (e.g. `<app>.<host>.duckdns.org` or a `*.crush.local` hosts-file entry for pure-local).
- [ ] **Advanced settings** (per app): `[new]` initial command override, `[new]` **append command** (add to crush's build command, don't replace), `[new]` cluster/replica count, `[new]` resource limits (reuse `RunOptions.memory_bytes`/`cpu_fraction` + Windows Job Objects already in `run.rs`), `[new]` volumes/mounts for persistence, `[new]` redirects, `[new]` security headers, `[new]` port settings, `[new]` raw Traefik/gateway config passthrough.

## A2. Docker Compose management
- [ ] [have] **Deploy/stop** compose — `crush compose up/down` exists; surface in GUI with status.
- [ ] [new] **Delete** compose project (down + remove volumes opt-in).
- [ ] [new] **Terminal with service selection** — pick a service → `docker compose exec <svc> sh`.
- [ ] **Source**: GitHub/Git/Raw (paste compose).
- [ ] [partial] **Env vars** per compose (crush already merges per-service env in `run.rs`).
- [ ] [partial] **Per-service monitoring** (CPU/mem/disk/network) — `docker stats` per service.
- [ ] [partial] **Per-service real-time logs** — extend Phase 2a to compose services.
- [ ] **Deployments view** + build logs + cancel queue (shared with A1).
- [ ] [new] **Append command** to the internal compose build command; **volumes/mounts** management.

## A3. Databases-as-a-service (Phase 5 + extras)
- [ ] [have] **Supported engines** — Postgres, Redis, MongoDB, MinIO already native (`crush-services`); **add MySQL + MariaDB** drivers (MySQL is referenced in `bluegreen`/`db.rs` already; add a `MySqlDriver` like the others). Dokploy lists MySQL/Postgres/Mongo/Redis/MariaDB.
- [ ] [have] **Deploy/stop** DB — GUI `start_native_service`/`stop_native_service` exist; add MySQL/MariaDB to the picker.
- [ ] [new] **Delete** DB (stop + remove data volume, confirm).
- [ ] [have] **Terminal in DB** — open `psql`/`mysql`/`mongosh`/`redis-cli` in a terminal (reuse `open_terminal`).
- [ ] [partial] **Env vars** for DB services.
- [ ] [partial] **Monitoring** CPU/mem/disk/network per DB.
- [ ] [have] **Backups** — `crush db snapshot/restore` exists; add `[new]` **scheduled** backups (cron) + GUI to configure + list + restore + delete.
- [ ] [partial] **Real-time logs** — Services page already reads service logs; add follow.
- [ ] **Advanced**: `[new]` custom Docker image for a DB, `[new]` initial command, `[new]` volumes, `[new]` resource limits.

## A4. Servers (infra)  — mostly done, extend
- [ ] [have] health (CPU/mem/disk/uptime/OS) + `docker ps` + restart/stop/logs (`/servers/[alias]`).
- [ ] [new] **Network** usage on the health panel (`/proc/net/dev` delta, or `ifstat`/`vnstat` if present).
- [ ] Phase 2a/2b/2c/2d: streaming logs, systemd/pm2 services, container exec, per-container stats.
- [ ] [new] **Add/remove server** from the GUI (append a `Host` block to `~/.ssh/config`; validate connectivity).
- [ ] [new] **Server-wide actions**: prune docker (`docker system prune`), reboot (confirm), free disk.

## A4b. Server → typed service inventory + studio linkage + deployment mgmt (Dokploy-grade)
Status: server detail shows generic `docker ps` + native services already. Build the rest:
- [ ] **Typed service inventory.** Classify each container by image (reuse `crush_build::service_orchestrator::native_driver_for` mapping): Postgres/MySQL/MariaDB/Mongo/Redis → **Database**; minio → **Object storage (S3)**; everything else → **App**. Render with the right `TechIcon` + a type badge, grouped (Databases · Storage · Apps · Other). This is the "see the db, s3, and other services" ask.
- [ ] **Open remote service in our studio (over SSH tunnel).** New command `open_ssh_tunnel(host, remote_port) -> local_port` that runs `ssh -N -L 127.0.0.1:<localport>:127.0.0.1:<remoteport> host` (track the child in `AppState` so it can be closed). Then:
  - Remote **Postgres/MySQL/Mongo/Redis** → "Manage" button opens the **DB Studio** (Plan #2) pointed at `localhost:<localport>` with the container's creds (read from its env via `docker inspect`).
  - Remote **MinIO** → "Browse" opens the **Storage Studio** (Plan #3) against the tunneled endpoint.
  - Close the tunnel when the studio view closes.
- [ ] **Manage deployments from the server.** List crush-deployed apps on the host: read blue/green containers + the gateway target (or `crush ps` over SSH). Per app: **Redeploy** (re-run `crush deploy`/blue-green), **Stop**, **Logs**, **open live URL/domain**. Ties the Servers view to the Deployments + Domains features.
- [ ] **Container env + inspect.** `docker inspect <id>` → show env (mask secrets), mounts, ports, restart policy in a detail drawer (needed to derive DB creds for the studio linkage above).
- [ ] **Accept:** open a server → see services grouped/typed with icons → click a Postgres container → DB Studio opens on it via tunnel and lists tables; see deployed apps and redeploy one.

## A5. Cross-cutting / polish (loose ends from this thread — capture all)
- [ ] [partial] **Branch picker default `main`** — Phase 1a.
- [ ] [partial] **Polyglot stack labels** (Go·React(Vite), no "latest") — Phase 1b.
- [ ] [partial] **Deployments page = local Vercel** (favicons, platform, default-main) — Phase 3.
- [ ] [done] Tunnel always-available (no longer webhook-gated) — verify in GUI.
- [ ] **Verify on Windows** (couldn't repro from Linux; confirm or fix): docs-page **scrollspy** highlight tracks scroll; **Stop** kills a hung run *during the run phase* (build-phase fix already shipped) — confirm the run-phase `kill_tree`/`taskkill /T` actually reaps `node`/`ts-node-dev`/`vite` trees.
- [ ] **App-side lint hint:** when `crush eject`/build hits the AWS-SDK `TS1010 '*/' expected` class of failures, surface a hint to add `"skipLibCheck": true` (a `crush lint`/`crush doctor` rule).
- [ ] **Icons:** ensure every platform/provider/stack has a `TechIcon` (verify simple-icons slugs at build; OpenAI has no slug → monogram). Add any missing (e.g. `amazonaws`, `ssh`, `render`).
- [ ] **Project favicons** in lists (Deployments + dashboard) — fetch from running dev server, else stack icon.
- [ ] **One-click deploy inference** refinement: detect git-integration vs CLI deploy (e.g. `.vercel/` linked → `vercel --prod`; vercel.json only + git remote → `git push`); read `vercel.json`/`netlify.toml` for build settings.
- [ ] **Settings:** AI provider already defaults to Gemini free tier; add model dropdown wiring to the engine (currently `CRUSH_AI_MODEL` env only).

## A6. Data model & persistence (build once, reuse everywhere)
- [ ] A unified **App/Project registry** under the data dir: `{ name, path, default_branch, deploy_target, env_scope, last_deploy }`. Many features above read/write it. Define it early (e.g. `crates/crush-types` or a GUI state file) so Phases 1–5 share it.
- [ ] **Deploy queue** state in `AppState` (for cancel-queued).
- [ ] **Domains** table (Phase 4).
- [ ] **Backup schedules** table (Phase 5).

# APPENDIX B — Concrete remote command crib (for SSH-driven features)
- Health: `nproc`; `free -m | awk '/Mem:/{print $2","$3}'`; `df -BG / | tail -1 | awk '{print $2","$3","$5}'`; `uptime -p`; `. /etc/os-release; echo $PRETTY_NAME`; docker presence via `command -v docker`.
- Network: `cat /proc/net/dev` (parse rx/tx per iface; delta over 1s for rate).
- Containers: `docker ps --all --format '{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}|{{.Ports}}'`.
- Stats: `docker stats --no-stream --format '{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}|{{.NetIO}}'`.
- Logs (follow): `docker logs -f --tail 200 <id>`.
- systemd: `systemctl list-units --type=service --state=running --no-pager --plain`.
- All remote calls go through the BatchMode `ssh_exec` in `commands/servers.rs` (extract to a shared module if reused). **Sanitize** every interpolated id/name (see `sanitize_id`); never pass raw user strings to the remote shell.

# APPENDIX C — Acceptance demo script (validator will run this)
For each phase, the implementing agent records exact click-paths the validator can replay in the GUI: e.g. "Servers → variantrade → Manage → see health + containers → Restart `api` → Logs shows output." If a feature can't be demoed in the GUI, it's not done.

# APPENDIX D — Dokploy/Coolify parity matrix (every row accounted for)
Scope tags: `[have]` exists, `[partial]` exists-incomplete, `[new]` build it, `[inherent]` true by crush's architecture, `[out]` deliberately out-of-scope (with reason — do NOT silently skip; if revisited, design-review first).

| Row (from the comparison) | Crush scope | Notes / where |
|---|---|---|
| One-command installation | [have] | `install.sh` / `crush install`; installer ships in releases. |
| Installation feedback + progress logs | [partial] | install scripts print steps; make the GUI first-run/update show progress. |
| Works with firewall + Tailscale out of the box | [partial→new] | Tunneling already gives public reach without port-forwarding. **[new]** detect Tailscale (`tailscale status`) and offer a Tailscale URL as a tunnel provider alongside cloudflared. |
| Lightweight CPU while idle | [inherent] | No daemon; nothing runs idle. Keep it that way — don't add a always-on background service. |
| Low memory usage | [inherent] | Native processes, no VM/containers in dev loop. |
| Teams & organizations | [out] | Crush is a **local, single-user** tool; multi-tenant accounts contradict the model. Revisit only if a hosted mode is ever added. |
| Projects grouping | [new] | Use the App/Project registry (Appendix A6); group by folder/workspace in the GUI. |
| Consistent, responsive UI | [partial] | Ongoing; Svelte 5 GUI. Keep pages consistent with existing patterns. |
| Built with Next.js / TypeScript | [out] | N/A by design — crush GUI is Tauri + SvelteKit + Rust (smaller, native). Not a feature to copy. |
| AI-assisted deployments | [partial] | `crush-ai` (Gemini default) does diagnosis; **[new]** extend to deploy suggestions / fix-on-failed-deploy. |
| Deploy from custom Docker images | [new] | A1 "Docker image source" — run/deploy a prebuilt image. |
| Database deployment (PG/MySQL/Redis/…) | [have]+[new] | Native services exist; **add MySQL + MariaDB drivers** (Appendix A3). |
| Scheduled DB backups (to S3) | [partial→new] | `crush db snapshot/restore` exists; **[new]** scheduling + **S3 target** (reuse MinIO/S3 client; configurable bucket/creds). |
| Back up arbitrary Docker volumes (not just DBs) | [new] | `docker run --rm -v <vol>:/v -v <out>:/out alpine tar czf /out/<vol>.tgz /v` over SSH; list/restore. |
| Preview deployments (review apps) | [partial] | Branch previews via worktrees (`preview_branch`/`list_worktrees`) exist locally; **[new]** make a preview a *deployable ephemeral env* (deploy the worktree, get a URL, tear down). |
| API + CLI tools for automation | [have]+[new] | CLI is extensive. **[new]** optional local HTTP API (opt-in, bound to localhost) for automation — must stay off by default (no idle daemon). |
| Multi-server deployment | [new] | Deploy one app to several servers from the Servers registry; aggregate status. |
| Docker Swarm clustering | [out] | Heavy, daemon-centric, contradicts native/no-daemon ethos. Mark out; if demanded, scope a separate design. |
| Cron jobs inside containers | [new] | Add a cron entry to a target container (`crontab`/systemd timer); manage from GUI. |
| Cron jobs on the host machine | [new] | Manage host cron (crontab on Linux servers; Task Scheduler on Windows) — schedule `crush` commands (e.g. backups, redeploys). |
| Monitoring metrics (CPU/RAM/Disk) | [have]+[partial] | Server health done; add **network** + per-app/per-container (Appendix A1/A4). |
| Metrics enabled by default | [new] | Make the server detail auto-poll (already 15s) and show on the Servers list cards (a tiny health dot/summary) without opening detail. |
| Automated alerts from metrics | [new] | Threshold alerts (disk >90%, mem >90%, container down) → desktop notification (Tauri notification API) + GUI banner. Opt-in thresholds. |

**Net:** of the 22 rows, ~17 are in-scope (several already `[have]`/`[inherent]`), 3 are `[out]` by deliberate design (teams/orgs, Swarm, "built with Next.js"), and the rest are concrete `[new]` tasks folded into Appendices A/D. Add these to the per-phase work — Tailscale + custom-image + volume backups + preview-deploy + cron + alerts are natural Phase 2/3/5 additions; multi-server + local API are their own small phases.

