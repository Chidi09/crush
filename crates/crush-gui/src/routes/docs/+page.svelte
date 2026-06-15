<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import * as api from '$lib/tauri';

  const SITE = 'https://github.com/Chidi09/crush';

  let copied = $state<string | null>(null);
  async function copy(text: string, id: string) {
    try { await navigator.clipboard.writeText(text); copied = id; setTimeout(() => { if (copied === id) copied = null; }, 1400); } catch {}
  }
  function open(url: string) { api.openUrl(url).catch(() => {}); }

  // ── Table of contents ────────────────────────────────────────────────
  const TOC = [
    { id: 'intro',        label: 'Introduction' },
    { id: 'install',      label: 'Installation' },
    { id: 'quickstart',   label: 'Quickstart' },
    { id: 'concepts',     label: 'Core concepts' },
    { id: 'vs-docker',    label: 'Crush vs Docker' },
    { id: 'detection',    label: 'Stack detection' },
    { id: 'crushfile',    label: 'The Crushfile' },
    { id: 'compose',      label: 'Docker Compose' },
    { id: 'commands',     label: 'Command reference' },
    { id: 'services',     label: 'Native services' },
    { id: 'networking',   label: 'Networking' },
    { id: 'volumes',      label: 'Volumes & data' },
    { id: 'images',       label: 'Images & registries' },
    { id: 'exec',         label: 'Running commands' },
    { id: 'deploy',       label: 'Deploying' },
    { id: 'previews',     label: 'Branch previews' },
    { id: 'ai',           label: 'AI diagnosis' },
    { id: 'security',     label: 'Security & SBOM' },
    { id: 'gui',          label: 'GUI tour' },
    { id: 'shortcuts',    label: 'Keyboard shortcuts' },
    { id: 'faq',          label: 'Troubleshooting & FAQ' },
    { id: 'glossary',     label: 'Glossary' },
    { id: 'resources',    label: 'Resources' },
  ];

  // ── Scrollspy: highlight the TOC entry for the section in view ──────────
  let activeId = $state(TOC[0].id);

  onMount(() => {
    const sections = TOC
      .map((t) => document.getElementById(t.id))
      .filter((el): el is HTMLElement => !!el);
    const visible = new Set<string>();
    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) visible.add(e.target.id);
          else visible.delete(e.target.id);
        }
        // The topmost section (in TOC order) that's currently near the top wins.
        const first = TOC.find((t) => visible.has(t.id));
        if (first) activeId = first.id;
      },
      // Trigger when a section reaches the top ~30% of the viewport.
      { rootMargin: '0px 0px -70% 0px', threshold: 0 },
    );
    sections.forEach((s) => obs.observe(s));
    return () => obs.disconnect();
  });

  function goTo(e: MouseEvent, id: string) {
    e.preventDefault();
    activeId = id;
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  // ── Commands, grouped by area ────────────────────────────────────────
  const CMD_GROUPS = [
    {
      group: 'Project lifecycle',
      items: [
        { cmd: 'crush run [path]',       desc: 'Auto-detect the stack at the given path (default: current dir), start native services it needs, build if necessary, and run the project. This is the everyday command.' },
        { cmd: 'crush run --dev',        desc: 'Development mode: mounts your source tree and enables hot-reload / watch for Node, Bun, Deno, and Python. The dev server stays attached to your terminal.' },
        { cmd: 'crush run --port 8080',  desc: 'Override the detected port. Useful when the auto-detected port clashes with something already running.' },
        { cmd: 'crush watch',            desc: 'Run with blue-green hot-swap: on file change, a new instance is built and traffic is switched once it is healthy — zero-downtime local reloads.' },
        { cmd: 'crush detect [path]',    desc: 'Print what Crush detects (language, framework, package manager, port, services) without running anything. Great for debugging detection.' },
        { cmd: 'crush stop <name>',      desc: 'Gracefully stop a running project/container (SIGTERM, then SIGKILL after a timeout). Pass --timeout to change the grace period.' },
        { cmd: 'crush ps [-a]',          desc: 'List running containers with ports, status, CPU and memory. Add -a to include stopped ones.' },
        { cmd: 'crush logs <name>',      desc: 'Show build + runtime logs. --follow streams live; --tail N limits history; --errors filters to stderr/error lines only.' },
      ],
    },
    {
      group: 'Working inside containers',
      items: [
        { cmd: 'crush exec <name> <cmd>', desc: 'Run a command in a running container’s working directory and environment. Inherits your terminal so interactive tools work.' },
        { cmd: 'crush exec -it <name> sh', desc: 'Open an interactive shell inside a running container — the equivalent of docker exec -it.' },
        { cmd: 'crush exec -e K=V <name> <cmd>', desc: 'Run with extra environment variables; -w <dir> sets the working directory.' },
        { cmd: 'crush inspect <name>',   desc: 'Dump full container metadata as JSON: env, mounts, ports, resource limits, health, restart policy.' },
        { cmd: 'crush stats',            desc: 'Live stream of per-container CPU, memory, and I/O usage.' },
      ],
    },
    {
      group: 'Images & registries',
      items: [
        { cmd: 'crush images',           desc: 'List local OCI images with real, de-duplicated sizes from the content-addressed blob store.' },
        { cmd: 'crush pull <image>',     desc: 'Pull any OCI image from Docker Hub, GHCR, or a private registry. Multi-arch manifests resolve to your platform automatically.' },
        { cmd: 'crush push <image>',     desc: 'Push a local image to any OCI-compatible registry. Run crush login first for private registries.' },
        { cmd: 'crush tag <src> <dst>',  desc: 'Add a new tag/reference to a local image without copying any blobs.' },
        { cmd: 'crush export <image> -o out.tar.gz', desc: 'Export an image to an OCI tarball that a Docker user can `docker load` and run.' },
        { cmd: 'crush rmi <image>',      desc: 'Remove a local image. Blobs shared with other images are kept; orphaned blobs are freed.' },
        { cmd: 'crush login <registry>', desc: 'Authenticate to a registry (Docker Hub, GHCR, ECR, GCR, …). Credentials are stored for pull/push.' },
        { cmd: 'crush scan <image>',     desc: 'Scan an image for CVEs and outdated packages locally — no data leaves your machine.' },
        { cmd: 'crush sbom <image>',     desc: 'Generate a Software Bill of Materials (SPDX/CycloneDX) listing every package in the image.' },
      ],
    },
    {
      group: 'Services, networks & volumes',
      items: [
        { cmd: 'crush services ls',      desc: 'List native services (Postgres, Redis, MongoDB, MinIO) and their status, ports, and connection strings.' },
        { cmd: 'crush services start <svc>', desc: 'Start a native service. The connection string is injected into your project as an env var.' },
        { cmd: 'crush services stop <svc>',  desc: 'Stop a native service. Data persists in its volume unless you remove it.' },
        { cmd: 'crush network ls',       desc: 'List virtual networks. Each project gets an isolated network so services resolve each other by name.' },
        { cmd: 'crush volume ls',        desc: 'List persistent volumes. Volumes survive restarts and removals; bind mounts point straight at host paths.' },
      ],
    },
    {
      group: 'Compose',
      items: [
        { cmd: 'crush compose up [-d]',  desc: 'Read docker-compose.yml / compose.yml and start every service in dependency order. -d runs detached.' },
        { cmd: 'crush compose down',     desc: 'Stop and remove the compose project’s containers and networks.' },
        { cmd: 'crush compose ps',       desc: 'Show the status of services in the current compose project.' },
      ],
    },
    {
      group: 'Deploy & escape hatches',
      items: [
        { cmd: 'crush deploy',           desc: 'Build and deploy to a cloud target. Crush suggests one from your stack; pass --target to choose.' },
        { cmd: 'crush deploy --target railway', desc: 'Deploy to a specific provider (railway, fly, gcp, aws, digitalocean, hetzner, ssh, …).' },
        { cmd: 'crush eject',            desc: 'Write a Dockerfile + docker-compose.yml for the detected stack so you can leave Crush anytime — zero lock-in.' },
      ],
    },
    {
      group: 'System',
      items: [
        { cmd: 'crush prune',            desc: 'Remove stopped containers, dangling images, and unused networks/volumes.' },
        { cmd: 'crush prune --all',      desc: 'Also remove all unused images and volumes — frees the most disk.' },
        { cmd: 'crush system',           desc: 'Show disk usage, data directory location, and daemon status.' },
        { cmd: 'crush health',           desc: 'Run a self-check: data dir writable, ports free, native service binaries present.' },
        { cmd: 'crush update',           desc: 'Self-update to the latest release. On Windows, installs to %LOCALAPPDATA%\\crush\\bin\\.' },
        { cmd: 'crush --version',        desc: 'Print the installed version, OS, and architecture.' },
      ],
    },
  ];

  const STACKS = [
    { tech: 'node',   label: 'Node.js',    note: 'Reads package.json → chooses npm / pnpm / yarn / bun from the lockfile. Picks the right script (dev → start → serve). Detects Express, Fastify, NestJS, Koa, Hono and more.' },
    { tech: 'bun',    label: 'Bun',        note: 'Triggered by bun.lockb or an engines.bun field. Uses `bun run`; sub-millisecond HMR in dev mode.' },
    { tech: 'deno',   label: 'Deno',       note: 'deno.json / deno.jsonc detection. Runs with the permissions declared in your tasks.' },
    { tech: 'python', label: 'Python',     note: 'requirements.txt / pyproject.toml / Pipfile. Supports Flask, FastAPI, Django, Uvicorn, Gunicorn — picks the ASGI/WSGI entrypoint and port.' },
    { tech: 'rust',   label: 'Rust',       note: 'Cargo.toml detection. Builds release mode and caches the target/ layer between runs for fast rebuilds.' },
    { tech: 'go',     label: 'Go',         note: 'go.mod detection. Compiles and strips debug info for a small static binary.' },
    { tech: 'java',   label: 'Java / JVM', note: 'Maven (pom.xml) and Gradle (build.gradle). Builds the fat jar and detects the Spring Boot / Quarkus port.' },
    { tech: 'dotnet', label: '.NET',       note: '*.csproj / *.sln. Publishes self-contained and reads the ASP.NET port from launchSettings.json.' },
    { tech: 'php',    label: 'PHP',        note: 'composer.json detection. Runs `php artisan serve` for Laravel or `php -S` for plain projects.' },
    { tech: 'ruby',   label: 'Ruby',       note: 'Gemfile detection. Rails, Sinatra and Hanami supported; runs the bundled server.' },
    { tech: 'react',  label: 'React / Vite / Next', note: 'Framework chip shown on the dashboard. Dev mode proxies HMR; production builds are served statically or SSR as appropriate.' },
    { tech: 'svelte', label: 'Svelte / SvelteKit', note: 'SSR builds use adapter-node; static builds are served by a fast file server.' },
  ];

  const SERVICES = [
    { tech: 'postgres', name: 'PostgreSQL', env: 'DATABASE_URL', port: '5432', desc: 'A portable Postgres binary runs directly on your machine — no container. The connection string is injected as DATABASE_URL. Connect with psql, pgAdmin, TablePlus, or any client. Data persists in a named volume across restarts.' },
    { tech: 'redis',    name: 'Redis',      env: 'REDIS_URL',    port: '6379', desc: 'A Redis-wire-compatible server (Garnet/Redis). Injected as REDIS_URL. Inspect keys live in the Services tab; works with redis-cli and every Redis SDK.' },
    { tech: 'mongodb',  name: 'MongoDB',    env: 'MONGO_URL',    port: '27017', desc: 'A portable mongod. Injected as MONGO_URL. The Services tab shows database stats and collection counts; use mongosh or Compass to browse.' },
    { tech: 'minio',    name: 'MinIO (S3)', env: 'S3_ENDPOINT',  port: '9000/9001', desc: 'S3-compatible object storage. Injects S3_ENDPOINT, S3_ACCESS_KEY, S3_SECRET_KEY. The web console runs on :9001; the S3 API on :9000. Works with the AWS SDK and aws-cli.' },
  ];

  const DEPLOY_TARGETS = [
    { name: 'Railway',                 note: 'Wraps the railway CLI. Detects services and provisions add-ons.' },
    { name: 'Fly.io',                  note: 'Wraps flyctl. Generates fly.toml from your stack on first deploy.' },
    { name: 'Google Cloud Run',        note: 'Wraps gcloud. Pushes to Artifact Registry, deploys a fully-managed service.' },
    { name: 'AWS App Runner',          note: 'Wraps aws-cli. Pushes to ECR and creates an App Runner service.' },
    { name: 'Azure Container Apps',    note: 'Wraps az. Pushes to ACR and deploys a container app.' },
    { name: 'DigitalOcean App Platform', note: 'Wraps doctl. Creates an app spec and deploys.' },
    { name: 'Render',                  note: 'Deploys via Render’s API / blueprint.' },
    { name: 'Vercel',                  note: 'Wraps the vercel CLI for frontend / serverless targets.' },
    { name: 'Netlify',                 note: 'Wraps the netlify CLI for static and edge targets.' },
    { name: 'Hetzner VPS',             note: 'Registry-free: ships the image over SSH and runs it on a plain VPS.' },
    { name: 'Generic SSH / registry',  note: 'Push to any OCI registry, or copy + run over SSH to any Linux host.' },
  ];

  const CRUSHFILE_FIELDS = [
    { key: 'runtime',   val: 'node | bun | python | rust | go | java | dotnet | php | ruby', desc: 'Force a runtime instead of auto-detecting it.' },
    { key: 'version',   val: '"20"',                desc: 'Pin the runtime/toolchain version.' },
    { key: 'port',      val: '3000',                desc: 'The port your app listens on; used for the preview and port-mapping.' },
    { key: 'start',     val: 'pnpm run start',      desc: 'Production start command (overrides the detected one).' },
    { key: 'dev_start', val: 'pnpm run dev',        desc: 'Command used by `crush run --dev`.' },
    { key: 'build',     val: 'pnpm run build',      desc: 'Build step run before start, if any.' },
    { key: 'services',  val: '[postgres, redis]',   desc: 'Native services to start alongside the app; their URLs are injected as env vars.' },
    { key: 'env',       val: 'KEY: value',          desc: 'Extra environment variables passed to the app.' },
    { key: 'base_image',val: 'debian:bookworm-slim',desc: 'Base image used when building an OCI image for deploy/export.' },
    { key: 'healthcheck',val: 'curl -f localhost:3000/health', desc: 'Command used to decide readiness for watch/blue-green and deploy.' },
  ];

  const COMPOSE_HONORED = [
    { key: 'image / build', state: 'full',    note: 'Pulls images; for build:, reads the Dockerfile’s base image and builds.' },
    { key: 'ports',         state: 'full',    note: 'HOST:CONTAINER and bare PORT forms, with /tcp|/udp suffixes.' },
    { key: 'environment',   state: 'full',    note: 'Both list (- KEY=val) and map (KEY: val) forms.' },
    { key: 'env_file',      state: 'full',    note: 'Loads variables from referenced .env files.' },
    { key: 'depends_on',    state: 'full',    note: 'Starts services in dependency order; detects cycles.' },
    { key: 'command / entrypoint', state: 'full', note: 'Service-level overrides replace the image’s, shell and exec forms.' },
    { key: 'volumes',       state: 'full',    note: 'Named volumes (persistent) and bind mounts; :ro honored.' },
    { key: 'networks',      state: 'full',    note: 'Services resolve peers by name only if they share a network.' },
    { key: 'restart',       state: 'full',    note: 'no | always | on-failure | unless-stopped.' },
    { key: 'healthcheck',   state: 'full',    note: 'CMD / CMD-SHELL / string forms, interval/timeout/retries.' },
    { key: 'deploy.resources', state: 'full', note: 'limits.cpus → CPU weight, limits.memory → memory cap.' },
    { key: 'container_name', state: 'full',   note: 'Uses your fixed name instead of a generated one.' },
  ];

  const FAQ = [
    { q: 'Do I need Docker Desktop or WSL2?', a: 'No. Crush runs your project natively and runs databases as portable binaries. Docker is only touched at the edges — pulling images, exporting an image a Docker user can run, and pushing to registries.' },
    { q: 'Where does Crush store its data?', a: 'On Linux: /var/lib/crush. On Windows: %PROGRAMDATA%\\Crush. On macOS/other: ~/.crush. It holds the image blob store, container state, native-service data, and volumes. Run `crush system` to see paths and usage.' },
    { q: 'A port is already in use — what do I do?', a: 'Pass crush run --port <free-port>, or stop whatever is holding it. `crush ps` and `crush services ls` show what Crush itself is running.' },
    { q: 'Can my teammates who use Docker run what I build?', a: 'Yes — that is the point. `crush export <image> -o app.tar.gz` produces an OCI tarball they can `docker load` and `docker run`. Or `crush push` to a registry they `docker pull` from.' },
    { q: 'How do I leave Crush / avoid lock-in?', a: 'Run `crush eject`. It writes a Dockerfile and docker-compose.yml for your detected stack so you can move to plain Docker with no rewrite.' },
    { q: 'My stack was detected wrong.', a: 'Run `crush detect` to see what it found, then drop a Crushfile in the project root to override the runtime, port, start command, or services.' },
    { q: 'How do I update Crush?', a: 'Run `crush update`, or download the latest release from GitHub. The GUI shows your version under Settings → About.' },
    { q: 'Does `crush run` create a container?', a: 'For local dev it runs your toolchain natively (fast, no isolation overhead). When you build/export/deploy, Crush produces a real OCI image. Containers appear in the Containers tab when you run an image directly.' },
  ];

  const GLOSSARY = [
    { term: 'OCI image', def: 'The open standard (Open Container Initiative) format Docker also uses. Crush stores, pulls, builds, and exports OCI images so it interoperates with the Docker ecosystem.' },
    { term: 'Content-addressed blob store', def: 'Image layers are stored by their sha256 hash. Identical layers are stored once and shared across images, so disk usage stays low.' },
    { term: 'Native service', def: 'A database/storage server (Postgres, Redis, MongoDB, MinIO) run as a portable binary on your host — no container or VM.' },
    { term: 'Crushfile', def: 'An optional per-project file that overrides auto-detection (runtime, port, commands, services, env).' },
    { term: 'Eject', def: 'Generating a Dockerfile + compose.yml from your project so you can use plain Docker instead of Crush.' },
    { term: 'Blue-green run', def: 'Starting a new instance, waiting for it to pass its healthcheck, then switching traffic — used by crush watch for zero-downtime reloads.' },
    { term: 'Bind mount vs volume', def: 'A bind mount points at a real host directory; a named volume is managed storage under the Crush data dir that persists across restarts and removals.' },
  ];
</script>

<div class="docs">
  <!-- Sticky table of contents -->
  <aside class="toc">
    <div class="toc-title">On this page</div>
    <nav>
      {#each TOC as t}
        <a href={`#${t.id}`} class:active={activeId === t.id} onclick={(e) => goTo(e, t.id)}>{t.label}</a>
      {/each}
    </nav>
  </aside>

  <div class="page">
    <header class="page-header">
      <h1>Crush Documentation</h1>
      <p class="subtitle">The complete guide to running, debugging, sharing, and shipping projects with Crush — native-first, Docker-compatible.</p>
    </header>

    <!-- Introduction -->
    <section id="intro" class="crush-card sec">
      <div class="sec-head"><Icon name="rocket" size={15} /><h2>Introduction</h2></div>
      <p class="prose">Crush is a developer tool for running and shipping applications <strong>without containers in your day-to-day loop</strong>. You point it at a project folder, it figures out the stack, starts whatever databases the app needs as native binaries, and runs it. When you are ready to share or deploy, Crush produces a real OCI image that anyone using Docker can run — and can deploy that image to a dozen cloud targets.</p>
      <p class="prose">The philosophy is simple: <strong>native execution for speed, Docker compatibility at the edges for collaboration.</strong> Locally there is no daemon to babysit, no VM, no Docker Desktop. But because Crush speaks the OCI standard, the rest of the world — your teammates, CI, and the cloud — never has to know you are not using Docker.</p>
      <div class="callout">
        <Icon name="box" size={14} />
        <div>
          <strong>Three things Crush gives you</strong>
          <ul class="tight">
            <li><strong>Run anything natively</strong> — auto-detected stacks, native databases, instant start.</li>
            <li><strong>Work with Docker users</strong> — pull, build, export, and push OCI images and compose files.</li>
            <li><strong>Deploy with one command</strong> — to Railway, Fly, Cloud Run, App Runner, a VPS, and more.</li>
          </ul>
        </div>
      </div>
    </section>

    <!-- Installation -->
    <section id="install" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Installation</h2></div>
      <p class="sec-desc">Crush ships as a single binary plus an optional desktop GUI. Pick your platform.</p>

      <h3 class="sub">Windows</h3>
      <p class="prose">Download the GUI installer (<code class="inline-code">.msi</code> or setup <code class="inline-code">.exe</code>) from the releases page and run it — it installs the app and the <code class="inline-code">crush</code> CLI, and adds it to your PATH. For the CLI only, drop <code class="inline-code">crush.exe</code> anywhere on your PATH.</p>
      <div class="cmd"><code>crush --version</code><button class="copy" onclick={() => copy('crush --version', 'i-win')}>{copied === 'i-win' ? 'Copied' : 'Copy'}</button></div>

      <h3 class="sub">macOS &amp; Linux</h3>
      <p class="prose">Download the binary for your architecture (or the <code class="inline-code">.deb</code> / AppImage on Linux), make it executable, and put it on your PATH:</p>
      <div class="cmd"><code>chmod +x crush &amp;&amp; sudo mv crush /usr/local/bin/</code><button class="copy" onclick={() => copy('chmod +x crush && sudo mv crush /usr/local/bin/', 'i-nix')}>{copied === 'i-nix' ? 'Copied' : 'Copy'}</button></div>

      <h3 class="sub">Verify &amp; update</h3>
      <p class="prose">Confirm the install with <code class="inline-code">crush health</code> (checks data dir, ports, and service binaries). Keep it current with:</p>
      <div class="cmd"><code>crush update</code><button class="copy" onclick={() => copy('crush update', 'i-upd')}>{copied === 'i-upd' ? 'Copied' : 'Copy'}</button></div>
      <p class="note">The GUI shows your installed version under <strong>Settings → About</strong>, and offers a one-click check for newer releases.</p>
    </section>

    <!-- Quickstart -->
    <section id="quickstart" class="crush-card sec">
      <div class="sec-head"><Icon name="rocket" size={15} /><h2>Quickstart</h2></div>
      <ol class="steps">
        <li><span class="step-n">1</span> Point Crush at any project folder — it auto-detects the stack:
          <div class="cmd"><code>crush run ./my-app</code><button class="copy" onclick={() => copy('crush run ./my-app', 'q1')}>{copied === 'q1' ? 'Copied' : 'Copy'}</button></div>
        </li>
        <li><span class="step-n">2</span> Crush installs dependencies, starts any native services it needs, and runs the dev server. The dashboard shows a live preview with the port, status, and streaming logs.</li>
        <li><span class="step-n">3</span> Need a database? Add one without Docker — its URL is injected automatically:
          <div class="cmd"><code>crush services start postgres</code><button class="copy" onclick={() => copy('crush services start postgres', 'q3')}>{copied === 'q3' ? 'Copied' : 'Copy'}</button></div>
        </li>
        <li><span class="step-n">4</span> Share a runnable image with a teammate who uses Docker:
          <div class="cmd"><code>crush export my-app -o my-app.tar.gz</code><button class="copy" onclick={() => copy('crush export my-app -o my-app.tar.gz', 'q4')}>{copied === 'q4' ? 'Copied' : 'Copy'}</button></div>
        </li>
        <li><span class="step-n">5</span> Ship it — Crush picks the right cloud target for your stack:
          <div class="cmd"><code>crush deploy</code><button class="copy" onclick={() => copy('crush deploy', 'q5')}>{copied === 'q5' ? 'Copied' : 'Copy'}</button></div>
        </li>
      </ol>
      <p class="note">That is the whole loop: <strong>run → add services → share → deploy</strong> — with no Docker Desktop and no WSL2.</p>
    </section>

    <!-- Core concepts -->
    <section id="concepts" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Core concepts</h2></div>

      <h3 class="sub">The native dev loop</h3>
      <p class="prose">When you <code class="inline-code">crush run</code> a project, Crush executes your real toolchain (<code class="inline-code">node</code>, <code class="inline-code">python</code>, <code class="inline-code">cargo</code>, …) directly on your machine. There is no container, no syscall translation, and no virtual machine — so startup is instant and file watching is native. On Windows the process tree is grouped so a single stop cleanly tears everything down; on Linux/macOS it runs in its own process group.</p>

      <h3 class="sub">Native services</h3>
      <p class="prose">Instead of <code class="inline-code">docker run postgres</code>, Crush downloads and runs a portable Postgres (or Redis, MongoDB, MinIO) binary and injects the connection string into your app. You get a real database in seconds, with data that persists in a managed volume.</p>

      <h3 class="sub">The image store</h3>
      <p class="prose">Crush keeps a <strong>content-addressed blob store</strong>: every image layer is saved under its sha256 hash, so layers shared between images are stored only once. <code class="inline-code">crush images</code> shows the real, de-duplicated sizes; <code class="inline-code">crush prune --all</code> reclaims space.</p>

      <h3 class="sub">Docker interop at the edges</h3>
      <p class="prose">Crush <em>consumes</em> the Docker world (pull images, read compose files) and <em>produces</em> for it (build/export/push OCI images). That boundary is what lets you collaborate with Docker users and deploy anywhere, while never running Docker yourself.</p>
    </section>

    <!-- vs Docker -->
    <section id="vs-docker" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Crush vs Docker</h2></div>
      <p class="sec-desc">Crush is not a Docker replacement for everything — it is a faster local workflow that stays compatible with Docker where it matters.</p>
      <div class="table">
        <div class="trow thead"><span>Topic</span><span>Docker</span><span>Crush</span></div>
        <div class="trow"><span>Local run</span><span>Container on a daemon/VM</span><span>Native process — no daemon</span></div>
        <div class="trow"><span>Databases</span><span>Containers you wire up</span><span>Native binaries, URL auto-injected</span></div>
        <div class="trow"><span>Image format</span><span>OCI</span><span>OCI (same standard)</span></div>
        <div class="trow"><span>Compose</span><span>docker compose</span><span>Reads the same compose files</span></div>
        <div class="trow"><span>Share with others</span><span>push / save</span><span>push / export (they docker load)</span></div>
        <div class="trow"><span>Deploy</span><span>Manual per provider</span><span>One command, many targets</span></div>
        <div class="trow"><span>Lock-in</span><span>—</span><span>crush eject → Dockerfile + compose</span></div>
      </div>
    </section>

    <!-- Detection -->
    <section id="detection" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Stack detection</h2></div>
      <p class="sec-desc">Crush reads your project files and picks the runtime, package manager, start command, and port — no config required. Run <code class="inline-code">crush detect</code> to see exactly what it found.</p>
      <div class="stack-grid">
        {#each STACKS as s}
          <div class="stack-row">
            <TechIcon name={s.tech} size={18} />
            <div>
              <span class="stack-name">{s.label}</span>
              <span class="stack-note">{s.note}</span>
            </div>
          </div>
        {/each}
      </div>
      <p class="note">Monorepos are detected via workspaces (npm/pnpm/yarn, Cargo) or a compose file — each service gets its own build, network alias, and log stream. Override anything with a <code class="inline-code">Crushfile</code>.</p>
    </section>

    <!-- Crushfile -->
    <section id="crushfile" class="crush-card sec">
      <div class="sec-head"><Icon name="services" size={15} /><h2>The Crushfile</h2></div>
      <p class="sec-desc">Drop a <code class="inline-code">Crushfile</code> in your project root to override any detected value. Every field is optional — set only what you need.</p>
      <div class="cmd code-block">
        <code>{`runtime: node
version: "20"
port: 3000
build: pnpm run build
start: pnpm run start
dev_start: pnpm run dev

services:
  - postgres
  - redis

env:
  NODE_ENV: production
  LOG_LEVEL: info

base_image: debian:bookworm-slim
healthcheck: curl -f http://localhost:3000/health`}</code>
        <button class="copy" onclick={() => copy('runtime: node\nversion: "20"\nport: 3000\nbuild: pnpm run build\nstart: pnpm run start\ndev_start: pnpm run dev\n\nservices:\n  - postgres\n  - redis\n\nenv:\n  NODE_ENV: production\n  LOG_LEVEL: info\n\nbase_image: debian:bookworm-slim\nhealthcheck: curl -f http://localhost:3000/health', 'cf')}>{copied === 'cf' ? 'Copied' : 'Copy'}</button>
      </div>
      <div class="table mt">
        <div class="trow thead"><span>Field</span><span>Example</span><span>What it does</span></div>
        {#each CRUSHFILE_FIELDS as f}
          <div class="trow"><span class="mono">{f.key}</span><span class="mono dim">{f.val}</span><span>{f.desc}</span></div>
        {/each}
      </div>
    </section>

    <!-- Compose -->
    <section id="compose" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Docker Compose compatibility</h2></div>
      <p class="sec-desc">Already have a <code class="inline-code">docker-compose.yml</code>? Crush reads it directly — no rewrite. <code class="inline-code">crush compose up</code> starts everything in dependency order.</p>
      <div class="cmd"><code>crush compose up -d</code><button class="copy" onclick={() => copy('crush compose up -d', 'co1')}>{copied === 'co1' ? 'Copied' : 'Copy'}</button></div>
      <div class="table mt">
        <div class="trow thead"><span>Compose key</span><span>Support</span><span>Notes</span></div>
        {#each COMPOSE_HONORED as c}
          <div class="trow">
            <span class="mono">{c.key}</span>
            <span><span class="pill {c.state}">{c.state === 'full' ? 'Honored' : c.state}</span></span>
            <span>{c.note}</span>
          </div>
        {/each}
      </div>
      <p class="note">Service-to-service references resolve by name when services share a network (e.g. <code class="inline-code">DB_HOST: postgres_db</code>). Named volumes persist under the Crush data dir; bind mounts point at host paths.</p>
    </section>

    <!-- Command reference -->
    <section id="commands" class="crush-card sec">
      <div class="sec-head"><Icon name="logs" size={15} /><h2>Command reference</h2></div>
      <p class="sec-desc">The full CLI, grouped by what you are doing. Square brackets are optional; angle brackets are required.</p>
      {#each CMD_GROUPS as g}
        <h3 class="sub">{g.group}</h3>
        <div class="cmds">
          {#each g.items as c}
            <div class="cmd-row">
              <code class="cmd-name">{c.cmd}</code>
              <span class="cmd-desc">{c.desc}</span>
              <button class="copy sm" onclick={() => copy(c.cmd, c.cmd)}>{copied === c.cmd ? '✓' : 'Copy'}</button>
            </div>
          {/each}
        </div>
      {/each}
    </section>

    <!-- Native services -->
    <section id="services" class="crush-card sec">
      <div class="sec-head"><Icon name="services" size={15} /><h2>Native services</h2></div>
      <p class="sec-desc">Crush runs portable database and storage binaries directly on your machine — no Docker, no VM. Connection details are injected as environment variables automatically.</p>
      <div class="svc-grid">
        {#each SERVICES as s}
          <div class="svc-card">
            <div class="svc-head"><TechIcon name={s.tech} size={18} /><h3>{s.name}</h3></div>
            <div class="svc-meta">
              <span class="kv"><span class="kk">env</span><code>{s.env}</code></span>
              <span class="kv"><span class="kk">port</span><code>{s.port}</code></span>
            </div>
            <p>{s.desc}</p>
          </div>
        {/each}
      </div>
      <div class="cmd-block">
        <div class="cmd"><code>crush services start postgres</code><button class="copy" onclick={() => copy('crush services start postgres', 'svc1')}>{copied === 'svc1' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush services ls</code><button class="copy" onclick={() => copy('crush services ls', 'svc2')}>{copied === 'svc2' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush services stop postgres</code><button class="copy" onclick={() => copy('crush services stop postgres', 'svc3')}>{copied === 'svc3' ? 'Copied' : 'Copy'}</button></div>
      </div>
      <p class="note">List a service in your <code class="inline-code">Crushfile</code> (or compose file) and Crush starts it automatically on <code class="inline-code">crush run</code> — no separate command needed.</p>
    </section>

    <!-- Networking -->
    <section id="networking" class="crush-card sec">
      <div class="sec-head"><Icon name="branch" size={15} /><h2>Networking</h2></div>
      <p class="prose">Each project gets an isolated virtual network. Services on the same network can reach each other <strong>by name</strong> — so <code class="inline-code">DB_HOST: postgres_db</code> just works, exactly as it does under Docker Compose. Services that do not share a network stay isolated from one another.</p>
      <p class="prose">Ports you expose (via <code class="inline-code">--port</code>, a Crushfile, or compose <code class="inline-code">ports:</code>) are mapped to your host so you can open the app in a browser. Use <code class="inline-code">crush network ls</code> to see the networks Crush manages.</p>
    </section>

    <!-- Volumes -->
    <section id="volumes" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Volumes &amp; data persistence</h2></div>
      <p class="prose">A <strong>named volume</strong> is managed storage that lives under the Crush data dir and survives restarts, rebuilds, and removals — ideal for database data. A <strong>bind mount</strong> points directly at a folder on your machine — ideal for mounting source code or config. Both are declared in a compose file’s <code class="inline-code">volumes:</code> list (<code class="inline-code">name:/path</code> for a named volume, <code class="inline-code">./host:/path</code> for a bind mount, with an optional <code class="inline-code">:ro</code> for read-only).</p>
      <div class="cmd"><code>crush volume ls</code><button class="copy" onclick={() => copy('crush volume ls', 'vol1')}>{copied === 'vol1' ? 'Copied' : 'Copy'}</button></div>
      <p class="note">Removing a container never deletes a named volume. To reclaim that space deliberately, use <code class="inline-code">crush prune --all</code>.</p>
    </section>

    <!-- Images & registries -->
    <section id="images" class="crush-card sec">
      <div class="sec-head"><Icon name="images" size={15} /><h2>Images &amp; registries</h2></div>
      <p class="prose">Crush pulls, builds, stores, and pushes standard OCI images. Because the store is content-addressed, pulling a layer you already have costs zero extra disk. Multi-arch manifests resolve to your platform automatically.</p>
      <h3 class="sub">Share a runnable image with Docker users</h3>
      <p class="prose">This is the core of working with people who use Docker. Export an OCI tarball they can load and run directly:</p>
      <div class="cmd-block">
        <div class="cmd"><code>crush export my-app -o my-app.tar.gz</code><button class="copy" onclick={() => copy('crush export my-app -o my-app.tar.gz', 'im1')}>{copied === 'im1' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code># on their machine:  docker load -i my-app.tar.gz &amp;&amp; docker run my-app</code><button class="copy" onclick={() => copy('docker load -i my-app.tar.gz && docker run my-app', 'im2')}>{copied === 'im2' ? 'Copied' : 'Copy'}</button></div>
      </div>
      <h3 class="sub">Or push to a registry</h3>
      <div class="cmd-block">
        <div class="cmd"><code>crush login ghcr.io</code><button class="copy" onclick={() => copy('crush login ghcr.io', 'im3')}>{copied === 'im3' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush push ghcr.io/you/my-app:latest</code><button class="copy" onclick={() => copy('crush push ghcr.io/you/my-app:latest', 'im4')}>{copied === 'im4' ? 'Copied' : 'Copy'}</button></div>
      </div>
    </section>

    <!-- exec -->
    <section id="exec" class="crush-card sec">
      <div class="sec-head"><Icon name="logs" size={15} /><h2>Running commands in a container</h2></div>
      <p class="prose"><code class="inline-code">crush exec</code> runs a command in a running container’s working directory and environment, inheriting your terminal so interactive tools work — the familiar <code class="inline-code">docker exec</code> experience.</p>
      <div class="cmd-block">
        <div class="cmd"><code>crush exec -it my-app sh</code><button class="copy" onclick={() => copy('crush exec -it my-app sh', 'ex1')}>{copied === 'ex1' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush exec my-app npm run migrate</code><button class="copy" onclick={() => copy('crush exec my-app npm run migrate', 'ex2')}>{copied === 'ex2' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush exec -e DEBUG=1 -w /srv my-app ./task.sh</code><button class="copy" onclick={() => copy('crush exec -e DEBUG=1 -w /srv my-app ./task.sh', 'ex3')}>{copied === 'ex3' ? 'Copied' : 'Copy'}</button></div>
      </div>
      <p class="note">You can reference a container by its name, full id, or a short id prefix — just like Docker.</p>
    </section>

    <!-- Deploy -->
    <section id="deploy" class="crush-card sec">
      <div class="sec-head"><Icon name="rocket" size={15} /><h2>Deploying</h2></div>
      <p class="sec-desc">Crush builds an image and ships it to your chosen target, wrapping each provider’s official CLI/API. Stack detection filters targets so static sites never see backend-only options.</p>
      <div class="dep-grid">
        {#each DEPLOY_TARGETS as t}
          <div class="dep-row"><span class="dep-name">{t.name}</span><span class="dep-note">{t.note}</span></div>
        {/each}
      </div>
      <div class="cmd-block">
        <div class="cmd"><code>crush deploy --target fly</code><button class="copy" onclick={() => copy('crush deploy --target fly', 'dp1')}>{copied === 'dp1' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush eject</code><button class="copy" onclick={() => copy('crush eject', 'dp2')}>{copied === 'dp2' ? 'Copied' : 'Copy'}</button></div>
      </div>
      <p class="note">Most providers need their CLI installed and authenticated once (e.g. <code class="inline-code">flyctl auth login</code>). Check the <strong>Settings</strong> tab to see which provider CLIs Crush can find. <code class="inline-code">crush eject</code> writes a Dockerfile + compose so you can deploy by hand anytime.</p>
    </section>

    <!-- Branch previews -->
    <section id="previews" class="crush-card sec">
      <div class="sec-head"><Icon name="branch" size={15} /><h2>Branch previews</h2></div>
      <p class="prose">Preview any git branch in an isolated worktree — your working tree stays untouched. Each preview is a separate run, tagged with the branch name and commit hash, with its own port and logs. Open one from the project page in the GUI to review a teammate’s branch without stashing your work or switching branches.</p>
    </section>

    <!-- AI -->
    <section id="ai" class="crush-card sec">
      <div class="sec-head"><Icon name="logs" size={15} /><h2>AI log diagnosis</h2></div>
      <p class="prose">When a run fails, click <strong>Diagnose</strong> on the logs to get a plain-English explanation of the root cause and a suggested fix. Add an API key under <strong>Settings → AI</strong> — Crush supports Anthropic (Claude), OpenAI, and any OpenAI-compatible endpoint. The model reads the failing logs and your detected stack to give targeted advice.</p>
      <p class="note">Diagnosis is opt-in: nothing is sent anywhere until you add a key and click Diagnose.</p>
    </section>

    <!-- Security -->
    <section id="security" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Security scanning &amp; SBOM</h2></div>
      <p class="prose"><code class="inline-code">crush scan &lt;image&gt;</code> checks the packages in an image against a CVE database and lists severity, affected package, and the version that fixes it. <code class="inline-code">crush sbom &lt;image&gt;</code> generates a Software Bill of Materials (SPDX / CycloneDX) enumerating every package. Both run locally — no image contents leave your machine.</p>
      <div class="cmd-block">
        <div class="cmd"><code>crush scan my-app</code><button class="copy" onclick={() => copy('crush scan my-app', 'se1')}>{copied === 'se1' ? 'Copied' : 'Copy'}</button></div>
        <div class="cmd"><code>crush sbom my-app</code><button class="copy" onclick={() => copy('crush sbom my-app', 'se2')}>{copied === 'se2' ? 'Copied' : 'Copy'}</button></div>
      </div>
    </section>

    <!-- GUI tour -->
    <section id="gui" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>GUI tour</h2></div>
      <p class="sec-desc">The desktop app mirrors the CLI. Each tab maps to a number key (1–7).</p>
      <div class="stack-grid">
        <div class="stack-row"><Icon name="rocket" size={16} /><div><span class="stack-name">Overview (1)</span><span class="stack-note">Pick a project, run/stop it, watch the live preview, port, status, and logs. Shows the detected stack and git info before a run.</span></div></div>
        <div class="stack-row"><Icon name="box" size={16} /><div><span class="stack-name">Containers (2)</span><span class="stack-note">Containers started from images. Native dev runs appear on the dashboard’s preview rather than here.</span></div></div>
        <div class="stack-row"><Icon name="images" size={16} /><div><span class="stack-name">Images (3)</span><span class="stack-note">Local OCI images with real sizes and platform. Inspect, tag, and remove.</span></div></div>
        <div class="stack-row"><Icon name="services" size={16} /><div><span class="stack-name">Services (4)</span><span class="stack-note">Start/stop native databases, see their connection strings and live stats.</span></div></div>
        <div class="stack-row"><Icon name="logs" size={16} /><div><span class="stack-name">Logs (5)</span><span class="stack-note">Build and runtime logs with AI diagnosis on failures.</span></div></div>
        <div class="stack-row"><Icon name="rocket" size={16} /><div><span class="stack-name">Deployments (6)</span><span class="stack-note">Deploy to a cloud target and track deployment history.</span></div></div>
        <div class="stack-row"><Icon name="box" size={16} /><div><span class="stack-name">Settings (7)</span><span class="stack-note">AI keys, provider-CLI status, version/About, and updates.</span></div></div>
      </div>
    </section>

    <!-- Shortcuts -->
    <section id="shortcuts" class="crush-card sec">
      <div class="sec-head"><Icon name="rocket" size={15} /><h2>Keyboard shortcuts (GUI)</h2></div>
      <div class="shortcuts">
        <div class="sc-row"><kbd>R</kbd><span>Run / restart the current project</span></div>
        <div class="sc-row"><kbd>S</kbd><span>Stop the running project</span></div>
        <div class="sc-row"><kbd>Ctrl+K</kbd><span>Open the project picker</span></div>
        <div class="sc-row"><kbd>Esc</kbd><span>Close the detail pane / terminal</span></div>
        <div class="sc-row"><kbd>1–7</kbd><span>Jump to Overview, Containers, Images, Services, Logs, Deployments, Settings</span></div>
      </div>
    </section>

    <!-- FAQ -->
    <section id="faq" class="crush-card sec">
      <div class="sec-head"><Icon name="logs" size={15} /><h2>Troubleshooting &amp; FAQ</h2></div>
      <div class="faq">
        {#each FAQ as f}
          <details>
            <summary>{f.q}</summary>
            <p>{f.a}</p>
          </details>
        {/each}
      </div>
    </section>

    <!-- Glossary -->
    <section id="glossary" class="crush-card sec">
      <div class="sec-head"><Icon name="box" size={15} /><h2>Glossary</h2></div>
      <dl class="gloss">
        {#each GLOSSARY as g}
          <div class="gloss-row"><dt>{g.term}</dt><dd>{g.def}</dd></div>
        {/each}
      </dl>
    </section>

    <!-- Resources -->
    <section id="resources" class="crush-card sec">
      <div class="sec-head"><Icon name="github" size={15} /><h2>Resources</h2></div>
      <div class="res">
        <button class="res-btn" onclick={() => open(SITE)}><Icon name="github" size={14} /> GitHub repository</button>
        <button class="res-btn" onclick={() => open(`${SITE}#readme`)}><Icon name="logs" size={14} /> Full README</button>
        <button class="res-btn" onclick={() => open(`${SITE}/releases`)}><Icon name="box" size={14} /> Releases &amp; changelog</button>
        <button class="res-btn" onclick={() => open(`${SITE}/issues`)}><Icon name="logs" size={14} /> Report an issue</button>
      </div>
    </section>
  </div>
</div>

<style>
  .docs { display: flex; gap: 28px; align-items: flex-start; }

  /* Sticky TOC */
  .toc { position: sticky; top: 12px; flex-shrink: 0; width: 184px; max-height: calc(100vh - 24px); overflow-y: auto; padding: 4px 0; }
  .toc-title { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-crush-text-muted); padding: 0 8px 8px; }
  .toc nav { display: flex; flex-direction: column; }
  .toc a { font-size: 12.5px; color: var(--color-crush-text-muted); text-decoration: none; padding: 5px 8px; border-radius: 6px; border-left: 2px solid transparent; transition: color 0.12s, background 0.12s, border-color 0.12s; }
  .toc a:hover { color: var(--color-crush-text); background: rgba(255,255,255,0.04); border-left-color: var(--color-crush-orange); }
  /* Scrollspy active link */
  .toc a.active { color: var(--color-crush-text); background: rgba(255,255,255,0.05); border-left-color: var(--color-crush-orange); font-weight: 600; }

  .page { display: flex; flex-direction: column; gap: 22px; max-width: 860px; flex: 1; min-width: 0; }
  .page-header h1 { font-size: 22px; font-weight: 600; margin: 0; }
  .subtitle { font-size: 13.5px; color: var(--color-crush-text-muted); margin: 8px 0 0; line-height: 1.6; }

  .sec { padding: 26px 28px; scroll-margin-top: 14px; }
  .sec-head { display: flex; align-items: center; gap: 9px; margin-bottom: 18px; color: var(--color-crush-text-muted); }
  .sec-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; margin: 0; }
  .sec-desc { font-size: 13px; color: var(--color-crush-text-muted); margin: 0 0 18px; line-height: 1.6; }
  .sub { font-size: 13px; font-weight: 600; margin: 28px 0 10px; color: var(--color-crush-text); }

  .prose { font-size: 13.5px; line-height: 1.75; color: var(--color-crush-text); margin: 0 0 14px; }
  .prose:last-child { margin-bottom: 0; }
  .tight { margin: 10px 0 0; padding-left: 20px; }
  .tight li { font-size: 13px; line-height: 1.7; color: var(--color-crush-text); margin-bottom: 5px; }

  .callout { display: flex; gap: 12px; align-items: flex-start; margin-top: 14px; padding: 12px 14px; border: 1px solid var(--color-crush-border); border-radius: 8px; background: rgba(224,85,64,0.05); }
  .callout :global(svg) { margin-top: 2px; color: var(--color-crush-orange); flex-shrink: 0; }
  .callout strong { font-size: 13px; display: block; margin-bottom: 2px; }

  .steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
  .steps li { font-size: 13.5px; line-height: 1.55; color: var(--color-crush-text); }
  .step-n { display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 50%; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); font-size: 11px; font-family: var(--font-mono); margin-right: 8px; color: var(--color-crush-text-muted); }
  .note { font-size: 12px; color: var(--color-crush-text-muted); margin: 14px 0 0; line-height: 1.55; }

  .cmd { display: flex; align-items: flex-start; gap: 8px; margin: 8px 0 0; background: rgba(9,9,11,0.6); border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 8px 10px; }
  .cmd code { flex: 1; font-family: var(--font-mono); font-size: 12.5px; color: var(--color-crush-text); white-space: pre-wrap; word-break: break-word; }
  .code-block code { white-space: pre; overflow-x: auto; line-height: 1.6; }
  .copy { background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 6px; padding: 3px 10px; font-size: 11px; cursor: pointer; flex-shrink: 0; }
  .copy:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .copy.sm { padding: 2px 8px; }
  .cmd-block { display: flex; flex-direction: column; gap: 4px; margin-top: 12px; }
  .inline-code { font-family: var(--font-mono); font-size: 11.5px; background: rgba(255,255,255,0.07); border: 1px solid var(--color-crush-border); border-radius: 4px; padding: 1px 5px; }

  .cmds { display: flex; flex-direction: column; margin-bottom: 4px; }
  .cmd-row { display: grid; grid-template-columns: 230px 1fr auto; align-items: center; gap: 12px; padding: 9px 0; border-bottom: 1px solid rgba(42,42,53,0.4); }
  .cmd-row:last-child { border-bottom: none; }
  .cmd-name { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text); }
  .cmd-desc { font-size: 12.5px; color: var(--color-crush-text-muted); line-height: 1.45; }

  .stack-grid { display: flex; flex-direction: column; gap: 10px; }
  .stack-row { display: flex; align-items: flex-start; gap: 12px; padding: 8px 0; border-bottom: 1px solid rgba(42,42,53,0.3); }
  .stack-row:last-child { border-bottom: none; }
  .stack-row :global(svg) { flex-shrink: 0; margin-top: 2px; }
  .stack-name { font-size: 13px; font-weight: 500; display: block; margin-bottom: 2px; }
  .stack-note { font-size: 12px; color: var(--color-crush-text-muted); display: block; line-height: 1.45; }

  /* Tables */
  .table { display: flex; flex-direction: column; border: 1px solid var(--color-crush-border); border-radius: 8px; overflow: hidden; }
  .table.mt { margin-top: 12px; }
  .trow { display: grid; grid-template-columns: 150px 130px 1fr; gap: 12px; padding: 9px 12px; border-bottom: 1px solid rgba(42,42,53,0.4); font-size: 12.5px; align-items: center; }
  .trow:last-child { border-bottom: none; }
  .trow.thead { background: rgba(255,255,255,0.03); font-weight: 600; color: var(--color-crush-text-muted); text-transform: uppercase; font-size: 11px; letter-spacing: 0.04em; }
  .trow span { line-height: 1.45; }
  .mono { font-family: var(--font-mono); font-size: 11.5px; }
  .dim { color: var(--color-crush-text-muted); }
  .pill { font-size: 11px; padding: 1px 8px; border-radius: 9999px; }
  .pill.full { background: rgba(80,200,120,0.12); color: #6fcf97; border: 1px solid rgba(80,200,120,0.25); }

  /* vs-docker table uses 3 even-ish columns */
  #vs-docker .trow { grid-template-columns: 130px 1fr 1fr; }
  /* crushfile table */
  #crushfile .trow { grid-template-columns: 120px 1fr 1.4fr; }

  /* services */
  .svc-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .svc-card { border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 12px 14px; }
  .svc-head { display: flex; align-items: center; gap: 9px; margin-bottom: 8px; }
  .svc-head h3 { font-size: 13.5px; font-weight: 600; margin: 0; }
  .svc-head :global(svg) { flex-shrink: 0; }
  .svc-meta { display: flex; gap: 14px; margin-bottom: 8px; }
  .kv { display: inline-flex; align-items: center; gap: 6px; }
  .kk { font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-crush-text-muted); }
  .kv code { font-family: var(--font-mono); font-size: 11px; color: var(--color-crush-text); }
  .svc-card p { font-size: 12px; color: var(--color-crush-text-muted); margin: 0; line-height: 1.55; }

  /* deploy */
  .dep-grid { display: flex; flex-direction: column; gap: 0; border: 1px solid var(--color-crush-border); border-radius: 8px; overflow: hidden; }
  .dep-row { display: grid; grid-template-columns: 220px 1fr; gap: 12px; padding: 9px 12px; border-bottom: 1px solid rgba(42,42,53,0.4); align-items: center; }
  .dep-row:last-child { border-bottom: none; }
  .dep-name { font-size: 12.5px; font-weight: 500; }
  .dep-note { font-size: 12px; color: var(--color-crush-text-muted); line-height: 1.45; }

  .shortcuts { display: flex; flex-direction: column; gap: 6px; }
  .sc-row { display: flex; align-items: center; gap: 12px; font-size: 13px; padding: 6px 0; border-bottom: 1px solid rgba(42,42,53,0.3); }
  .sc-row:last-child { border-bottom: none; }
  kbd { font-family: var(--font-mono); font-size: 11px; padding: 2px 7px; border: 1px solid var(--color-crush-border); border-radius: 5px; background: var(--color-crush-surface); color: var(--color-crush-text); white-space: nowrap; }

  /* faq */
  .faq { display: flex; flex-direction: column; gap: 6px; }
  .faq details { border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 0 12px; }
  .faq summary { font-size: 13px; font-weight: 500; cursor: pointer; padding: 11px 0; color: var(--color-crush-text); list-style: none; }
  .faq summary::-webkit-details-marker { display: none; }
  .faq summary::before { content: '+'; display: inline-block; width: 16px; color: var(--color-crush-orange); font-family: var(--font-mono); }
  .faq details[open] summary::before { content: '–'; }
  .faq details p { font-size: 12.5px; color: var(--color-crush-text-muted); margin: 0 0 12px 16px; line-height: 1.6; }

  /* glossary */
  .gloss { margin: 0; display: flex; flex-direction: column; gap: 0; }
  .gloss-row { padding: 10px 0; border-bottom: 1px solid rgba(42,42,53,0.3); }
  .gloss-row:last-child { border-bottom: none; }
  .gloss dt { font-size: 13px; font-weight: 600; color: var(--color-crush-text); margin-bottom: 3px; }
  .gloss dd { font-size: 12.5px; color: var(--color-crush-text-muted); margin: 0; line-height: 1.6; }

  @media (max-width: 880px) {
    .toc { display: none; }
    .svc-grid { grid-template-columns: 1fr; }
    .cmd-row { grid-template-columns: 1fr; gap: 4px; }
    .cmd-row .copy { justify-self: start; }
    .dep-row, .trow, #vs-docker .trow, #crushfile .trow { grid-template-columns: 1fr; gap: 4px; }
  }
</style>
