# GUI Fixes Plan — crush-gui robustness pass

Context for the executing agent: a functional review of `crates/crush-gui`
found two bugs that break core flows (items 1–2), one launch-blocker for
non-root Linux users (item 3), and two robustness gaps (items 4–5). Fix them
in this order. Each item lists the evidence, the fix, and acceptance criteria.
Do not refactor beyond what each item requires; match the existing code style
of each file.

Relevant layout:
- Tauri backend: `crates/crush-gui/src-tauri/src/` (commands/, state.rs, events.rs, lib.rs)
- Frontend: `crates/crush-gui/src/` (lib/tauri.ts is the typed command/event contract)
- Run engine the GUI calls into: `crates/crush-build/src/run.rs` (`RunHandle`, `RunEvent`)

Verification baseline before starting: `bash scripts/ci-local.sh --fast` must
pass (it does today). It must still pass after every item.

---

## 1. Run events never reach the frontend (emit/listen name mismatch)

**Bug.** The backend emits per-kind event names:
`crates/crush-gui/src-tauri/src/commands/run_cmd.rs:47`
```rust
let _ = emit_window.emit(&format!("run-event::{}::{}", run_id, kind), &event);
```
but the only frontend listener subscribes without the kind suffix:
`crates/crush-gui/src/lib/tauri.ts:206`
```ts
return listen<RunEvent>(`run-event::${runId}`, (e) => cb(e.payload));
```
Tauri event matching is exact (no wildcards), so `onRunEvent` receives
nothing: no `detected`, no `build-output`, no `port-bound`, no `exited`.

**Fix.**
- Emit on `run-event::{run_id}` (drop the `::{kind}` suffix) and make sure the
  payload carries the kind discriminant, because the TS types
  (`tauri.ts` `RunEvent` union) expect a `kind: 'build-output' | ...` field.
- Check the serde attributes on `crush_build::run::RunEvent`: it must
  serialize as an internally-tagged enum with kebab-case kinds, i.e.
  `#[serde(tag = "kind", rename_all = "kebab-case")]` — variant names like
  `BuildOutput` must serialize to `"build-output"` to match the TS union. If
  the attribute is missing or different, add/adjust it on the enum (this is a
  shared type — run the full workspace check after).
- The `aborted` emission in the same file (line ~53) must follow the same
  scheme: same event name, payload `{ "kind": "aborted" }` (add the variant to
  the TS union).
- The `event_name()` helper in run_cmd.rs becomes dead code once the suffix is
  gone — delete it (or keep it only if used for logging).

**Accept.** A `run_project` invocation from the GUI shows live build output,
the port-bound URL, and the exit event in the run drawer. Add/keep a TS-side
type for every Rust variant. `cargo check --workspace --all-targets` clean.

---

## 2. Abort doesn't kill the process

**Bug.** `crush_build::run::RunHandle` exposes the real abort channel
(`run.rs:152-156`); the inner run loop polls it (`run.rs:1226`,
`abort_rx.try_recv().is_ok()`). But `run_cmd.rs:29` creates a *new* oneshot
channel, stores that one in `AppState.runs`, and lets `handle.abort` drop.
Dropping a oneshot Sender makes `try_recv()` return `Err`, not `Ok`, so the
runner never aborts. Clicking Stop in the UI emits an "aborted" event and
stops the forwarder, while the build/app process keeps running.

**Fix.**
- Store the real sender: `RunProcess { abort: handle.abort }` (adjust the
  destructuring at the top of `run_project`; currently only `run_id` and
  `events` are taken out of the handle).
- `abort_run` sends on it (already does — it just has the wrong sender today).
- The event forwarder no longer needs its own abort arm: after a successful
  abort the run loop emits `Exited` (verify in run.rs — if it does not emit an
  event on abort-break, emit the `aborted` UI event from `abort_run` itself
  after sending). The forwarder should exit when the events channel closes or
  `Exited` arrives, then clean up the `runs` map entry as it does now.

**Accept.** Start a long-running project from the GUI, click Stop: the child
process tree is actually gone (verify with `pgrep` on the dev server command)
and the UI reflects the stop.

