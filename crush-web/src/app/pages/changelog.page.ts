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
