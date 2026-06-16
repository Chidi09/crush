# Crush API Studio — Plan

> **One sentence.** Postman and Swagger share a blind spot — they know nothing about the
> app you're actually running. Crush launched it, knows its port, tails its logs, manages its
> env, and runs its database. **API Studio** turns that into a single tool that *explores*,
> *documents*, and *sandbox-tests* your API against the real, running stack — and is therefore
> not "another API client" but something neither tool can copy.

Three pillars, one model:

1. **Explore** — import OpenAPI/Postman (or a live URL) into a clean interactive client that runs
   requests from Rust (no CORS, real cookies, streaming) and auto-points at your running service.
2. **Document** — documentation that is *derived and verified* from your real API and real
   traffic, lives in your repo, and proves it's still in sync — instead of hand-written prose
   that rots in a cloud box.
3. **Sandbox** — a real but disposable, seeded database to run/test against; reset to pristine
   in one click. The honest replacement for mock servers: don't fake the API, give it a
   throwaway backend.

The unfair advantage running through all three: **Crush runs your app.**

Legend — Effort: ▁ small (≤½ day) · ▃ medium (1–2 days) · ▅ large (3–5 days) · ▇ epic (1wk+).
Priority: **P0** foundation everything depends on · **P1** core wedge · **P2** strong · **P3** stretch.

Anchors below were read from the current tree — keep them honest as code moves.

---

## The shared model (decide this first — expensive to retrofit)

All three pillars normalise onto one structure so the UI never special-cases OpenAPI vs Postman,
and so capture/verification/docs are first-class from day one (not bolted on like Postman).

```rust
// crate: crush-apispec  (pure, heavily unit-tested)
struct ApiModel {
    servers: Vec<Server>,          // base URLs; one may be "live" (from the run store)
    groups:  Vec<Group>,           // OpenAPI tags / Postman folders
    auth:    Option<AuthScheme>,
    variables: Vec<Variable>,      // {{vars}}; values can resolve from project .env
}
struct Group  { name: String, requests: Vec<Request> }
struct Request {
    id: String, method: Method, path: String,
    params: Vec<Param>, headers: Vec<Header>,
    body: Option<BodySpec>,        // schema-driven where the spec provides one
    auth: Option<AuthScheme>,
    doc: RequestDoc,               // ← documentation is part of the request, not an afterthought
}
struct RequestDoc {
    summary: String,
    description_md: String,
    examples: Vec<CapturedExample>,        // real request+response pairs, not hand-typed
    error_examples: Vec<CapturedExample>,  // real 4xx/5xx
}
struct CapturedExample {
    label: String,
    request: SavedRequest,
    response: SavedResponse,      // actual status/headers/body captured from a real call
    verified_at: u64,            // unix secs — drives the freshness badge
    schema_ok: Option<bool>,     // validated against the spec at capture time
}
```

**Why this shape:** capture-from-traffic and drift-verification (Pillars 2) only work if examples
are structured `request+response+verified_at`, not Markdown blobs. Locking this in now is the one
irreversible decision.

**Storage.** `.crush/api/` inside the project, **git-versioned**: a manifest + per-group Markdown
so docs diff cleanly in PRs and humans can edit them in their editor. Use the atomic
write+rename pattern from `crates/crush-build/src/depstate.rs:save_deps_state`. Variables/secrets
resolve through the existing env command (`crates/crush-gui/src-tauri/src/commands/env.rs`).

---

## Phase A — Foundation: parse everything into the model · P0

### A1 — `crush-apispec` crate: unified model + parsers · ▅
**Goal.** Turn OpenAPI 2/3 **and** Postman v2.1 into the same `ApiModel`.
**Design.**
- OpenAPI: `openapiv3` crate (+ `serde_yaml`/`serde_json` for YAML/JSON). Tags→groups,
  `paths.*.operationId`→request id, `requestBody.content.schema`→`BodySpec`, `servers`→servers,
  `securitySchemes`→auth.
- Postman: a `serde` struct mirroring the v2.1 collection schema. Folders→groups, `{{var}}`→
  variables, `auth`→auth, saved responses→`CapturedExample`.
