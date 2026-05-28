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
        'Scaffolded Tauri 2 + SvelteKit desktop app GUI shell (<code class="text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded">crates/crush-gui/src-tauri</code>).',
        'Implemented Tauri command bindings for native process lifecycle control, image stores, and system configurations.',
        'Wired real backends including <code class="text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded">crush-api</code> unix sockets, <code class="text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded">crush-proto</code> OCI gateway, and <code class="text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded">crush-tui</code> sparklines.',
        'Created a high-fidelity brand assets generator to avoid <code class="text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded">windres</code> resources compilation errors during Windows GUI builds.',
      ],
    },
    {
      version: 'v0.7.74',
      date: '2026-05-28',
      headline: 'Restores four output lines that v0.7.73&#x27;s run_project refactor dropped against v0.7.72:',
      items: [
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">↳ dependencies layer cached (unchanged)</code> after image-fresh',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">✓ crushed to image &lt;tag&gt;:latest (0 MB)</code> headline on warm runs',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">✓ dependencies fresh — node_modules newer than lockfile (--rebuild to force)</code> when the install step is skipped',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">↳ warm run — launching</code> with the correct cyan info icon (was incorrectly emitted as a yellow <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">⚠</code> warning)',
        'Added two structured RunEvent variants (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">WarmRun</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">DepsFresh</code>) so the GUI gets these signals too, not just the CLI. Also dropped an unused speculative <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">Line</code> variant that wasn&#x27;t compiling.',
        'CLI behaviour vs v0.7.72: identical on warm runs.',
      ],
    },
    {
      version: 'v0.7.73',
      date: '2026-05-28',
      headline: 'The Commands::Default flow (~1200 lines) was extracted from <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-cli/src/main.rs</code> into <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush_build::run::run_project()</code>. CLI behaviour is identical — <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">println!</code> calls became <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">tx.send(RunEvent::…)</code> and the CLI&#x27;s Default arm became a thin event-printing consumer.',
      items: [
        'This is the prep enabler for the v0.8 GUI. The GUI will consume the same <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">RunEvent</code> stream via <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">tauri::Window::emit</code> instead of stdout.',
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
        '&#x27;crush history&#x27; subcommand reads it (text or --format json)',
        '&#x27;crush ps --format json&#x27; emits the container list as JSON',
        '&#x27;crush services ps --format json&#x27; emits per-project native + container state. Add --all-projects to span every project',
        '&#x27;crush images --format json&#x27; emits the image list',
        'GUI_DATA_CONTRACT.md documents stable file paths, schemas, polling cadence, anti-fragility rules',
        '*Not yet:**',
        'The crush_build::run::run_project() function body — that&#x27;s the next agent&#x27;s first task. Types and target signature are pinned in run.rs and CRUSH_V8_PLAN.md so they don&#x27;t have to design the API.',
      ],
    },
    {
      version: 'v0.7.71',
      date: '2026-05-28',
      headline: 'Sets PYTHONUTF8=1 (PEP 540) and PYTHONUNBUFFERED=1 when crush spawns a Python app:',
      items: [
        'PYTHONUTF8=1 forces UTF-8 for stdout/stderr and open() regardless of system locale. Fixes Windows cp1252 UnicodeEncodeError when apps print emoji or non-ASCII (e.g. gazillion-be-staging crashed printing &#x27;🔧 Loading configuration...&#x27;).',
        'PYTHONUNBUFFERED=1 flushes prints immediately instead of buffering when stdout isn&#x27;t a TTY. Log lines appear as they&#x27;re emitted.',
      ],
    },
    {
      version: 'v0.7.70',
      date: '2026-05-28',
      headline: 'Implements PGVECTOR_PLAN.md. When a project asks for pgvector/pgvector:* and Docker isn&#x27;t available, crush now builds the extension against the host PostgreSQL using MSVC and installs it before the app connects.',
      items: [
        'Requires Visual Studio Build Tools 2022 with the &#x27;Desktop development with C++&#x27; workload + Windows SDK. Install with:',
        'winget install --id Microsoft.VisualStudio.2022.BuildTools --override &quot;--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows10SDK.19041&quot;',
        'The pgvector source is cloned at the pinned tag v0.8.0 into your crush cache and the build only runs once per PG install (idempotent via vector.control check). The install step writes into PostgreSQL&#x27;s lib/ and share/extension/ — if PG is under Program Files this requires an elevated terminal.',
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
      headline: 'Found while running crush on project_approval_system_root: Django detection (v0.7.67) succeeded but the build step failed because there&#x27;s no requirements.txt — deps were installed once into .venv and the manifest forgotten.',
      items: [
        'Skip install step when .venv is present but no requirements.txt / pyproject.toml exists',
        'Use .venv/Scripts/python (Windows) / .venv/bin/python (Unix) for Django entry when .venv exists. Bare &#x27;python&#x27; on PATH is a different interpreter',
        'Drop &#x27;collectstatic&#x27; from default Django build — runserver serves statics with DEBUG=True. Add back via Crushfile if your prod flow needs it',
      ],
    },
    {
      version: 'v0.7.67',
      date: '2026-05-28',
      headline: 'Fixes found while running detect across 12 real projects:',
      items: [
        '<strong>Django detection</strong> fires on manage.py alone — no longer hijacked by a stray package.json at the same root.',
        '<strong>Script-driven port resolution</strong> — if your dev/start script literally invokes vite / nuxt / next / astro / ng serve, that tool&#x27;s port wins over the framework signal. Fixes Angular-with-vite (Solexpay-frontend) reporting 4200 when the server is actually on 5173.',
        '<strong>$PORT translation</strong> — Windows spawn_shell now rewrites bash $VAR and ${VAR} to cmd.exe %VAR%. Fixes &#x27;uvicorn --port $PORT&#x27; on FastAPI/Python projects.',
      ],
    },
    {
      version: 'v0.7.66',
      date: '2026-05-28',
      headline: 'Extends the docker-shape heuristic (v0.7.48+) to Java/Maven/Gradle. When no Dockerfile/compose exists in the repo, crush now treats it as a dev workflow and:',
      items: [
        'Maven: &#x27;mvn -B compile&#x27; + &#x27;mvn spring-boot:run&#x27; (was: package + java -jar)',
        'Gradle: &#x27;gradle classes&#x27; + &#x27;gradle bootRun&#x27; (was: bootJar + java -jar)',
        'For Solexpay-backend this skips the ~60s &#x27;mvn package&#x27; (jar build + repackage) and runs Spring directly. If you add spring-boot-devtools to pom.xml, subsequent code changes trigger ~3s hot restarts instead of full boot.',
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
        '*--priority high | above-normal** — sets the Job Object PriorityClass on Windows so crush&#x27;s process tree runs at a higher scheduling priority. Useful when the system is under load. Silently ignored on non-Windows.',
      ],
    },
    {
      version: 'v0.7.64',
      date: '2026-05-28',
      headline: 'springdoc-openapi lazy-inits on first request (~4s) which exceeds the 700ms probe timeout, causing an AsyncRequestNotUsable stack trace. swagger-ui (which we already probe) is the user-facing URL and back-links to api-docs internally.',
      items: [
        'Remaining noise on Solexpay is the app&#x27;s own /actuator/health spawning a mail health check with no SMTP creds — not crush. Silence with &#x27;management.health.mail.enabled=false&#x27; in application.yml.',
      ],
    },
    {
      version: 'v0.7.63',
      date: '2026-05-28',
      headline: 'Spring Boot, FastAPI, NestJS, Express, and Fastify now get a narrower probe list (only paths each framework actually exposes) plus the SPA-shell &#x27;/&#x27; fingerprint is skipped for these backends. Cuts the 5-10 stack traces per crush run.',
      items: [
        'Unknown stacks still get the full 12-path probe so we don&#x27;t miss anything on projects we don&#x27;t recognise.',
      ],
    },
    {
      version: 'v0.7.62',
      date: '2026-05-28',
      headline: 'v0.7.61&#x27;s CREATE DATABASE step used psql&#x27;s \gexec meta-command via -c, which is treated as part of the SQL string and silently errored as a syntax error. Now uses plain CREATE DATABASE and ignores the &#x27;already exists&#x27; error.',
      items: [
      ],
    },
    {
      version: 'v0.7.61',
      date: '2026-05-28',
      headline: 'After postgres starts, run an idempotent CREATE/ALTER ROLE block so configured creds win even when the data dir was init&#x27;d with different ones by an earlier crush version.',
      items: [
      ],
    },
    {
      version: 'v0.7.60',
      date: '2026-05-28',
      headline: 'v0.7.59 wrapped JAVA_HOME path in quotes unconditionally. cmd.exe doubly-escaped the quotes Tokio adds, producing literal \&quot;path\&quot; that fails to resolve. Now: skip quoting entirely when the path is space-free (scoop/sdkman/jenv installs).',
      items: [
      ],
    },
    {
      version: 'v0.7.59',
      date: '2026-05-28',
      headline: 'Fixes UnsupportedClassVersionError when the project&#x27;s required JDK is newer than the bare &#x27;java&#x27; on PATH. Now spawn_shell rewrites a leading &#x27;java &#x27; to &#x27;$JAVA_HOME/bin/java &#x27; when JAVA_HOME is set — same JDK Maven uses, so class versions match.',
      items: [
        'Falls back to bare &#x27;java&#x27; when JAVA_HOME is unset or the binary doesn&#x27;t exist.',
      ],
    },
    {
      version: 'v0.7.58',
      date: '2026-05-28',
      headline: 'cmd.exe doesn&#x27;t expand glob patterns, so &#x27;java -jar target/*.jar&#x27; failed with &#x27;Unable to access jarfile target/*.jar&#x27; even after a successful Maven build.',
      items: [
        'spawn_shell now resolves the glob to a concrete jar path before exec, skipping Spring Boot&#x27;s *.jar.original (the pre-repackage artifact).',
      ],
    },
    {
      version: 'v0.7.57',
      date: '2026-05-28',
      headline: 'Switch Java/Maven build from &#x27;-DskipTests&#x27; (skips only execution) to &#x27;-Dmaven.test.skip=true&#x27; (skips compile AND execution).',
      items: [
        'Unblocks projects where tests reference stale/missing symbols but the actual app code is fine — common on feature branches where tests haven&#x27;t caught up.',
      ],
    },
    {
      version: 'v0.7.56',
      date: '2026-05-28',
      headline: 'Multi-service runs already probed /swagger-ui, /v3/api-docs, /healthz, /graphql etc. Single-service runs (a plain backend API) didn&#x27;t.',
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
      headline: 'When image is fresh AND node_modules is current, crush no longer asks &#x27;run it now? [Y/n]&#x27; — it just runs. The prompt is friction on warm reruns where you obviously came here to run the app.',
      items: [
        'Cold runs and first builds still show the prompt for user verification.',
      ],
    },
    {
      version: 'v0.7.54',
      date: '2026-05-28',
      headline: 'Crush was still running &#x27;pnpm install&#x27; on warm runs even when node_modules was up to date — paying ~10s for pnpm&#x27;s &#x27;Already up to date&#x27; check.',
      items: [
        'Now: when install command is just &#x27;&lt;pm&gt; install&#x27; (no &#x27;&amp;&amp; build&#x27;), crush compares node_modules mtime to lockfile mtime. If node_modules is newer, install is skipped entirely.',
        'Applies to pnpm/npm/yarn/bun/deno. Override with --rebuild.',
      ],
    },
    {
      version: 'v0.7.53',
      date: '2026-05-28',
      headline: '',
      items: [
        'Vite default port corrected from 3000 to 5173 (was causing &#x27;no response on :3000&#x27; false alarm)',
        'AnalogJS detection via @analogjs/platform/@analogjs/router (port 5173)',
        'Angular detection via @angular/core (port 4200)',
      ],
    },
    {
      version: 'v0.7.52',
      date: '2026-05-28',
      headline: 'When <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush</code> lands on the generic fallback (no detectable stack) and theres no entrypoint.sh, it now scans immediate subdirs for project markers (package.json, Cargo.toml, go.mod, pyproject.toml, etc) and suggests <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cd &lt;subdir&gt; &amp;&amp; crush</code> instead of trying to spawn a missing entrypoint.sh.',
      items: [
        'Replaces the cryptic <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">entrypoint.sh is not recognized as an internal or external command</code> error you would otherwise see.',
      ],
    },
    {
      version: 'v0.7.51',
      date: '2026-05-28',
      headline: 'Real fix for the standalone-frontend dev workflow.',
      items: [
        'In v0.7.48 the docker-shape heuristic applied only to the fallback entry path. But Nuxt/Vite/Next/Astro/Remix/SvelteKit each set entry_prod explicitly (e.g. <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pnpm run preview</code> for Nuxt), masking the heuristic.',
        'Now: no prod-shape Docker artifacts -&gt; entry_prod falls back to dev_entry (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pnpm run dev</code>) regardless of framework, and the build step skips <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pnpm run build</code> since dev servers compile on the fly.',
      ],
    },
    {
      version: 'v0.7.50',
      date: '2026-05-28',
      headline: '',
      items: [
        '<strong>crush watch</strong> on Windows now exits with a friendly hint pointing to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush --watch</code> (native flow). The old subcommand used Linux overlayfs which broke on Windows.',
        '<strong>TUI Compose tab</strong> now shows real running native dep services from <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">&lt;data_dir&gt;/native/*.json</code> (postgres, redis-compat, mysql) with live pid probing. Replaces the hardcoded web/db/cache mock data.',
      ],
    },
    {
      version: 'v0.7.49',
      date: '2026-05-28',
      headline: '',
      items: [
        '<strong>eject marker</strong>: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush eject</code> writes a <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\"># crush:eject</code> marker on line 1 of Dockerfile and docker-compose.yml. The dev/prod heuristic skips marked files, so <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush eject</code> followed by <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush</code> still runs dev mode (HMR). Delete the marker line to adopt the generated file as your real prod config.',
        '<strong>ps empty-state</strong>: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush ps</code> with no containers prints inline instead of flashing the alt-screen TUI and leaving no trace in scrollback.',
      ],
    },
    {
      version: 'v0.7.48',
      date: '2026-05-28',
      headline: 'Node entry picks dev vs start based on whether the repo ships prod-shape Docker artifacts.',
      items: [
        'Repo with Dockerfile or compose (in /, infra/, docker/, etc.) -&gt; prefer <strong>start</strong> (match what docker would run).',
        'Repo without -&gt; prefer <strong>dev</strong> (match native pnpm dev workflow with HMR).',
        'Applies to both monorepo and standalone Node projects. Fixes standalone Nuxt/Next/Vite repos running prod preview when the dev wanted HMR.',
      ],
    },
    {
      version: 'v0.7.47',
      date: '2026-05-28',
      headline: '## Speed &amp; ergonomics — six units shipped',
      items: [
        '<strong>Unit 1 — Warm-run cache</strong>: skip image pack when sources unchanged (content fingerprint via mtimes)',
        '<strong>Unit 2 — Parallel builds</strong>: sub-services build concurrently, capped by semaphore (min cpus, 4)',
        '<strong>Unit 3 — Parallel deps</strong>: dep services start in parallel; Garnet binary prefetched in background',
        '<strong>Unit 4 — Lifecycle commands</strong>: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush ps</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush logs</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush stop</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush restart</code>',
        '<strong>Unit 5 — Resource limits</strong>: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--memory</code> and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--cpus</code> flags via Windows Job Object controls',
        '<strong>Unit 6 — Watch mode</strong>: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--watch</code> reruns the affected sub-service on source change',
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
        'Postgres <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">initdb</code> pwfile now written to parent directory, not inside the data dir — initdb requires a completely empty data directory',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv run</code> entry point now includes <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--no-build-package {name}</code> using the project name from <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pyproject.toml</code>, preventing <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv_build.build_editable</code> failures on projects with non-standard <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">src/</code> layouts',
      ],
    },
    {
      version: 'v0.7.5',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:**',
        'Data directory moved from <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">%PROGRAMDATA%</code> (requires admin) to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">%LOCALAPPDATA%\Crush</code> — no more permission errors on Windows',
        'uv sync now passes <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--no-build-package {name}</code> using the project name from <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pyproject.toml</code>, preventing <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv_build.build_editable</code> failures on projects with non-standard <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">src/</code> layouts',
      ],
    },
    {
      version: 'v0.7.4',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:**',
        'PostgreSQL: clean partial data directory before <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">initdb</code> so a previously-failed run doesn&#x27;t block a fresh start',
        'Python/uv: read the exact <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv sync</code> command from the project&#x27;s Dockerfile (picks up <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--frozen</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--no-install-project</code>, etc.); fallback is <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv sync --no-dev --no-install-project</code>',
      ],
    },
    {
      version: 'v0.7.3',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:** PostgreSQL cluster init now calls <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">initdb</code> directly instead of via <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pg_ctl initdb -o</code>, eliminating argument quoting failures on paths with spaces. Error output is now shown when initdb fails.',
      ],
    },
    {
      version: 'v0.7.2',
      date: '2026-05-27',
      headline: '',
      items: [
        '*New:** PostgreSQL now auto-downloads portable binaries on Windows when not installed — no <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">winget install</code>, no admin rights, no manual setup. First <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush</code> run downloads ~30 MB of EDB portable binaries and caches them at <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">~/.crush/cache/postgres/</code>.',
      ],
    },
    {
      version: 'v0.7.1',
      date: '2026-05-27',
      headline: '',
      items: [
        '*Fixes:** Python apps (FastAPI, Flask, Django, Starlette) now launch correctly with <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uvicorn</code>/<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">python</code> instead of trying to exec the <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">.py</code> file directly.',
      ],
    },
    {
      version: 'v0.7.0',
      date: '2026-05-27',
      headline: '## What&#x27;s new in v0.7.0',
      items: [
        '### Zero-container native service runtime',
        'Crush can now start Postgres and Redis-compatible servers <strong>without Docker, WSL2, or any container runtime</strong> — pure native processes on both Windows and Linux.',
        '*How it works:**',
        '<strong>Postgres</strong>: uses <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pg_ctl</code> (bundled with any PostgreSQL install) to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">initdb</code> + start a cluster in a project-local data directory. Port-isolated, auto-initialized on first run.',
        '<strong>Redis (Windows)</strong>: auto-downloads <a href=\"https://github.com/microsoft/garnet\" class=\"text-crush-orange hover:underline\" target=\"_blank\">Microsoft Garnet</a> — a .NET self-contained <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">.exe</code>, zero dependencies.',
        '<strong>Redis (Linux/macOS)</strong>: uses system <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">valkey-server</code> or <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">redis-server</code> if present; otherwise auto-downloads <a href=\"https://github.com/valkey-io/valkey\" class=\"text-crush-orange hover:underline\" target=\"_blank\">Valkey 7.2.5</a>.',
        '### New commands',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush services status</code> — show running native services for the current project',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush services stop</code> — stop all native services for the current project',
        '### Architecture',
        'New <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-services</code> crate: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">ServiceDriver</code> trait, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">BinaryCache</code> (download + SHA-256 verify + extract), <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">NativeServiceState</code> persistence',
        'Removed all C/FFI dependencies (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pg-embed</code>) — cross-compiles cleanly on any toolchain',
        '## Installation',
        'Download <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-v0.7.0-x86_64-windows.exe</code>, rename to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush.exe</code>, and place it on your PATH.',
        '*Prerequisites:**',
        'Postgres: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">winget install PostgreSQL.PostgreSQL</code> (or any existing install)',
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
        'Three major capability upgrades: <strong>cloud deployment</strong> (Crushfile → production server in one command), <strong>eBPF-based per-container metrics</strong> (network + disk I/O from the kernel), and a <strong>smarter stack detector</strong> that reads your actual dependency files instead of guessing from filenames.',
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
        '### What&#x27;s new',
        '#### <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush deploy</code> — one-command cloud deployment',
        'Add a <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">[deploy]</code> section to your Crushfile and run <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush deploy</code>. Crush builds the image, exports an OCI tarball, provisions the server (or reuses an existing one), and starts the container remotely.',
        '```toml',
        '[deploy]',
        'provider = &quot;hetzner&quot;',
        'region = &quot;nbg1-dc3&quot;',
        'server_type = &quot;cx21&quot;',
        '[deploy.hetzner]',
        'api_token = &quot;${HETZNER_API_TOKEN}&quot;',
        'ssh_key_name = &quot;my-key&quot;',
        '```',
        '```',
        'crush deploy           # build + provision + deploy',
        'crush deploy --status  # show URL, server, deployed-at',
        'crush deploy --logs    # tail container logs after deploy',
        'crush deploy --destroy # remove the server',
        '```',
        '*Supported providers:** Hetzner Cloud, DigitalOcean, AWS EC2, GCP Compute Engine, Fly.io, and any server accessible over SSH.',
        'For all cloud providers, Crush SSHes in to install itself and run the container — no Docker, no registry, no Kubernetes.',
        '*Deployment state** is persisted to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">~/.crush/deployments/&lt;project&gt;.json</code> so subsequent <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush deploy</code> calls update in-place rather than creating new servers.',
        '#### eBPF metrics — network + disk I/O per container',
        'When running on a Linux kernel ≥ 5.4 with BTF, the <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush stats</code> TUI now shows real per-container network and disk I/O sourced from the kernel via two new eBPF programs:',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush_net_ingress</code> / <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush_net_egress</code> — <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cgroup_skb</code> programs attached to each container&#x27;s cgroup, counting bytes in and out.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush_block_rq_complete</code> — <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">tracepoint/block/block_rq_complete</code> counting bytes read and written per cgroup when block requests complete.',
        'The stats view gains four new sparklines: <strong>NET IN</strong>, <strong>NET OUT</strong>, <strong>DISK R</strong>, <strong>DISK W</strong>, color-coded green &lt; 1 MB/s, yellow 1–10 MB/s, red &gt; 10 MB/s.',
        'On kernels without BTF or eBPF support, the same columns fall back to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">/proc/&lt;pid&gt;/net/dev</code> and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">/proc/&lt;pid&gt;/io</code> for approximate per-process I/O rates.',
        '#### Detector overhaul — signal scoring with real dep-tree parsing',
        'The stack detector no longer relies purely on file-existence heuristics. It now reads your actual dependency manifests and scores signals:',
        '<strong>Node.js:</strong> reads <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">dependencies</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">devDependencies</code>, and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">peerDependencies</code> from <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">package.json</code>. Config files (e.g. <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">next.config.ts</code>) score 10 pts, direct deps (e.g. <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">&quot;next&quot;</code> in deps) score 8 pts, start-script patterns score 4–5 pts. Winner is the framework with the highest score.',
        '<strong>Python:</strong> parses <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">requirements.txt</code> line by line and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pyproject.toml</code> (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">[project.dependencies]</code> for PEP 621, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">[tool.poetry.dependencies]</code> for Poetry). FastAPI, Flask, Django, Tornado, aiohttp, Starlette, Litestar are all detected from the actual installed packages.',
        '<strong>Ruby:</strong> parses <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">Gemfile</code> declarations (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">gem &#x27;rails&#x27;</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">gem &quot;sinatra&quot;</code>, etc.) instead of inferring from directory structure.',
        '<strong>PHP:</strong> parses <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">composer.json</code> <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">require</code> and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">require-dev</code> sections instead of reading the file as a raw string.',
        'When a framework is found as a <strong>direct dependency</strong> (not just a config file), confidence is raised to <strong>0.99</strong> — meaning crush is as certain as it can be without running the code.',
        '--',
        '### Assets',
        '| File | Platform |',
        '|------|----------|',
        '| <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-0.4.0-windows-x86_64.exe</code> | Windows x86-64 |',
        'Linux and macOS binaries can be self-compiled: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cargo build --release -p crush-cli</code>',
      ],
    },
    {
      version: 'v0.3.0',
      date: '2026-05-27',
      headline: '## Crush v0.3.0',
      items: [
        'Three quality-of-life upgrades: <strong>PATH self-installation</strong>, a smarter <strong>project detector</strong>, and a readable <strong>inspect</strong> command.',
        '--',
        '### Installation',
        '*Windows — first-time install**',
        '```',
        'curl -LO https://github.com/Chidi09/crush/releases/download/v0.3.0/crush-0.3.0-windows-x86_64.exe',
        'crush-0.3.0-windows-x86_64.exe install',
        '```',
        'Copies the binary to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">%LOCALAPPDATA%\crush\bin\crush.exe</code> and adds it to your user PATH — no admin rights required, takes effect in any new terminal immediately.',
        '*Upgrading from v0.2.0**',
        '```',
        'crush update',
        '```',
        'Downloads the new binary and automatically re-runs the PATH install so your shell entry stays current.',
        '--',
        '### What&#x27;s new',
        '#### Global PATH self-installation',
        'New <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush install</code> subcommand — copies the binary to a stable location and writes the directory to the Windows user PATH registry key (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">HKCU\Environment</code>), then broadcasts <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">WM_SETTINGCHANGE</code> so new terminals see it without logging off. No admin rights needed.',
        'On Linux, installs to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">~/.local/bin/crush</code> with executable permissions.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush update</code> now calls <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush install</code> after a successful download, keeping the PATH entry up to date automatically.',
        'New installer scripts: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">scripts/install.ps1</code> (PowerShell) and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">scripts/install.sh</code> (bash).',
        '#### Smarter project detector',
        '<strong>Bug fix — Go frameworks:</strong> Previously checked for a directory named <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">gin</code> in the project root (wrong). Now correctly scans <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">go.mod</code> for import paths like <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">github.com/gin-gonic/gin</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">github.com/labstack/echo</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">github.com/gofiber/fiber</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">github.com/go-chi/chi</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">google.golang.org/grpc</code>.',
        '<strong>Bug fix — Node package manager:</strong> Build commands always said <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">npm</code> even for pnpm/yarn projects. Now checks for <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pnpm-lock.yaml</code> and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">yarn.lock</code> first and outputs the correct <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pnpm run build</code> or <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">yarn build</code>.',
        '<strong>New frameworks:</strong> SvelteKit, Astro, SolidStart, Qwik, Laravel, Symfony.',
        '<strong>New Python toolchains:</strong> <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv</code> (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">uv sync --no-dev</code>), <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pdm</code> (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pdm install --prod</code>), conda (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">environment.yml</code>).',
        '<strong>Versioned base images:</strong> Instead of always pulling <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">ubuntu:22.04</code>, the build pipeline now resolves an optimised base image from the detected runtime version — <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">node:20-alpine</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">python:3.11-slim</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">golang:1.22-alpine</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">eclipse-temurin:21-jre-alpine</code>, etc.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush detect --json</code> outputs machine-readable JSON for scripting and CI pipelines.',
        '#### Readable inspect output',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush inspect</code> now prints a structured, human-readable report by default — container state, ports table, mounts table (bind vs tmpfs, ro/rw), cgroup resource limits, health check status, and restart policy.',
        'Raw JSON is still available: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush inspect &lt;id&gt; --format json</code>',
        'Same formatting applied to image inspection: architecture, digest, entrypoint, layers, environment variables.',
        '--',
        '### Assets',
        '| File | Platform |',
        '|------|----------|',
        '| <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-0.3.0-windows-x86_64.exe</code> | Windows x86-64 |',
        'Linux and macOS binaries can be self-compiled: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cargo build --release -p crush-cli</code>',
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
        'Run the binary directly. For global install with PATH, upgrade to <a href=\"https://github.com/Chidi09/crush/releases/tag/v0.3.0\" class=\"text-crush-orange hover:underline\" target=\"_blank\">v0.3.0</a>.',
        '--',
        '### What&#x27;s new',
        '#### Runtime fixes',
        '<strong>Container entrypoint:</strong> <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush run</code> was spawning <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">/sbin/init</code> with the container ID as an argument instead of the image&#x27;s actual entrypoint. Fixed — the correct command is now loaded from the container config on disk.',
        '<strong>PID persistence:</strong> Container PIDs are now written to disk on start and restored on daemon restart, so <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush ps</code> correctly shows running containers after a crash or reboot.',
        '<strong>cgroup v2 CPU limits:</strong> Docker-style <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cpu_shares</code> (1–262144) are now correctly converted to cgroup v2 <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cpu.weight</code> (1–10000) before being written to the kernel. The old code passed raw values that could trigger <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">EINVAL</code>.',
        '<strong>Debug logging removed:</strong> The container runner was writing a step-by-step trace to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">/tmp/crush_pre_exec_debug.log</code> on every container start. Stripped.',
        '#### Build pipeline',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush build</code> and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush default</code> now actually execute the detected build command (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">npm install</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cargo build --release</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">pip install</code>, etc.) inside the project directory. Previously this step stored a placeholder string and returned immediately.',
        'If a <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">Crushfile</code> is present in the project root, its <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">[[stages]]</code> are parsed and used directly instead of auto-generated ones.',
        '#### CLI command completions',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush logs --follow</code> tails the live log file instead of streaming only the initial snapshot.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush inspect</code> loads and displays real container/image data from disk instead of a stub.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush health</code> runs the container&#x27;s configured health check command with the configured timeout.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush volume</code> subcommands (<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">ls</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">create</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">rm</code>, <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">inspect</code>) are wired to the <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">VolumeManager</code>.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush daemon</code> starts the real API server (Unix socket on Linux, named pipe on Windows).',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush docker-context</code> writes a valid Docker CLI context to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">~/.docker/contexts/</code> so <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">docker</code> commands are routed to Crush.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush ps</code> shows human-readable uptime (&quot;Up 3 hours&quot;) and dynamically sized columns.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush export</code> produces a standards-compliant OCI tarball with <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">oci-layout</code>, blobs, and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">index.json</code>.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush pull</code> and <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush build</code> accept a <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">--platform</code> flag (e.g. <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">linux/arm64</code>) for multi-arch image handling.',
        '#### Windows runtime',
        'Real PID tracking — <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">get_pid()</code> no longer returns a hardcoded <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">4242</code>.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">exec()</code> spawns a process inside the existing Job Object.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">delete()</code> removes the Job Object handle and cleans up the container state directory.',
        '<code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush run</code> on Windows now dispatches to <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">WindowsRuntime</code> instead of the Linux runner path.',
        '--',
        '### Assets',
        '| File | Platform |',
        '|------|----------|',
        '| <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-0.2.0-windows-x86_64.exe</code> | Windows x86-64 |',
        'Linux and macOS binaries can be self-compiled: <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">cargo build --release -p crush-cli</code>',
      ],
    },
    {
      version: 'v0.1.0',
      date: '2026-05-26',
      headline: 'Windows x86_64 build from 2026-05-26 22:04 UTC.',
      items: [
        '## Install',
        'Download <code class=\"text-crush-orange bg-crush-orange/10 px-1 py-0.5 rounded\">crush-0.1.0-windows-x86_64.exe</code> below and place it on your PATH:',
        '```powershell',
        'New-Item -ItemType Directory -Force -Path C:\crush',
        'Invoke-WebRequest &#x27;&lt;download-url&gt;&#x27; -OutFile C:\crush\crush.exe',
        '$env:PATH += &#x27;;C:\crush&#x27;',
        '[Environment]::SetEnvironmentVariable(&#x27;PATH&#x27;, $env:PATH, &#x27;User&#x27;)',
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
