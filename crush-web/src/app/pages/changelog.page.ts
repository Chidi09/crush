import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmBadgeDirective } from '../ui/badge';

@Component({
  selector: 'page-changelog',
  standalone: true,
  imports: [RouterLink, HlmBadgeDirective],
  template: `
    <div class="mx-auto max-w-4xl px-4 py-12 sm:px-6 lg:px-8">
      <header class="mb-16 select-none border-b border-crush-border/30 pb-8">
        <h1 class="text-4xl font-extrabold text-white mb-3">Changelog</h1>
        <p class="text-lg text-crush-textMuted">
          Live release logs, performance enhancements, and system design history for Crush.
        </p>
      </header>

      <div class="space-y-12">
        @for (release of releases; track release.version) {
          <div class="border-l-2 border-crush-border/50 pl-6 sm:pl-8 pb-12 relative last:pb-0">
            <!-- Pulsing Timeline Node -->
            <div
              class="absolute left-[-9px] top-1.5 h-4 w-4 rounded-full border-2 border-crush-orange bg-crush-black shadow-[0_0_12px_rgba(224,85,64,0.3)]"
            ></div>
            
            <div class="flex items-center gap-4 mb-4 flex-wrap">
              <h2 class="text-2xl font-bold text-white font-mono leading-none">{{ release.version }}</h2>
              <span
                class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold border border-crush-orange/30 bg-crush-orange/10 text-crush-orange"
              >
                {{ release.date }}
              </span>
            </div>
            
            <p class="text-sm text-crush-textMuted mb-4 italic" *ngIf="release.headline">
              {{ release.headline }}
            </p>

            <ul class="space-y-3">
              @for (item of release.items; track item) {
                <li class="text-sm sm:text-base text-slate-300 flex items-start gap-2.5 leading-relaxed">
                  <span class="text-crush-orange mt-1.5 shrink-0 w-1.5 h-1.5 rounded-full bg-crush-orange"></span>
                  <span [innerHTML]="item"></span>
                </li>
              }
            </ul>
          </div>
        }
      </div>
    </div>
  `,
})
export default class ChangelogPage implements OnInit {
  releases = [
    {
      version: 'v0.8.0-alpha',
      date: 'In Development',
      headline: 'Preparation series for the cross-platform Tauri desktop dashboard.',
      items: [
        'Scaffolded Tauri 2 + SvelteKit desktop app GUI shell (`crates/crush-gui/src-tauri`).',
        'Implemented Tauri command bindings for native process lifecycle control, image stores, and system configurations.',
        'Wired real backends including `crush-api` unix sockets, `crush-proto` OCI gateway, and `crush-tui` sparklines.',
        'Created a high-fidelity brand assets generator to avoid `windres` resources compilation errors during Windows GUI builds.',
      ],
    },
    {
      version: 'v0.7.74',
      date: '2026-05-28',
      headline: 'Restores four output lines that v0.7.73\'s run_project refactor dropped against v0.7.72:',
      items: [
        '`↳ dependencies layer cached (unchanged)` after image-fresh',
        '`✓ crushed to image <tag>:latest (0 MB)` headline on warm runs',
        '`✓ dependencies fresh — node_modules newer than lockfile (--rebuild to force)` when the install step is skipped',
        '`↳ warm run — launching` with the correct cyan info icon (was incorrectly emitted as a yellow `⚠` warning)',
        'Added two structured RunEvent variants (`WarmRun`, `DepsFresh`) so the GUI gets these signals too, not just the CLI. Also dropped an unused speculative `Line` variant that wasn\'t compiling.',
        'CLI behaviour vs v0.7.72: identical on warm runs.',
      ],
    },
    {
      version: 'v0.7.73',
      date: '2026-05-28',
      headline: 'The Commands::Default flow (~1200 lines) was extracted from `crush-cli/src/main.rs` into `crush_build::run::run_project()`. CLI behaviour is identical — `println!` calls became `tx.send(RunEvent::…)` and the CLI\'s Default arm became a thin event-printing consumer.',
      items: [
        'This is the prep enabler for the v0.8 GUI. The GUI will consume the same `RunEvent` stream via `tauri::Window::emit` instead of stdout.',
        'No user-visible changes for CLI users.',
      ],
    },
    {
      version: 'v0.7.72',
      date: '2026-05-28',
      headline: 'Pre-work for the v0.8 GUI implementation. CLI behavior unchanged for normal usage.',
      items: [
        '*New:**',
        'crush_build::run module with stable types (RunEvent, RunOptions, BuildRecord) the GUI consumes',
        'data_dir/build-history.json written on every build outcome (cap 200, atomic rename)',
        '\'crush history\' subcommand reads it (text or --format json)',
        '\'crush ps --format json\' emits the container list as JSON',
        '\'crush services ps --format json\' emits per-project native + container state. Add --all-projects to span every project',
        '\'crush images --format json\' emits the image list',
        'GUI_DATA_CONTRACT.md documents stable file paths, schemas, polling cadence, anti-fragility rules',
        '*Not yet:**',
        'The crush_build::run::run_project() function body — that\'s the next agent\'s first task. Types and target signature are pinned in run.rs and CRUSH_V8_PLAN.md so they don\'t have to design the API.',
      ],
    },
    {
      version: 'v0.7.71',
      date: '2026-05-28',
      headline: 'Sets PYTHONUTF8=1 (PEP 540) and PYTHONUNBUFFERED=1 when crush spawns a Python app:',
      items: [
        'PYTHONUTF8=1 forces UTF-8 for stdout/stderr and open() regardless of system locale. Fixes Windows cp1252 UnicodeEncodeError when apps print emoji or non-ASCII (e.g. gazillion-be-staging crashed printing \'🔧 Loading configuration...\').',
        'PYTHONUNBUFFERED=1 flushes prints immediately instead of buffering when stdout isn\'t a TTY. Log lines appear as they\'re emitted.',
      ],
    },
    {
      version: 'v0.7.70',
      date: '2026-05-28',
      headline: 'Implements PGVECTOR_PLAN.md. When a project asks for pgvector/pgvector:* and Docker isn\'t available, crush now builds the extension against the host PostgreSQL using MSVC and installs it before the app connects.',
      items: [
        'Requires Visual Studio Build Tools 2022 with the \'Desktop development with C++\' workload + Windows SDK. Install with:',
        'winget install --id Microsoft.VisualStudio.2022.BuildTools --override \"--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows10SDK.19041\"',
        'The pgvector source is cloned at the pinned tag v0.8.0 into your crush cache and the build only runs once per PG install (idempotent via vector.control check). The install step writes into PostgreSQL\'s lib/ and share/extension/ — if PG is under Program Files this requires an elevated terminal.',
      ],
    },
    {
      version: 'v0.7.69',
      date: '2026-05-28',
      headline: 'Cosmetic cleanup of the message printed when a python project has no requirements.txt / pyproject.toml. Just a follow-up to v0.7.68.',
      items: [
      ],
    },
    {
      version: 'v0.7.68',
      date: '2026-05-28',
      headline: 'Found while running crush on project_approval_system_root: Django detection (v0.7.67) succeeded but the build step failed because there\'s no requirements.txt — deps were installed once into .venv and the manifest forgotten.',
      items: [
        'Skip install step when .venv is present but no requirements.txt / pyproject.toml exists',
        'Use .venv/Scripts/python (Windows) / .venv/bin/python (Unix) for Django entry when .venv exists. Bare \'python\' on PATH is a different interpreter',
        'Drop \'collectstatic\' from default Django build — runserver serves statics with DEBUG=True. Add back via Crushfile if your prod flow needs it',
      ],
    },
    {
      version: 'v0.7.67',
      date: '2026-05-28',
      headline: 'Fixes found while running detect across 12 real projects:',
      items: [
        '**Django detection** fires on manage.py alone — no longer hijacked by a stray package.json at the same root.',
        '**Script-driven port resolution** — if your dev/start script literally invokes vite / nuxt / next / astro / ng serve, that tool\'s port wins over the framework signal. Fixes Angular-with-vite (Solexpay-frontend) reporting 4200 when the server is actually on 5173.',
        '**$PORT translation** — Windows spawn_shell now rewrites bash $VAR and ${VAR} to cmd.exe %VAR%. Fixes \'uvicorn --port $PORT\' on FastAPI/Python projects.',
      ],
    },
    {
      version: 'v0.7.66',
      date: '2026-05-28',
      headline: 'Extends the docker-shape heuristic (v0.7.48+) to Java/Maven/Gradle. When no Dockerfile/compose exists in the repo, crush now treats it as a dev workflow and:',
      items: [
        'Maven: \'mvn -B compile\' + \'mvn spring-boot:run\' (was: package + java -jar)',
        'Gradle: \'gradle classes\' + \'gradle bootRun\' (was: bootJar + java -jar)',
        'For Solexpay-backend this skips the ~60s \'mvn package\' (jar build + repackage) and runs Spring directly. If you add spring-boot-devtools to pom.xml, subsequent code changes trigger ~3s hot restarts instead of full boot.',
        'Repos with a Dockerfile/compose keep the package + java -jar flow (prod-shape).',
      ],
    },
    {
      version: 'v0.7.65',
      date: '2026-05-28',
      headline: '',
      items: [
        '*.crushignore** — one pattern per line, # for comments. Extends the built-in skip list (node_modules, target, dist, etc.) for the fingerprint and mtime walks. Real win on huge repos with non-standard build outputs:',
        'storybook-static/',
        'coverage/',
        '.nx/cache/',
        'legacy-vendor',
        '*--priority high | above-normal** — sets the Job Object PriorityClass on Windows so crush\'s process tree runs at a higher scheduling priority. Useful when the system is under load. Silently ignored on non-Windows.',
      ],
    },
    {
      version: 'v0.7.64',
      date: '2026-05-28',
      headline: 'springdoc-openapi lazy-inits on first request (~4s) which exceeds the 700ms probe timeout, causing an AsyncRequestNotUsable stack trace. swagger-ui (which we already probe) is the user-facing URL and back-links to api-docs internally.',
      items: [
        'Remaining noise on Solexpay is the app\'s own /actuator/health spawning a mail health check with no SMTP creds — not crush. Silence with \'management.health.mail.enabled=false\' in application.yml.',
      ],
    },
    {
      version: 'v0.7.63',
      date: '2026-05-28',
      headline: 'Spring Boot, FastAPI, NestJS, Express, and Fastify now get a narrower probe list (only paths each framework actually exposes) plus the SPA-shell \'/\' fingerprint is skipped for these backends. Cuts the 5-10 stack traces per crush run.',
      items: [
        'Unknown stacks still get the full 12-path probe so we don\'t miss anything on projects we don\'t recognise.',
      ],
    },
    {
      version: 'v0.7.62',
      date: '2026-05-28',
      headline: 'v0.7.61\'s CREATE DATABASE step used psql\'s \gexec meta-command via -c, which is treated as part of the SQL string and silently errored as a syntax error. Now uses plain CREATE DATABASE and ignores the \'already exists\' error.',
      items: [
      ],
    },
    {
      version: 'v0.7.61',
      date: '2026-05-28',
      headline: 'After postgres starts, run an idempotent CREATE/ALTER ROLE block so configured creds win even when the data dir was init\'d with different ones by an earlier crush version.',
      items: [
      ],
    },
    {
      version: 'v0.7.60',
      date: '2026-05-28',
      headline: 'v0.7.59 wrapped JAVA_HOME path in quotes unconditionally. cmd.exe doubly-escaped the quotes Tokio adds, producing literal \"path\" that fails to resolve. Now: skip quoting entirely when the path is space-free (scoop/sdkman/jenv installs).',
      items: [
      ],
    },
    {
      version: 'v0.7.59',
      date: '2026-05-28',
      headline: 'Fixes UnsupportedClassVersionError when the project\'s required JDK is newer than the bare \'java\' on PATH. Now spawn_shell rewrites a leading \'java \' to \'$JAVA_HOME/bin/java \' when JAVA_HOME is set — same JDK Maven uses, so class versions match.',
      items: [
        'Falls back to bare \'java\' when JAVA_HOME is unset or the binary doesn\'t exist.',
      ],
    },
    {
      version: 'v0.7.58',
      date: '2026-05-28',
      headline: 'cmd.exe doesn\'t expand glob patterns, so \'java -jar target/*.jar\' failed with \'Unable to access jarfile target/*.jar\' even after a successful Maven build.',
      items: [
        'spawn_shell now resolves the glob to a concrete jar path before exec, skipping Spring Boot\'s *.jar.original (the pre-repackage artifact).',
      ],
    },
    {
      version: 'v0.7.57',
      date: '2026-05-28',
      headline: 'Switch Java/Maven build from \'-DskipTests\' (skips only execution) to \'-Dmaven.test.skip=true\' (skips compile AND execution).',
      items: [
        'Unblocks projects where tests reference stale/missing symbols but the actual app code is fine — common on feature branches where tests haven\'t caught up.',
      ],
    },
    {
      version: 'v0.7.56',
      date: '2026-05-28',
      headline: 'Multi-service runs already probed /swagger-ui, /v3/api-docs, /healthz, /graphql etc. Single-service runs (a plain backend API) didn\'t.',
      items: [
        'Now after a single-service binds, crush probes the same well-known paths and surfaces hits in the open: section. SPA-shell fingerprinting still filters out frontend false positives.',
        'Probes (first hit per category wins):',
        'docs: /swagger-ui/index.html, /swagger-ui.html, /swagger, /docs, /api-docs',
        'openapi: /v3/api-docs, /openapi.json',
        'health: /actuator/health, /health, /healthz',
        'graphql: /graphql',
      ],
    },
    {
      version: 'v0.7.55',
      date: '2026-05-28',
      headline: 'When image is fresh AND node_modules is current, crush no longer asks \'run it now? [Y/n]\' — it just runs. The prompt is friction on warm reruns where you obviously came here to run the app.',
      items: [
        'Cold runs and first builds still show the prompt for user verification.',
      ],
    },
    {
      version: 'v0.7.54',
      date: '2026-05-28',
      headline: 'Crush was still running \'pnpm install\' on warm runs even when node_modules was up to date — paying ~10s for pnpm\'s \'Already up to date\' check.',
      items: [
        'Now: when install command is just \'<pm> install\' (no \'&& build\'), crush compares node_modules mtime to lockfile mtime. If node_modules is newer, install is skipped entirely.',
        'Applies to pnpm/npm/yarn/bun/deno. Override with --rebuild.',
      ],
    },
    {
      version: 'v0.7.53',
      date: '2026-05-28',
      headline: '',
      items: [
        'Vite default port corrected from 3000 to 5173 (was causing \'no response on :3000\' false alarm)',
        'AnalogJS detection via @analogjs/platform/@analogjs/router (port 5173)',
        'Angular detection via @angular/core (port 4200)',
      ],
    },
    {
      version: 'v0.7.52',
      date: '2026-05-28',
      headline: 'When `crush` lands on the generic fallback (no detectable stack) and theres no entrypoint.sh, it now scans immediate subdirs for project markers (package.json, Cargo.toml, go.mod, pyproject.toml, etc) and suggests `cd <subdir> && crush` instead of trying to spawn a missing entrypoint.sh.',
      items: [
        'Replaces the cryptic `entrypoint.sh is not recognized as an internal or external command` error you would otherwise see.',
      ],
    },
    {
      version: 'v0.7.51',
      date: '2026-05-28',
      headline: 'Real fix for the standalone-frontend dev workflow.',
      items: [
        'In v0.7.48 the docker-shape heuristic applied only to the fallback entry path. But Nuxt/Vite/Next/Astro/Remix/SvelteKit each set entry_prod explicitly (e.g. `pnpm run preview` for Nuxt), masking the heuristic.',
        'Now: no prod-shape Docker artifacts -> entry_prod falls back to dev_entry (`pnpm run dev`) regardless of framework, and the build step skips `pnpm run build` since dev servers compile on the fly.',
      ],
    },
    {
      version: 'v0.7.50',
      date: '2026-05-28',
      headline: '',
      items: [
        '**crush watch** on Windows now exits with a friendly hint pointing to `crush --watch` (native flow). The old subcommand used Linux overlayfs which broke on Windows.',
        '**TUI Compose tab** now shows real running native dep services from `<data_dir>/native/*.json` (postgres, redis-compat, mysql) with live pid probing. Replaces the hardcoded web/db/cache mock data.',
      ],
    },
    {
      version: 'v0.7.49',
      date: '2026-05-28',
      headline: '',
      items: [
        '**eject marker**: `crush eject` writes a `# crush:eject` marker on line 1 of Dockerfile and docker-compose.yml. The dev/prod heuristic skips marked files, so `crush eject` followed by `crush` still runs dev mode (HMR). Delete the marker line to adopt the generated file as your real prod config.',
        '**ps empty-state**: `crush ps` with no containers prints inline instead of flashing the alt-screen TUI and leaving no trace in scrollback.',
      ],
    },
    {
      version: 'v0.7.48',
      date: '2026-05-28',
      headline: 'Node entry picks dev vs start based on whether the repo ships prod-shape Docker artifacts.',
      items: [
        'Repo with Dockerfile or compose (in /, infra/, docker/, etc.) -> prefer **start** (match what docker would run).',
        'Repo without -> prefer **dev** (match native pnpm dev workflow with HMR).',
        'Applies to both monorepo and standalone Node projects. Fixes standalone Nuxt/Next/Vite repos running prod preview when the dev wanted HMR.',
      ],
    },
    {
      version: 'v0.7.47',
      date: '2026-05-28',
      headline: '## Speed & ergonomics — six units shipped',
      items: [
        '**Unit 1 — Warm-run cache**: skip image pack when sources unchanged (content fingerprint via mtimes)',
        '**Unit 2 — Parallel builds**: sub-services build concurrently, capped by semaphore (min cpus, 4)',
        '**Unit 3 — Parallel deps**: dep services start in parallel; Garnet binary prefetched in background',
        '**Unit 4 — Lifecycle commands**: `crush ps`, `crush logs`, `crush stop`, `crush restart`',
        '**Unit 5 — Resource limits**: `--memory` and `--cpus` flags via Windows Job Object controls',
        '**Unit 6 — Watch mode**: `--watch` reruns the affected sub-service on source change',
        'Unit 7 (crushd daemon) deferred — see SPEED_PLAN.md.',
      ],
    },
    {
      version: 'v0.7.41',
      date: '2026-05-28',
      headline: 'feat(proxy): built-in reverse proxy on localhost:8000 for dev/prod URL parity. Routes /api → backend, / → frontend. WebSocket upgrade (HMR) preserved. --no-proxy disables.',
      items: [
      ],
    },
    {
      version: 'v0.7.40',
      date: '2026-05-28',
      headline: 'feat(windows): Job Object with KILL_ON_JOB_CLOSE — no more orphaned processes when crush exits',
      items: [
      ],
    },
    {
      version: 'v0.7.39',
      date: '2026-05-28',
      headline: 'fix: tighten compose dep classifier (no more blue/green app services as deps) + bump Garnet to v1.1.9',
      items: [
      ],
    },
    {
      version: 'v0.7.38',
      date: '2026-05-28',
      headline: 'fix(compose): rewrite subdir search + add CRUSH_DEBUG_COMPOSE',
      items: [
      ],
    },
    {
      version: 'v0.7.37',
      date: '2026-05-28',
      headline: 'fix: compose discovery in infra/docker subdirs + monorepo prod entry prefers start over dev',
      items: [
      ],
    },
    {
      version: 'v0.7.36',
      date: '2026-05-28',
      headline: 'fix(probe): drop SPA-shell false positives from service-link probe (no more bogus /swagger-ui on Vite frontends)',
      items: [
      ],
    },
    {
      version: 'v0.7.35',
      date: '2026-05-27',
      headline: 'feat(cache): skip rebuilds when artifacts are newer than sources (Node/Go/Java). --rebuild forces a build.',
      items: [
      ],
    },
    {
      version: 'v0.7.34',
      date: '2026-05-27',
      headline: 'feat(urls): scrape http://localhost URLs from child stdout into Ready panel — surfaces secondary listeners and turbo/monorepo child URLs',
      items: [
      ],
    },
    {
      version: 'v0.7.33',
      date: '2026-05-27',
      headline: 'fix(windows): rewrite ./ prefix in spawn_shell + name go binary .exe so cmd.exe resolves the built artifact correctly',
      items: [
      ],
    },
    {
      version: 'v0.7.32',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Full Changelog**: https://github.com/Chidi09/crush/compare/v0.7.31...v0.7.32',
      ],
    },
    {
      version: 'v0.7.31',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Full Changelog**: https://github.com/Chidi09/crush/compare/v0.7.30...v0.7.31',
      ],
    },
    {
      version: 'v0.7.30',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Full Changelog**: https://github.com/Chidi09/crush/compare/v0.7.29...v0.7.30',
      ],
    },
    {
      version: 'v0.7.29',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Full Changelog**: https://github.com/Chidi09/crush/compare/v0.7.28...v0.7.29',
      ],
    },
    {
      version: 'v0.7.28',
      date: '2026-05-27',
      headline: 'fix(multi): bail bind loop when remaining services are dead (Ready panel shows immediately)',
      items: [
      ],
    },
    {
      version: 'v0.7.27',
      date: '2026-05-27',
      headline: 'feat(ux): Ready panel + swagger/health probes + interactive log filter (a/e/p/N/q)',
      items: [
      ],
    },
    {
      version: 'v0.7.26',
      date: '2026-05-27',
      headline: 'feat(ux): prefixed multi-service logs; named exit announce; crush eject (Dockerfile + compose)',
      items: [
      ],
    },
    {
      version: 'v0.7.25',
      date: '2026-05-27',
      headline: 'feat(stacks): runnable entry commands for Rails, Laravel, Phoenix, .NET, Flask; multi-service support for all',
      items: [
      ],
    },
    {
      version: 'v0.7.24',
      date: '2026-05-27',
      headline: 'feat(ux): coloured output (cyan arrows, green ✓, red ✗, dimmed details)',
      items: [
      ],
    },
    {
      version: 'v0.7.23',
      date: '2026-05-27',
      headline: 'fix(multi): fire when root is generic fallback',
      items: [
      ],
    },
    {
      version: 'v0.7.22',
      date: '2026-05-27',
      headline: 'feat(multi): detect backend/+frontend/ pattern and spawn each in parallel',
      items: [
      ],
    },
    {
      version: 'v0.7.21',
      date: '2026-05-27',
      headline: 'fix(java): skip test compilation; honest port-ready message on failure',
      items: [
      ],
    },
    {
      version: 'v0.7.20',
      date: '2026-05-27',
      headline: 'fix(run): pick install/entry from detected stack; Java uses mvn spring-boot:run',
      items: [
      ],
    },
    {
      version: 'v0.7.19',
      date: '2026-05-27',
      headline: 'feat(spring): parse application.yml to auto-start postgres/redis deps',
      items: [
      ],
    },
    {
      version: 'v0.7.18',
      date: '2026-05-27',
      headline: 'fix(detect): Java wins over sibling package.json; Node version from engines.node',
      items: [
      ],
    },
    {
      version: 'v0.7.17',
      date: '2026-05-27',
      headline: 'fix(run): separate install_command; spawn via cmd /c on Windows for .cmd shims',
      items: [
      ],
    },
    {
      version: 'v0.7.16',
      date: '2026-05-27',
      headline: 'feat(detect): monorepo support — use root dev script via package manager',
      items: [
      ],
    },
    {
      version: 'v0.7.14',
      date: '2026-05-27',
      headline: 'feat(postgres): honor POSTGRES_USER/DB from compose; createdb when db != user',
      items: [
      ],
    },
    {
      version: 'v0.7.13',
      date: '2026-05-27',
      headline: 'feat(deps): synthesize DATABASE_URL/REDIS_URL from compose dep services',
      items: [
      ],
    },
    {
      version: 'v0.7.12',
      date: '2026-05-27',
      headline: 'feat(detect): parse entrypoint.sh/Dockerfile for uvicorn module path',
      items: [
      ],
    },
    {
      version: 'v0.7.11',
      date: '2026-05-27',
      headline: 'fix(build): also exclude .venv et al. from tar (only hash had been fixed)',
      items: [
      ],
    },
    {
      version: 'v0.7.10',
      date: '2026-05-27',
      headline: 'perf(build): exclude .venv, __pycache__, dist, build from layered tar',
      items: [
      ],
    },
    {
      version: 'v0.7.9',
      date: '2026-05-27',
      headline: 'fix(run): run build_command before spawn when venv/node_modules is missing',
      items: [
      ],
    },
    {
      version: 'v0.7.8',
      date: '2026-05-27',
      headline: 'fix(run): invoke uvicorn from venv directly; tolerate pg_ctl already-running exit',
      items: [
      ],
    },
    {
      version: 'v0.7.6',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:**',
        'Postgres `initdb` pwfile now written to parent directory, not inside the data dir — initdb requires a completely empty data directory',
        '`uv run` entry point now includes `--no-build-package {name}` using the project name from `pyproject.toml`, preventing `uv_build.build_editable` failures on projects with non-standard `src/` layouts',
      ],
    },
    {
      version: 'v0.7.5',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:**',
        'Data directory moved from `%PROGRAMDATA%` (requires admin) to `%LOCALAPPDATA%\Crush` — no more permission errors on Windows',
        'uv sync now passes `--no-build-package {name}` using the project name from `pyproject.toml`, preventing `uv_build.build_editable` failures on projects with non-standard `src/` layouts',
      ],
    },
    {
      version: 'v0.7.4',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:**',
        'PostgreSQL: clean partial data directory before `initdb` so a previously-failed run doesn\'t block a fresh start',
        'Python/uv: read the exact `uv sync` command from the project\'s Dockerfile (picks up `--frozen`, `--no-install-project`, etc.); fallback is `uv sync --no-dev --no-install-project`',
      ],
    },
    {
      version: 'v0.7.3',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:** PostgreSQL cluster init now calls `initdb` directly instead of via `pg_ctl initdb -o`, eliminating argument quoting failures on paths with spaces. Error output is now shown when initdb fails.',
      ],
    },
    {
      version: 'v0.7.2',
      date: '2026-05-27',
      headline: '',
      items: [
        '*New:** PostgreSQL now auto-downloads portable binaries on Windows when not installed — no `winget install`, no admin rights, no manual setup. First `crush` run downloads ~30 MB of EDB portable binaries and caches them at `~/.crush/cache/postgres/`.',
      ],
    },
    {
      version: 'v0.7.1',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:** Python apps (FastAPI, Flask, Django, Starlette) now launch correctly with `uvicorn`/`python` instead of trying to exec the `.py` file directly.',
      ],
    },
    {
      version: 'v0.7.0',
      date: '2026-05-27',
      headline: '## What\'s new in v0.7.0',
      items: [
        '### Zero-container native service runtime',
        'Crush can now start Postgres and Redis-compatible servers **without Docker, WSL2, or any container runtime** — pure native processes on both Windows and Linux.',
        '*How it works:**',
        '**Postgres**: uses `pg_ctl` (bundled with any PostgreSQL install) to `initdb` + start a cluster in a project-local data directory. Port-isolated, auto-initialized on first run.',
        '**Redis (Windows)**: auto-downloads [Microsoft Garnet](https://github.com/microsoft/garnet) — a .NET self-contained `.exe`, zero dependencies.',
        '**Redis (Linux/macOS)**: uses system `valkey-server` or `redis-server` if present; otherwise auto-downloads [Valkey 7.2.5](https://github.com/valkey-io/valkey).',
        '### New commands',
        '`crush services status` — show running native services for the current project',
        '`crush services stop` — stop all native services for the current project',
        '### Architecture',
        'New `crush-services` crate: `ServiceDriver` trait, `BinaryCache` (download + SHA-256 verify + extract), `NativeServiceState` persistence',
        'Removed all C/FFI dependencies (`pg-embed`) — cross-compiles cleanly on any toolchain',
        '## Installation',
        'Download `crush-v0.7.0-x86_64-windows.exe`, rename to `crush.exe`, and place it on your PATH.',
        '*Prerequisites:**',
        'Postgres: `winget install PostgreSQL.PostgreSQL` (or any existing install)',
        'Redis: none — Garnet is auto-downloaded on first use',
      ],
    },
    {
      version: 'v0.6.0',
      date: '2026-05-27',
      headline: 'v0.6.0 — Native Service Orchestration',
      items: [
      ],
    },
    {
      version: 'v0.5.1',
      date: '2026-05-27',
      headline: 'Patch: fix scanner false positives (pnpm-lock.yaml, test files, admin role labels). crush update will now detect this as a newer version.',
      items: [
      ],
    },
    {
      version: 'v0.5.0',
      date: '2026-05-27',
      headline: 'Scanner false-positive fixes: pnpm-lock.yaml excluded, test files skip vuln scanner, HARDCODED_ADMIN pattern tightened. Plus instant crush default command and base-image fallbacks.',
      items: [
      ],
    },
    {
      version: 'v0.4.0',
      date: '2026-05-27',
      headline: '## Crush v0.4.0',
      items: [
        'Three major capability upgrades: **cloud deployment** (Crushfile → production server in one command), **eBPF-based per-container metrics** (network + disk I/O from the kernel), and a **smarter stack detector** that reads your actual dependency files instead of guessing from filenames.',
        '--',
        '### Installation',
        '*Windows — first-time install**',
        '```',
        'curl -LO https://github.com/Chidi09/crush/releases/download/v0.4.0/crush-0.4.0-windows-x86_64.exe',
        'crush-0.4.0-windows-x86_64.exe install',
        '```',
        '*Upgrading from v0.3.x**',
        '```',
        'crush update',
        '```',
        '--',
        '### What\'s new',
        '#### `crush deploy` — one-command cloud deployment',
        'Add a `[deploy]` section to your Crushfile and run `crush deploy`. Crush builds the image, exports an OCI tarball, provisions the server (or reuses an existing one), and starts the container remotely.',
        '```toml',
        '[deploy]',
        'provider = \"hetzner\"',
        'region = \"nbg1-dc3\"',
        'server_type = \"cx21\"',
        '[deploy.hetzner]',
        'api_token = \"${HETZNER_API_TOKEN}\"',
        'ssh_key_name = \"my-key\"',
        '```',
        '```',
        'crush deploy           # build + provision + deploy',
        'crush deploy --status  # show URL, server, deployed-at',
        'crush deploy --logs    # tail container logs after deploy',
        'crush deploy --destroy # remove the server',
        '```',
        '*Supported providers:** Hetzner Cloud, DigitalOcean, AWS EC2, GCP Compute Engine, Fly.io, and any server accessible over SSH.',
        'For all cloud providers, Crush SSHes in to install itself and run the container — no Docker, no registry, no Kubernetes.',
        '*Deployment state** is persisted to `~/.crush/deployments/<project>.json` so subsequent `crush deploy` calls update in-place rather than creating new servers.',
        '#### eBPF metrics — network + disk I/O per container',
        'When running on a Linux kernel ≥ 5.4 with BTF, the `crush stats` TUI now shows real per-container network and disk I/O sourced from the kernel via two new eBPF programs:',
        '`crush_net_ingress` / `crush_net_egress` — `cgroup_skb` programs attached to each container\'s cgroup, counting bytes in and out.',
        '`crush_block_rq_complete` — `tracepoint/block/block_rq_complete` counting bytes read and written per cgroup when block requests complete.',
        'The stats view gains four new sparklines: **NET IN**, **NET OUT**, **DISK R**, **DISK W**, color-coded green < 1 MB/s, yellow 1–10 MB/s, red > 10 MB/s.',
        'On kernels without BTF or eBPF support, the same columns fall back to `/proc/<pid>/net/dev` and `/proc/<pid>/io` for approximate per-process I/O rates.',
        '#### Detector overhaul — signal scoring with real dep-tree parsing',
        'The stack detector no longer relies purely on file-existence heuristics. It now reads your actual dependency manifests and scores signals:',
        '**Node.js:** reads `dependencies`, `devDependencies`, and `peerDependencies` from `package.json`. Config files (e.g. `next.config.ts`) score 10 pts, direct deps (e.g. `\"next\"` in deps) score 8 pts, start-script patterns score 4–5 pts. Winner is the framework with the highest score.',
        '**Python:** parses `requirements.txt` line by line and `pyproject.toml` (`[project.dependencies]` for PEP 621, `[tool.poetry.dependencies]` for Poetry). FastAPI, Flask, Django, Tornado, aiohttp, Starlette, Litestar are all detected from the actual installed packages.',
        '**Ruby:** parses `Gemfile` declarations (`gem \'rails\'`, `gem \"sinatra\"`, etc.) instead of inferring from directory structure.',
        '**PHP:** parses `composer.json` `require` and `require-dev` sections instead of reading the file as a raw string.',
        'When a framework is found as a **direct dependency** (not just a config file), confidence is raised to **0.99** — meaning crush is as certain as it can be without running the code.',
        '--',
        '### Assets',
        '| File | Platform |',
        '|------|----------|',
        '| `crush-0.4.0-windows-x86_64.exe` | Windows x86-64 |',
        'Linux and macOS binaries can be self-compiled: `cargo build --release -p crush-cli`',
      ],
    },
    {
      version: 'v0.3.0',
      date: '2026-05-27',
      headline: '## Crush v0.3.0',
      items: [
        'Three quality-of-life upgrades: **PATH self-installation**, a smarter **project detector**, and a readable **inspect** command.',
        '--',
        '### Installation',
        '*Windows — first-time install**',
        '```',
        'curl -LO https://github.com/Chidi09/crush/releases/download/v0.3.0/crush-0.3.0-windows-x86_64.exe',
        'crush-0.3.0-windows-x86_64.exe install',
        '```',
        'Copies the binary to `%LOCALAPPDATA%\crush\bin\crush.exe` and adds it to your user PATH — no admin rights required, takes effect in any new terminal immediately.',
        '*Upgrading from v0.2.0**',
        '```',
        'crush update',
        '```',
        'Downloads the new binary and automatically re-runs the PATH install so your shell entry stays current.',
        '--',
        '### What\'s new',
        '#### Global PATH self-installation',
        'New `crush install` subcommand — copies the binary to a stable location and writes the directory to the Windows user PATH registry key (`HKCU\Environment`), then broadcasts `WM_SETTINGCHANGE` so new terminals see it without logging off. No admin rights needed.',
        'On Linux, installs to `~/.local/bin/crush` with executable permissions.',
        '`crush update` now calls `crush install` after a successful download, keeping the PATH entry up to date automatically.',
        'New installer scripts: `scripts/install.ps1` (PowerShell) and `scripts/install.sh` (bash).',
        '#### Smarter project detector',
        '**Bug fix — Go frameworks:** Previously checked for a directory named `gin` in the project root (wrong). Now correctly scans `go.mod` for import paths like `github.com/gin-gonic/gin`, `github.com/labstack/echo`, `github.com/gofiber/fiber`, `github.com/go-chi/chi`, `google.golang.org/grpc`.',
        '**Bug fix — Node package manager:** Build commands always said `npm` even for pnpm/yarn projects. Now checks for `pnpm-lock.yaml` and `yarn.lock` first and outputs the correct `pnpm run build` or `yarn build`.',
        '**New frameworks:** SvelteKit, Astro, SolidStart, Qwik, Laravel, Symfony.',
        '**New Python toolchains:** `uv` (`uv sync --no-dev`), `pdm` (`pdm install --prod`), conda (`environment.yml`).',
        '**Versioned base images:** Instead of always pulling `ubuntu:22.04`, the build pipeline now resolves an optimised base image from the detected runtime version — `node:20-alpine`, `python:3.11-slim`, `golang:1.22-alpine`, `eclipse-temurin:21-jre-alpine`, etc.',
        '`crush detect --json` outputs machine-readable JSON for scripting and CI pipelines.',
        '#### Readable inspect output',
        '`crush inspect` now prints a structured, human-readable report by default — container state, ports table, mounts table (bind vs tmpfs, ro/rw), cgroup resource limits, health check status, and restart policy.',
        'Raw JSON is still available: `crush inspect <id> --format json`',
        'Same formatting applied to image inspection: architecture, digest, entrypoint, layers, environment variables.',
        '--',
        '### Assets',
        '| File | Platform |',
        '|------|----------|',
        '| `crush-0.3.0-windows-x86_64.exe` | Windows x86-64 |',
        'Linux and macOS binaries can be self-compiled: `cargo build --release -p crush-cli`',
      ],
    },
    {
      version: 'v0.2.0',
      date: '2026-05-27',
      headline: '## Crush v0.2.0',
      items: [
        'The runtime actually works now. v0.2.0 is a foundational correctness release — every command that previously printed placeholder output has been replaced with a real implementation.',
        '--',
        '### Installation',
        '*Windows**',
        '```',
        'curl -LO https://github.com/Chidi09/crush/releases/download/v0.2.0/crush-0.2.0-windows-x86_64.exe',
        '```',
        'Run the binary directly. For global install with PATH, upgrade to [v0.3.0](https://github.com/Chidi09/crush/releases/tag/v0.3.0).',
        '--',
        '### What\'s new',
        '#### Runtime fixes',
        '**Container entrypoint:** `crush run` was spawning `/sbin/init` with the container ID as an argument instead of the image\'s actual entrypoint. Fixed — the correct command is now loaded from the container config on disk.',
        '**PID persistence:** Container PIDs are now written to disk on start and restored on daemon restart, so `crush ps` correctly shows running containers after a crash or reboot.',
        '**cgroup v2 CPU limits:** Docker-style `cpu_shares` (1–262144) are now correctly converted to cgroup v2 `cpu.weight` (1–10000) before being written to the kernel. The old code passed raw values that could trigger `EINVAL`.',
        '**Debug logging removed:** The container runner was writing a step-by-step trace to `/tmp/crush_pre_exec_debug.log` on every container start. Stripped.',
        '#### Build pipeline',
        '`crush build` and `crush default` now actually execute the detected build command (`npm install`, `cargo build --release`, `pip install`, etc.) inside the project directory. Previously this step stored a placeholder string and returned immediately.',
        'If a `Crushfile` is present in the project root, its `[[stages]]` are parsed and used directly instead of auto-generated ones.',
        '#### CLI command completions',
        '`crush logs --follow` tails the live log file instead of streaming only the initial snapshot.',
        '`crush inspect` loads and displays real container/image data from disk instead of a stub.',
        '`crush health` runs the container\'s configured health check command with the configured timeout.',
        '`crush volume` subcommands (`ls`, `create`, `rm`, `inspect`) are wired to the `VolumeManager`.',
        '`crush daemon` starts the real API server (Unix socket on Linux, named pipe on Windows).',
        '`crush docker-context` writes a valid Docker CLI context to `~/.docker/contexts/` so `docker` commands are routed to Crush.',
        '`crush ps` shows human-readable uptime (\"Up 3 hours\") and dynamically sized columns.',
        '`crush export` produces a standards-compliant OCI tarball with `oci-layout`, blobs, and `index.json`.',
        '`crush pull` and `crush build` accept a `--platform` flag (e.g. `linux/arm64`) for multi-arch image handling.',
        '#### Windows runtime',
        'Real PID tracking — `get_pid()` no longer returns a hardcoded `4242`.',
        '`exec()` spawns a process inside the existing Job Object.',
        '`delete()` removes the Job Object handle and cleans up the container state directory.',
        '`crush run` on Windows now dispatches to `WindowsRuntime` instead of the Linux runner path.',
        '--',
        '### Assets',
        '| File | Platform |',
        '|------|----------|',
        '| `crush-0.2.0-windows-x86_64.exe` | Windows x86-64 |',
        'Linux and macOS binaries can be self-compiled: `cargo build --release -p crush-cli`',
      ],
    },
    {
      version: 'v0.1.0',
      date: '2026-05-26',
      headline: 'Windows x86_64 build from 2026-05-26 22:04 UTC.',
      items: [
        '## Install',
        'Download `crush-0.1.0-windows-x86_64.exe` below and place it on your PATH:',
        '```powershell',
        'New-Item -ItemType Directory -Force -Path C:\crush',
        'Invoke-WebRequest \'<download-url>\' -OutFile C:\crush\crush.exe',
        '$env:PATH += \';C:\crush\'',
        '[Environment]::SetEnvironmentVariable(\'PATH\', $env:PATH, \'User\')',
        '```',
      ],
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Changelog — Crush');
    this.meta.updateTag({
      name: 'description',
      content: 'Crush version history and release notes.',
    });
  }
}