- Keep it **pure** (no I/O beyond reading bytes) so it's trivially unit-testable.
**Anchors.** New crate `crates/crush-apispec`; add to workspace `Cargo.toml` members.
**Acceptance.** Fixture round-trips: a real OpenAPI 3 spec and a real Postman export both produce
a populated `ApiModel` with matching groups/requests. Unit tests for tag→group, schema→body,
`{{var}}` extraction, auth mapping. ≥20 tests; this is the cheap-to-test, costly-to-get-wrong layer.

### A2 — Import + auto-discovery · ▃
**Goal.** Zero-config: find the spec instead of making the user wire it.
**Design.**
- Import button: drop an OpenAPI/Postman file, or paste a URL.
- **Project scan:** look for `openapi.{json,yaml}`, `swagger.json`, `postman_collection.json`.
- **Live probe:** hit the running server's common spec endpoints — `/openapi.json`,
  `/v3/api-docs`, `/swagger.json`, `/api-docs` (FastAPI/NestJS/Spring emit these for free).
**Anchors.** Probe uses the run store's live endpoints
(`crates/crush-gui/src/lib/stores/run.svelte.ts:29` `endpoints[]`).
**Acceptance.** With a FastAPI app running under Crush, opening API Studio auto-loads its
endpoints with no manual import.

---

## Phase B — Explore: the client that beats browser-Swagger on day one · P1

### B1 — Rust-side request executor · ▃
**Goal.** Execute requests from Rust — the thing browser Swagger fundamentally can't.
**Design.** Tauri command `api_send(req, env) -> ApiResponse { status, headers, body, timing_ms,
size_bytes }` using `reqwest` (already a workspace dep with `json` + `stream`,
`Cargo.toml:62`). Because it's Rust, not the webview: **no CORS, real cookies, redirects we
control, file uploads, mTLS**, and SSE/WS streaming via the `stream` feature.
**Anchors.** New `crates/crush-gui/src-tauri/src/commands/api.rs`; register in
`src-tauri/src/lib.rs` `invoke_handler!`; wrapper in `src/lib/tauri.ts`.
**Acceptance.** Fire GET/POST with headers + JSON body against the running app; get status,
headers, pretty body, and timing. A cross-origin call that fails in browser-Swagger succeeds here.

### B2 — Explorer UI · ▅
**Goal.** A clean three-pane client matching the DB/Storage studio design language.
**Design.** New `/api` Svelte route: left = endpoint tree (grouped by tag, searchable);
center = request builder (method/url/params/headers/body, with schema-driven form fields when the
spec supplies a body schema — *not* raw JSON); right = response viewer (pretty body, headers,
timing, size). Nav entry in `Sidebar.svelte` + icon.
**Anchors.** Mirror `src/routes/database/+page.svelte` / `storage/+page.svelte` structure.
**Acceptance.** Browse imported endpoints, edit a request, send, read the response — all without
leaving Crush. Persists across navigation like the run store does.

### B3 — Persistence + auto base-URL + env/auth wiring · ▃
**Goal.** No "create an environment" ritual.
**Design.** Collections/history/environments persisted under `.crush/api/`. The "live" server is
auto-filled from the run store endpoints — open the explorer and it's already on
`http://localhost:<port>`. Variables and auth tokens resolve from the project `.env` via the env
command, so secrets are never re-entered.
**Acceptance.** Open the explorer on a running app → base URL pre-set; `{{API_KEY}}` resolves from
`.env`; history survives restart.

---

## Phase C — Document: derived, verified, in-repo · P1

> **Principle:** documentation is *generated from truth and proven in sync*, not authored-and-
> rotting. Authoring is enrichment on top of real data.

### C1 — Capture examples from real traffic · ▃
**Goal.** Kill example-rot. The big one.
**Design.** In the explorer, every send yields a real `request+response`. One click → **"Save as
documented example"** stores it as a `CapturedExample` with `verified_at`. "Save as error example"
captures real 4xx/5xx — the docs everyone forgets. No hand-typed sample bodies, ever.
**Anchors.** Consumes `api_send`'s `ApiResponse` (B1); writes into `RequestDoc.examples`.
**Acceptance.** Fire a request, save it, and it appears in the endpoint's docs as a real example
with a timestamp.