---

## 3. Non-root Linux users can't launch the app

**Bug chain.** `crates/crush-gui/src-tauri/src/state.rs:31` hardcodes
`/var/lib/crush` on Linux → `create_dir_all(&base).ok()` swallows the
permission error → `lib.rs:19-23` `ImageStore::new(...).expect(...)` panics in
`setup` → the app dies on launch with no visible error.

**Fix.**
- First check how the crush CLI resolves its data dir on Linux (search
  crush-cli / crush-types for the data-dir logic and any `CRUSH_DATA_DIR` env
  override). The GUI lists containers/services/images from this directory, so
  GUI and CLI must agree or the GUI shows an empty world. Extract or reuse one
  shared resolution helper rather than duplicating the policy.
- Resolution order on Linux: honor the existing env override if there is one;
  use `/var/lib/crush` only when it exists *and* is writable; otherwise fall
  back to `dirs::data_dir().join("crush")` (matches the macOS branch).
- Replace the `expect` in `lib.rs` setup with a graceful failure: show a
  native error dialog (tauri dialog plugin or a minimal error window) naming
  the path that failed, then exit cleanly.

**Accept.** As a non-root user with no `/var/lib/crush`, the app launches and
uses the user data dir. As root on a box where the CLI already populated
`/var/lib/crush`, the GUI still sees the same containers/services as before.

---

## 4. Log replay floods the UI on large logs

**Bug.** `crates/crush-gui/src-tauri/src/commands/logs.rs:36-45`
(`subscribe_logs`) reads the entire stdout/stderr log files into memory and
emits every historical line as an individual Tauri event. A long-running
chatty service produces 100k+ events on subscribe and freezes the webview.

**Fix.**
- Replay only the last N lines (N = 500) per stream: seek to EOF, read a
  bounded tail window (e.g. read the final 256 KB and take the last 500
  lines), set the offset to EOF, then let the existing 200 ms incremental
  poll loop take over unchanged.
- Batch the replay: emit the tail as one event carrying an array of lines
  (add a `log-replay::{container_id}` event + TS listener), or at minimum
  chunk emissions. One event per historical line is the thing to eliminate.
- While in the file: the initial-offset bookkeeping currently uses
  `content.len()` from `read_to_string` — byte length of a lossily-decoded
  string can drift from the file's byte length on invalid UTF-8. Use file
  metadata/seek positions for offsets (the incremental loop already does).

**Accept.** Subscribing to a container with a multi-MB log renders the tail
instantly; new lines still stream live. No change to small-log behavior.

---

## 5. GUI can only run prod mode

**Gap.** `run_cmd.rs:12-22` hardcodes `RunOptions { dev: false, ... }`. The
CLI's whole detection philosophy prefers dev workflows (HMR) for local runs;
the GUI can't express it at all.

**Fix.**
- Add a `dev: bool` parameter to the `run_project` command (Tauri camelCases
  it to `dev` on the TS side; default false to stay backward compatible) and
  pass it into `RunOptions`.
- Update `tauri.ts` `runProject(projectPath: string, dev = false)`.
- Add the UI control where runs are launched (the project run flow — locate
  the component calling `runProject`; likely the dashboard or build page): a
  simple "Dev mode (hot reload)" toggle, persisted per project if a settings
  store already exists, otherwise component-local state is fine for now.

**Accept.** Running with the toggle on launches the project's
`dev_entry_point` (visible in the streamed command/output), toggle off keeps
today's behavior.

---

## Final verification (all items)

1. `bash scripts/ci-local.sh --fast` — green.
2. `cargo check --workspace --all-targets --all-features` — no errors, no new warnings.
3. `cd crates/crush-gui && pnpm build` — frontend compiles (type errors in
   tauri.ts surface here).
4. Manual smoke (needs a display or `xvfb-run`): `pnpm tauri dev`, run a small
   sample project, watch live events (item 1), stop it (item 2), open its logs
   (item 4), toggle dev mode (item 5).
5. Do NOT run `pnpm tauri build` (release bundling) as part of this task — it
   is slow and covered by the release pipeline.
