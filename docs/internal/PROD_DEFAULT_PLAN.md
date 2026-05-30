# Plan: flip crush default to production mode (v0.7.29 / "v8" milestone)

## Decision
`crush` (no flag) → production-style local run (build → serve artifact).
`crush --dev` → existing HMR / hot-reload behaviour.

Rationale: crush's identity (`crushed to image`, `crush eject` → Dockerfile)
already screams "I run prod locally". The runtime has quietly been doing
`npm run dev` for Node which contradicts the framing. Flipping it makes
the eject-to-Dockerfile output behaviour-identical to what users see locally,
which is the whole pitch.

---

## Per-stack matrix

| Stack            | Prod (default)                                          | Dev (`--dev`)                            | Notes                                                       |
|------------------|---------------------------------------------------------|------------------------------------------|-------------------------------------------------------------|
| **Node + Vite**  | `npm run build` → `npm exec vite preview -- --port $PORT --host 0.0.0.0` | `npm run dev`                            | `vite preview` is built-in, no extra install                |
| **Next.js**      | `npm run build` → `npm run start` (or `next start -p $PORT`)              | `next dev`                               | `next start` reads `.next/` from build                      |
| **Nuxt**         | `npm run build` → `npm run preview` (or `node .output/server/index.mjs`)  | `nuxt dev`                               | Nuxt 3 emits a node server                                  |
| **SvelteKit**    | `npm run build` → `node build/index.js`                                   | `vite dev`                               | Adapter-node assumed                                        |
| **Remix**        | `npm run build` → `npm run start`                                         | `remix dev`                              |                                                             |
| **Astro**        | `npm run build` → `npm run preview`                                       | `astro dev`                              | SSG default; SSR if adapter present                         |
| **Node API** (Fastify/Express/NestJS) | `npm run build` → `npm run start` (often `node dist/index.js`) | `tsx watch src/index.ts` etc. (`npm run dev`) | Build step needed only if TS                                |
| **Bun**          | `bun build ./src/index.ts --outfile dist/app` → `bun dist/app`            | `bun --hot ./src/index.ts`               |                                                             |
| **Deno**         | `deno compile` (optional) or `deno task start`                            | `deno task dev`                          | Most repos drive via `deno.json` tasks                      |
| **Go**           | `go build -o ./bin/$name .` → `./bin/$name`                               | `go run .`                               | Compile is fast; binary path → cwd-relative                 |
| **Rust**         | `cargo build --release` → `target/release/$name`                          | `cargo run`                              | Parse name from `[[bin]]` or use crate name                 |
| **Java/Spring**  | `mvn -B package -DskipTests` → `java -jar target/*.jar`                   | `mvn spring-boot:run -Dmaven.test.skip=true` | Resolve glob to the actual fat jar; skip *-sources.jar  |
| **Java/Gradle**  | `gradle bootJar -x test` → `java -jar build/libs/*.jar`                   | `gradle bootRun -x test`                 |                                                             |
| **.NET**         | `dotnet publish -c Release -o out` → `dotnet out/$proj.dll`               | `dotnet watch run`                       | `dotnet run` mid-ground; pick publish for true prod parity  |
| **Python uv + FastAPI** | `.venv/bin/uvicorn src.core.main:app --host 0.0.0.0 --port $PORT --workers 2` | `... --reload`                       | Skip `--workers` on Windows (only one worker supported)     |
| **Python Django**| `python manage.py collectstatic --noinput && gunicorn $name.wsgi -b 0.0.0.0:$PORT` | `python manage.py runserver 0.0.0.0:$PORT` | gunicorn needs `pip install gunicorn` first             |
| **Python Flask** | `gunicorn $module:app -b 0.0.0.0:$PORT`                                   | `flask --app $module run --host=0.0.0.0` | Same gunicorn caveat                                        |
| **Ruby/Rails**   | `bundle exec rails assets:precompile && RAILS_ENV=production bundle exec rails server -b 0.0.0.0 -p $PORT` | `bundle exec rails server -b 0.0.0.0 -p $PORT` (RAILS_ENV=development) |                                                             |
| **PHP/Laravel**  | `composer install --no-dev --optimize-autoloader && php artisan config:cache && php artisan serve --host=0.0.0.0 --port=$PORT` | `php artisan serve --host=0.0.0.0 --port=$PORT` | Real prod usually wants nginx+fpm, but `artisan serve` is fine for local prod sim |
| **Phoenix**      | `MIX_ENV=prod mix release` → `_build/prod/rel/$name/bin/$name start`      | `mix phx.server`                         | Release tar is the prod path                                |

---

## Implementation

### 1. CLI: add `--dev` flag