### C2 — Drift / freshness verification · ▃
**Goal.** Docs that know when they're lying — impossible in Postman.
**Design.** Validate each captured response against the OpenAPI schema at capture and on demand.
Each example shows **"✓ verified N min ago"** or **"⚠ live response no longer matches."** A
"re-verify all" replays saved example requests against the running app and flags drift.
**Anchors.** Schema validation in `crush-apispec`; replay via `api_send`.
**Acceptance.** Change an endpoint's response shape, hit re-verify → the stale example is flagged.

### C3 — Docs-as-code + narrative guides · ▃
**Goal.** Versioned, reviewable, branch-diffable docs with a layer above individual endpoints.
**Design.** Persist docs as **Markdown + manifest** in `.crush/api/` (git-tracked). Add **guides**
("Authentication", "Pagination", "Webhooks") in Markdown that reference endpoints by `id` and stay
linked across renames. Optional: merge source doc-comments (FastAPI/NestJS descriptions, JSDoc,
docstrings, Rust `///`) into endpoint descriptions so docs live next to code. Git-aware: **diff API
docs across branches**.
**Acceptance.** Write a guide referencing two endpoints; rename an endpoint and the link follows;
`git diff` shows readable doc changes.

### C4 — Interactive docs + publish · ▃
**Goal.** Docs you can *run*, shippable without a cloud account.
**Design.** Markdown with embedded **runnable request blocks** (notebook-style: prose, then a live
"Try it" cell that executes via `api_send` and shows the real response inline). Publish: generate a
clean static doc site served locally through the L7 gateway
(`crates/crush-build/src/gateway.rs:run_l7_gateway`, with ACME/TLS we already have) or shared via
the tunnel; export enriched OpenAPI / Markdown.
**Acceptance.** A guide's embedded request executes against the running app; "publish" serves a
clean doc site at a local URL with no Postman account.

---

## Phase D — Sandbox: a real, disposable, seeded backend · P1

> **Reframe:** don't mock the API — give it a throwaway, seeded DB. You test the real stack against
> deterministic data, then reset or discard. Only possible because Crush runs the DB.

### D1 — Ephemeral DB via TEMPLATE clone + env redirect · ▃
**Goal.** Instant, isolated, disposable database state.
**Design (the elegant core).** On the crush-managed Postgres: build a pristine
`crush_seed_template` (migrated + seeded) once. Each sandbox run is
`CREATE DATABASE crush_sandbox_<id> TEMPLATE crush_seed_template` — near-instant (file copy).
**Reset = drop + recreate from template = instant clean slate.** Drop on exit. Point the app at it
by injecting `DATABASE_URL`/connection env into `dep_env` — exactly what `synthesize_dep_env`
already does for native services (`crates/crush-build/src/run.rs:505`,
`synthesize_dep_env`). MySQL/Mongo get the equivalent throwaway-database approach.
**Anchors.** Reuses native DB drivers (`crates/crush-services/src/driver.rs` `ServiceDriver`),
`dep_env` injection, and snapshot/restore (`crates/crush-build/src/dbsnapshot.rs`,
`commands/database.rs` `pg_restore`).
**Acceptance.** `crush run --sandbox` runs the app against a cloned DB; mutating data never touches
the dev DB; "Reset data" returns to pristine in <1s on a moderate schema.

### D2 — Fresh ephemeral instance (stronger isolation / no crush DB) · ▃
**Goal.** OS-level isolation, or sandbox when no crush Postgres is running.
**Design.** Spin a second Postgres on a temp `data_dir` + spare port via `ServiceDriver`
(`ensure_ready`→`start`), seed, run, then `stop` + delete the data dir. ~1–2s startup.
**Acceptance.** Sandbox works with no pre-existing crush DB; instance + data dir are fully removed
on teardown.

