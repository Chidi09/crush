# UX backlog

## Multi-service polish (from NCIC run)
- **Child exit announcement**: when a spawned service exits, print
  `✗ backend exited (code 1) — stopping other services` instead of
  letting the raw stderr trail off without a crush-side summary.
  Catch in the "wait for any exit" loop in main.rs — match the exited
  child by handle and look up its name from the original `children` vec.
- **Per-service log prefixes**: tee each child's stdout/stderr through
  a labeling reader that prepends `[backend]` / `[frontend]` to every
  line, turborepo-style. Use `tokio::io::BufReader::lines()` and spawn
  a task per stream. Color the prefix per service for fast visual scan.
- **Better port-not-bound message**: if a service is alive but never
  bound its expected port, suggest checking `PORT` env / framework
  default rather than the current "no response on :X" line.

## `crush export` — generate Dockerfile + compose from a working detection
Crush already knows the runtime, build/install command, entry point, port,
and dep services for the current project. After a successful native run,
offer (or expose as `crush export`):
- A `Dockerfile` — multi-stage, language-appropriate base image, COPY
  source, RUN install_command, CMD/ENTRYPOINT entry_point.
- A `docker-compose.yml` — the app service plus all the dep services
  crush spun up (postgres, redis, etc.) with the same env it injected.
- For multi-service projects: one Dockerfile per service in its subdir
  + a root compose file that wires them with networks + DATABASE_URL.

UX: after `✓ running natively on :X`, ask
`   ↳ export for prod (Dockerfile + compose)? [y/N]`.
Or non-interactive: `crush export --to dist/`.

Rationale: crush isn't widely adopted, so projects still need to deploy
through standard tooling. Crush's value is *speed of local dev* — having
it also emit production artifacts means it never traps users.

Care needed:
- Don't overwrite an existing Dockerfile without prompt.
- Generated files should be small, readable, and idiomatic (not a wall
  of options nobody understands). Add a header comment with the
  detection inputs so users can sanity-check.
- For Python/uv, multi-stage `uv export --format requirements-txt`
  then `pip install -r requirements.txt` produces smaller images
  than copying the venv.

## Visual style (v0.7.24+)
Goal: crush output looks like a polished modern CLI (think vercel /
turbo / vite), not a debug script.

Conventions:
- `↳`   dim cyan         — progress arrows
- `✓`   bold green       — success
- `✗`   bold red         — failure
- `⚠`   bold yellow      — warning / partial
- service / app names    bold
- ports                  cyan
- file paths             dim
- elapsed times          dim italic
- section headers (rare) bold + uppercase or with a leading rule

Implementation: add `owo-colors` to crush-cli. Wrap all `println!` /
`print!` in detect, multi-service, and run sections. Auto-detect TTY
via `std::io::IsTerminal` so piped output stays plain.