```rust
// In Cli struct (top-level args)
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long, short = 'D', help = "Run dev-mode (HMR / watch / reload) instead of prod")]
    dev: bool,
    // ... existing fields
}
```

Pass `cli.dev` into the run path. Single source of truth.

### 2. Detection produces TWO entry points

Easiest path: extend `Detection` (and `InferredStack`) with a second field:

```rust
pub struct Detection {
    // ... existing
    pub entry_point: String,        // PROD command (default)
    pub dev_entry_point: String,    // HMR / watch command
    pub build_command: String,      // PROD prep step (install + compile)
    pub dev_install_command: String,// DEV prep step (usually just install)
}
```

Each `try_*` detector populates both. `infer_node_entry` already knows the
build script; just split it: `build` → `entry_point = build && start`,
`dev` → `dev_entry_point = npm run dev`.

For multi-service: same on `SubService` — store both, pick at spawn time.

### 3. main.rs run path

```rust
let entry = if cli.dev { &stack.dev_entry_point } else { &stack.entry_point };
let install = if cli.dev { &stack.dev_install_command } else { &stack.build_command };
```

Same for multi-service spawn loop.

### 4. Build step needs progress UX

Prod runs do a real build (mvn package, cargo build --release, npm run build,
go build, etc.). These take 30s–3min. Need:
- A `⚙ building backend...` line with elapsed time
- Stream child output (already prefixed via [name] logger)
- If build fails, kill siblings and report cleanly with the existing exit-announce
- Maybe skip the build if the artifact is fresh? cache key = source hash + last build time

### 5. Port handling

Many prod servers want `PORT` env var set; some want a flag. Keep both:
- Always set `PORT` env
- For commands that don't pick it up, pass via flag (`-p $PORT`, `--port=$PORT`)

### 6. Ready panel changes

In prod mode, log filter should default to **OnlyErrors** (current behaviour).
In dev mode, default to **All** — devs want to see HMR reloads and compile output
streaming.

---

## Edge cases / open questions

1. **First-time build is slow**. Should `crush` print a one-liner "first prod
   build takes longer, use `crush --dev` for iteration"?
2. **Existing build artifacts**. If `dist/` / `target/` / `.next/` is already
   present and fresh, skip the build step? Risk: stale builds. Mitigation:
   compare source mtime vs artifact mtime. Cheap.
3. **`npm run build` produces SPA only, no server**. Need a static server.
   Options: `vite preview` (built-in to Vite), `serve` package (adds install),
   or roll a tiny static server in crush itself (Rust + tokio, 50 LOC).
   **Decision: prefer the framework's preview command. Only fall back to a
   bundled static server if the project has no preview script.**
4. **Multi-service with mixed dev/prod**: should it be possible to run
   backend prod + frontend dev? Probably not in v0.7.29 — global flag only.
   Add `--dev frontend` per-service later if asked.
5. **Auto-rebuild on source change**? No. That's what `--dev` is for. Prod
   mode is one-shot: build, run until killed.
6. **Database deps**: no change — they always run the same way regardless
   of dev/prod.
7. **`crush eject`** should generate from the prod commands, not dev ones —
   make this explicit by passing `is_prod=true` to the generator.

---

## Testing checklist after implementation

- [ ] `crush` in NCIC (Go + Vite) → backend `go build` then `./bin/...`,
      frontend `npm run build` then `vite preview --port 5173`. Should
      hit Ready panel.
- [ ] `crush --dev` in NCIC → current behaviour (go run, vite dev).
- [ ] `crush` in safe-meet (turbo + Next + Fastify) → `pnpm build` then
      `pnpm start`. Backend with Prisma might need `prisma generate` first
      (already in their `prebuild` script).
- [ ] `crush` in gazillion-be-staging (FastAPI + uv) → uvicorn no-reload
      with workers.
- [ ] `crush` in Solexpay (Spring Boot) → `mvn package` then `java -jar`.
- [ ] `crush --dev` for each — confirm HMR still works.
- [ ] `crush eject` → Dockerfile uses prod commands (not dev).

---

## Out of scope (deliberate)

- Hot-rebuild watcher on prod mode
- Per-service dev/prod override
- Automatic minification beyond what the framework does
- Compressing built artifacts
- Caching builds across `crush` invocations (we already cache the source tarball)

These can land later if there's pull.

---

## Migration / breaking change note

This is a default behaviour flip. Document in the release notes:

> v0.7.29 changes `crush`'s default from dev-mode to prod-mode. If you were
> using `crush` for hot-reload iteration, switch to `crush --dev` to keep the
> old behaviour. The new default builds and serves your app the way it would
> deploy, matching `crush eject`'s output.
