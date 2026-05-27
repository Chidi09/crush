# pgvector auto-install on Windows (deferred)

## Problem
`pgvector/*` Docker images bundle the `vector` extension. Crush's native
postgres driver uses system PostgreSQL, which doesn't have `vector.dll`.
No trusted prebuilt binary exists anywhere on the public internet
(checked: pgvector releases, EDB Stack Builder XML feed, chocolatey,
community repos). User does not want Docker.

## Chosen path: build-on-first-use via MSVC
One-time setup per machine; subsequent projects are seamless.

## Prerequisites the user installs
```
winget install --id Microsoft.VisualStudio.2022.BuildTools \
  --override "--quiet --wait --norestart \
    --add Microsoft.VisualStudio.Workload.VCTools \
    --add Microsoft.VisualStudio.Component.Windows10SDK.19041"
```
~6 GB.

## Crush code changes
1. **Revert v0.7.15 carve-out**: restore `pgvector` and `timescale` to
   `native_driver_for` (currently they fall through to container).
2. **New module `crush-services/src/extensions/pgvector.rs`**:
   - `fn is_installed(pg_root: &Path) -> bool` — check for
     `pg_root/share/extension/vector.control`.
   - `async fn ensure_installed(pg_root: &Path, cache: &BinaryCache)`:
     - if installed, return Ok
     - locate `cl.exe` via `vswhere.exe`; if missing → clear actionable
       error pointing at the winget command above
     - clone `https://github.com/pgvector/pgvector` at a pinned tag
       (start with `v0.8.0`) into `cache/pgvector-src/`
     - shell out: `vcvars64.bat && set PGROOT=<pg_root> && nmake /F Makefile.win && nmake /F Makefile.win install`
     - the install step needs admin (`C:\Program Files\PostgreSQL\17\lib\` is protected).
       Detect non-admin and `Start-Process -Verb RunAs` an elevated installer step,
       or fail with a one-line "re-run elevated" message.
3. **Hook in `PostgresDriver::start`**: after `pg_ctl start` succeeds,
   if `config.extra_env` contains an image hint of `pgvector/*`, call
   `pgvector::ensure_installed(pg_root, cache)` before returning.
   (Need to pass the source image through `ServiceConfig` — add an
   `image: Option<String>` field.)

## Test cases
- Fresh PG17, no pgvector → first `crush` run builds + installs +
  loads extension; second run is fast (sentinel check).
- MSVC missing → clear error, no half-state.
- Non-admin → either prompt elevation or print exact elevated command.
- Project switches from `postgres` to `pgvector/pgvector` between runs
  → reinit detected, extension installed.

## Fallback if MSVC install is a blocker for a future user
Add an opt-in env var `CRUSH_PGVECTOR_DOCKER=1` that re-routes
pgvector images to the container backend (v0.7.15 behavior). Keep
the carve-out code in a `cfg`-gated path; don't delete it.

## Open question
Long-term we should host our own built binaries (one per PG major)
on the crush releases page so users without MSVC just download. Needs
a Windows builder — GH Actions when account is unblocked, or one
manual build per PG major when convenient.