### D3 — Seeding layer · ▃
**Goal.** Get realistic data in fast, three ways.
**Design.**
1. **Detected seed script** — run the project's own (`prisma db seed`, `knex seed`,
   `rails db:seed`, `drizzle`, `seed.sql`), using the same command-detection approach as the
   migration guard (`crates/crush-build/src/dbsnapshot.rs:is_migration_command`).
2. **Fixtures** in `.crush/seeds/` (SQL or JSON), git-versioned.
3. **Schema-aware generation (the wedge)** — read schema (extend the column introspection in
   `commands/database.rs` with `information_schema` for types + FKs), topologically sort tables by
   FK order, and generate constraint-correct rows ("500 users, 2k orders" with valid foreign
   keys). Postman has nothing like this.
**Acceptance.** Each seed source populates the sandbox; generated data satisfies NOT NULL / FK /
type constraints and inserts in dependency order.

### D4 — Auto-teardown sweeper · ▁
**Goal.** Sandboxes never leak.
**Design.** Drop stale `crush_sandbox_*` DBs / remove orphaned ephemeral data dirs on startup and
on a periodic tick (same idea as the ACME-state cleanup in `gateway.rs`).
**Acceptance.** A sandbox left over from a crash is reclaimed on next launch.

---

## Phase E — The unified loop (the magic neither competitor can match) · P2

### E1 — Live log + DB correlation · ▃
Fire a request in the explorer → see the matching lines surface in the run-log pane → see the row
change in DB studio, all in one window. Correlate by time + the sandbox DB connection.
**Acceptance.** A POST shows its server log lines and the new DB row side-by-side with the response.

### E2 — Codegen · ▁
Generate `curl` / `fetch` / language snippets from any request (and from captured examples).

### E3 — API diff across branches · ▃
Using git-aware docs + spec, show what changed in the API between `main` and the current branch
(added/removed endpoints, changed schemas, drifted examples).

### E4 — Streaming + webhooks · ▃
SSE/WebSocket inspection via reqwest `stream`; point external webhooks (Stripe/Paystack) at the
Crush tunnel and inspect the inbound request in the same UI.

---

## Suggested sequencing
1. **A1 + A2** — model + parsers + discovery. Nothing works without the model; it's also the most
   testable, so it de-risks everything.
2. **B1 + B2 + B3** — the explorer. Ships a tool that already beats browser-Swagger (no-CORS +
   persistence + auto base-URL) — a usable product at this point.
3. **C1 + C2** — capture + drift. The documentation wedge, riding on B1.
4. **D1 + D3** — TEMPLATE-clone sandbox + detected-seed/fixtures. Cheap, huge value.
5. **C3 + C4** — docs-as-code, guides, publish.
6. **D2, D3(generation), E*** — stronger isolation, generated data, and the correlation/diff magic.

## What we deliberately do NOT build (so we don't lose)
- Postman's cloud team-sync, JS scripting sandbox, monitors/schedulers — that's their bloated moat,
  not our game.
- Standalone mock servers returning fake bodies — replaced by the **real seeded sandbox**, which is
  strictly better.
Our edge is **spec-first + verified docs + deeply wired into the app you're running locally.**
Stay there and API Studio is genuinely 1-of-1.

## Cross-cutting requirements (every task)
- `crush-apispec` is pure and unit-tested (parsing, schema validation, var extraction, FK ordering).
- New Tauri commands registered in `lib.rs` `invoke_handler!` with wrappers in `tauri.ts`; new
  events handled with a safe `default` in the dispatcher.
- `cargo check --target x86_64-pc-windows-gnu` for anything touching `#[cfg(windows)]` (the 1.0.3
  `sysinfo` regression came from host-only validation).
- Match the existing design language (Svelte 5 runes, `Icon.svelte`/`TechIcon.svelte`, the DB/
  Storage studio layouts). No stubs, no `todo!()`, no `panic!("not implemented")`.
- Docs/collections live in `.crush/api/` and `.crush/seeds/` — git-versioned, human-editable,
  written atomically (depstate pattern).
