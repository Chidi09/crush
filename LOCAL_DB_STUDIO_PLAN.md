# Crush → Local Database Studio — Implementation Plan (Plan #2)

**Thesis (validated):** crush's biggest differentiation opportunity is **local database management**. It already runs native Postgres/MySQL/MongoDB/Redis/MinIO (`crates/crush-services`), has read inspectors (`commands/inspect.rs`), and snapshots (`crush db`). Turn that into a **slick, free, local Supabase-Studio/TablePlus-class GUI** on the `/database` page. This plan is what's *worth building* — Supabase ideas that don't fit crush are listed as **out-of-scope with reasons** so we don't chase them.

**Same ground rules as `LOCAL_PAAS_PLAN.md`** (read that file's "Ground rules" + "Architecture cheat-sheet" first): solo commits on `main`, no co-author trailers, disk-tight box (`cargo clean` before big builds), verify with `cargo build` + `svelte-check` (baseline noise = the 2 env errors only), **no stubs — every command wired to visible UI**, Windows-first, don't push `ci`/release (the validator does that). Work one phase per commit; hand off for validation.

## Scope decision (the part you asked me to validate)

| Supabase pillar | Crush scope | Why |
|---|---|---|
| **Database management** (data, schema, extensions, queries, perf, roles/RLS, functions/triggers) | **IN — the core of this plan** | Local, native PG already runs; pure dev-tool value; under-served by free local tools. |
| **Queues (pgmq)** | IN, late phase (D7) | Just an extension + thin UI; falls out of extension management. |
| **Scheduled backups (incl. S3)** | IN (shared with PaaS plan Phase 5 / D6) | `crush db snapshot` exists; add schedule + S3 target. |
| **Edge Functions** | OUT (maybe tiny "run a Deno function" later) | The value is *global edge distribution* = a CDN/hosted product, not a local tool. Webhook use-case already covered by tunneling. |
| **Realtime** (Broadcast/Presence/Postgres Changes service) | OUT | Hosted, always-on, distributed websocket service — daemon-heavy, opposite of crush's no-daemon model. (A `LISTEN/NOTIFY` debug viewer is the only sliver that could fit, far future.) |
| **Vault (secrets in DB)** | OUT | Supabase-specific; crush already has env/secret handling. |

## Foundations (build first, reused by every phase)

**Backend — a real SQL execution layer.** Today `inspect.rs` shells/queries for read-only summaries. We need a general **query runner** per engine.
- Add a `crush-db` capability (new module in `crates/crush-services` or a new small crate `crush-db`): `run_sql(engine, conn, sql) -> { columns, rows, affected, error, duration_ms }`.
  - Postgres: use a Rust driver (`tokio-postgres` — already transitively available? if not, add it) OR shell out to the bundled `psql` with `--csv` and parse. Prefer `tokio-postgres` for typed results + parameterization; fall back to `psql --csv` if adding the dep is heavy. **Decide with validator before adding a DB driver dep** (build-size/Windows cross-compile impact).
  - MySQL: `mysql --batch` / `mysql_async`. MongoDB: `mongosh --eval` JSON. Redis: `redis-cli`.
- Connection resolution: reuse `db.rs::resolve_connection` (DATABASE_URL / crush-managed service) and the Services registry. The studio targets a chosen connection (crush-managed native DB by default; or a pasted URL).
- **Safety:** parameterize where possible; for the raw SQL editor, this is a power tool (the user owns the DB) but: confirm-gate destructive statements (DROP/TRUNCATE/DELETE without WHERE), run in a read-only txn for "preview", never interpolate untrusted strings into identifiers.

**Frontend — the `/database` page** (route already in the nav). A connection switcher at top (crush-managed services + add-URL), then tabs: **Data · SQL · Schema · Extensions · Performance · Security · Backups**. Reuse `inspect_*` for the initial overview.

---

## Phase D1 — Data + SQL (the daily driver)
- **Tables/views list** (sidebar): from `information_schema`/`pg_catalog` (Postgres), `SHOW TABLES` (MySQL), collections (Mongo).
- **Data grid:** select a table → paginated rows; sortable columns; filter bar (WHERE builder or raw). **Edit cells** (UPDATE by PK), **insert row**, **delete row** (confirm). Show types + nullability. Reuse existing `PgInspect` types as a starting point; extend.
- **SQL editor:** textarea (later: CodeMirror) → run → results grid + affected-rows + timing + errors. History of run queries (local).
- **Accept:** open a crush Postgres, browse a table, edit a cell (persists), run `select …`, see rows + timing.

## Phase D2 — Extensions manager (the differentiator)
- List available + installed extensions: `SELECT * FROM pg_available_extensions`.
- **One-click enable/disable:** `CREATE EXTENSION IF NOT EXISTS <x>` / `DROP EXTENSION`. crush already builds **pgvector** natively (`crates/crush-services/src/extensions`) — surface that + the rest (postgis, pg_cron, pg_stat_statements, pg_trgm, uuid-ossp, hstore, pgmq, pg_net, citext, etc.).
- For extensions needing a server-side library not present, show the install hint (and, where crush can, build/fetch it like pgvector).
- **Accept:** enable `pgvector` and `pg_stat_statements` from the GUI on a crush PG; see them move to "installed."

## Phase D3 — Performance / advisors
- **pg_stat_statements** panel: top queries by total/mean time, calls (enable the ext if missing).
- **EXPLAIN / EXPLAIN ANALYZE** button in the SQL editor with a readable plan view.
- **Index advisor** (`index_advisor`/`hypopg` extensions) panel: suggest indexes for a query.
- DB size / table sizes / bloat overview.
- **Accept:** run a query → EXPLAIN shows the plan; stat panel lists slow queries.

## Phase D4 — Security (roles, RLS, column security)
- **Roles:** list, create, grant/revoke (`pg_roles`, `GRANT`/`REVOKE`).
- **RLS:** per-table toggle (`ENABLE ROW LEVEL SECURITY`), list/add/drop **policies** with a simple editor.
- **Column privileges:** grant/revoke per column.
- **Accept:** enable RLS on a table, add a policy, see it listed.

## Phase D5 — Functions, triggers, "webhooks"
- List/create/edit **functions** (`pg_proc`) and **triggers** (`pg_trigger`) with a SQL editor.
- **Database webhooks** = a trigger that calls `pg_net`/`http` on change → a small wizard that writes that trigger (enable `pg_net` first via D2).
- **Accept:** create a trigger from the GUI; it appears and fires.

## Phase D6 — Backups (shared with PaaS Phase 5)
- Manual snapshot/restore already exist (`crush db`). Add: **scheduled** backups (host cron / Task Scheduler running `crush db snapshot`), retention, and an **S3 target** (reuse MinIO/S3 client). List + restore + delete from the Database page.
- **Accept:** schedule a daily snapshot; see snapshots; restore one.

## Phase D7 — Queues (pgmq)
- Enable `pgmq` (via D2). UI to **create queue**, **list queues**, **peek/read/archive messages**, depth metrics — all thin wrappers over pgmq functions (`pgmq.create`, `pgmq.send`, `pgmq.read`, `pgmq.archive`, `pgmq.metrics`).
- **Accept:** create a queue, send a test message, see it, archive it.

## Cross-engine notes
- MySQL/MariaDB: D1 (data+SQL) + basic schema; skip PG-only phases (extensions/RLS/pgmq).
- MongoDB: collection browse + find/insert/update via `mongosh`; no SQL tabs.
- Redis: key browser (exists partly in inspectors) + TTL/type + set/del; no SQL.
- Drive the tab set off the engine of the selected connection.

## Per-phase definition of done
Same as Plan #1: Rust builds, svelte-check only-baseline-errors, unit tests for pure parsers/builders (e.g. CSV/result parsing, identifier-quoting, destructive-statement detection), **feature visible+usable in the GUI**, commit per phase, hand to validator for build+test+release. Do not bump versions or release.

## Reliability musts (this is data — be careful)
- Never interpolate identifiers/values unsafely; quote identifiers (`format!("\"{}\"", ident.replace('"', "\"\""))`) and parameterize values.
- Destructive statements: detect (DROP/TRUNCATE/ALTER/DELETE|UPDATE without WHERE) → explicit confirm with the statement shown.
- Every query has a timeout; long queries are cancellable.
- Read paths must tolerate permission errors / missing extensions gracefully (show, don't crash).
- Connection creds are never logged (mirror `LlmClient`'s redacted Debug).
