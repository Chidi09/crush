<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import * as api from '$lib/tauri';

  const SITE = 'https://github.com/Chidi09/crush';

  let copied = $state<string | null>(null);
  async function copy(text: string, id: string) {
    try { await navigator.clipboard.writeText(text); copied = id; setTimeout(() => { if (copied === id) copied = null; }, 1400); } catch {}
  }
  function open(url: string) { api.openUrl(url).catch(() => {}); }

  const COMMANDS = [
    { cmd: 'crush run',             desc: 'Auto-detect the stack, build an image, and start the project. Equivalent to the dashboard Run button.' },
    { cmd: 'crush run --dev',       desc: 'Run in dev mode — mounts your source tree and enables HMR/watch for Node, Bun, Deno, and Python.' },
    { cmd: 'crush build',           desc: 'Build an optimised image without starting it. Use --force to bypass the cache.' },
    { cmd: 'crush ps',              desc: 'List all running containers, their ports, CPU and memory.' },
    { cmd: 'crush stop <name>',     desc: 'Stop a named container gracefully (SIGTERM → SIGKILL after timeout).' },
    { cmd: 'crush rm <name>',       desc: 'Remove a stopped container and its overlay filesystem.' },
    { cmd: 'crush logs <name>',     desc: 'Tail build + runtime logs. --follow streams live; --errors shows only stderr/error lines.' },
    { cmd: 'crush exec <name> sh',  desc: 'Open a shell inside a running container (like docker exec -it).' },
    { cmd: 'crush inspect <name>',  desc: 'Dump full container metadata — env, mounts, ports, resource limits.' },
    { cmd: 'crush images',          desc: 'List local OCI images with real sizes. Crush stores images in a content-addressed blob store.' },
    { cmd: 'crush pull <image>',    desc: 'Pull any OCI image from Docker Hub, GHCR, or a private registry.' },
    { cmd: 'crush rmi <image>',     desc: 'Remove a local image and free its blobs.' },
    { cmd: 'crush push <image>',    desc: 'Push a local image to any OCI-compatible registry.' },
    { cmd: 'crush tag <src> <dst>', desc: 'Tag a local image with a new reference without copying blobs.' },
    { cmd: 'crush export <image>',  desc: 'Export an image to an OCI tarball (.tar.gz) for transfer or archival.' },
    { cmd: 'crush scan <image>',    desc: 'Scan an image for CVEs and outdated packages using the embedded scanner.' },
    { cmd: 'crush services',        desc: 'List, start, stop and inspect native services (Postgres, Redis, MongoDB, MinIO).' },
    { cmd: 'crush deploy',          desc: 'Deploy to a cloud provider. Crush picks a target from your stack; pass --target to override.' },
    { cmd: 'crush eject',           desc: 'Write a Dockerfile + compose.yml for the detected stack — no lock-in.' },
    { cmd: 'crush update',          desc: 'Self-update to the latest release. On Windows, installs to %LOCALAPPDATA%\\crush\\bin\\.' },
    { cmd: 'crush prune',           desc: 'Remove stopped containers, dangling images, unused networks and volumes.' },
    { cmd: 'crush prune --all',     desc: 'Also remove all unused images and unused volumes (frees the most disk).' },
    { cmd: 'crush network ls',      desc: 'List virtual networks. Each project gets an isolated bridge network by default.' },
    { cmd: 'crush volume ls',       desc: 'List persistent volumes. Volumes survive container restarts; bind mounts do not.' },
  ];

  const STACKS = [
    { tech: 'node',   label: 'Node.js',    note: 'Detects package.json → chooses npm/pnpm/yarn/bun. Picks the right start script (dev / start / serve).' },
    { tech: 'bun',    label: 'Bun',        note: 'Uses bun run when bun.lockb or "bun" in engines. Sub-millisecond HMR in dev mode.' },
    { tech: 'python', label: 'Python',     note: 'Detects requirements.txt / pyproject.toml / Pipfile. Supports Flask, FastAPI, Django, Uvicorn, Gunicorn.' },
    { tech: 'rust',   label: 'Rust',       note: 'Cargo.toml detection. Builds in release mode, caches the target/ layer between runs.' },
    { tech: 'go',     label: 'Go',         note: 'go.mod detection. Compiles + strips debug info for a minimal image.' },
    { tech: 'java',   label: 'Java / JVM', note: 'Maven (pom.xml) and Gradle (build.gradle). Runs the fat jar; detects Spring Boot port.' },
    { tech: 'dotnet', label: '.NET',       note: '*.csproj / *.sln detection. Publishes self-contained; detects ASP.NET port from launchSettings.json.' },
    { tech: 'php',    label: 'PHP',        note: 'Composer detection. Runs php artisan serve (Laravel) or php -S for plain projects.' },
    { tech: 'react',  label: 'React / Vite / Next', note: 'Framework chip shown in dashboard. Dev mode enables HMR proxying.' },
    { tech: 'svelte', label: 'Svelte / SvelteKit', note: 'SvelteKit in SSR mode uses adapter-node; static builds served with a fast file server.' },
  ];

  const SERVICES = [
    { icon: 'services', name: 'Postgres',  desc: 'Runs a portable postgres binary. Connection string auto-injected as DATABASE_URL. Browse with pgAdmin or any Postgres client.' },
    { icon: 'services', name: 'Redis',     desc: 'Portable redis-server. Auto-injected as REDIS_URL. Inspect keys live in the Services tab.' },
    { icon: 'services', name: 'MongoDB',   desc: 'Portable mongod. Injected as MONGO_URL. The Services tab shows DB stats and collection counts.' },
    { icon: 'services', name: 'MinIO',     desc: 'S3-compatible object storage. Injected as S3_ENDPOINT + S3_ACCESS_KEY + S3_SECRET_KEY. Console at :9001.' },
  ];

  const DEPLOY_TARGETS = [
    'Railway', 'Google Cloud Run', 'AWS App Runner', 'Azure Container Apps',
    'DigitalOcean App Platform', 'Render', 'Fly.io', 'Vercel', 'Netlify',
    'Hetzner VPS (registry-free)', 'Generic Docker registry',
  ];

  const FEATURES = [
    { icon: 'branch',   title: 'Branch previews',       body: 'Preview any git branch in an isolated worktree — your working tree stays untouched. Each preview is a separate deployment tagged with branch + commit hash. Open from the project page.' },
    { icon: 'logs',     title: 'AI log diagnosis',       body: 'After a failed run, click Diagnose to get an AI explanation of the root cause and a fix suggestion. Add an API key under Settings → AI (supports OpenAI, Anthropic, and any OpenAI-compatible endpoint).' },
    { icon: 'images',   title: 'Content-addressed blobs', body: 'Images are stored as deduplicated blobs (sha256). Pulling the same layer twice costs zero extra disk. crush images shows real sizes; crush prune --all reclaims space.' },
    { icon: 'services', title: 'Crushfile',              body: 'Drop a Crushfile in your project root to override detection: set the runtime, port, start command, native services, and env vars. Compose-compatible syntax is also accepted.' },
    { icon: 'rocket',   title: 'Monorepo support',       body: 'Crush detects multi-service projects via Crushfile or compose.yml. Each service gets its own build, network alias, and log stream. Run all services with one crush run.' },
    { icon: 'box',      title: 'Security scanning',      body: 'crush scan <image> checks packages against a CVE database. Output lists severity, affected package, and fixed version. Runs locally — no data leaves your machine.' },
  ];
</script>

<div class="page">
  <header class="page-header">
    <h1>Documentation</h1>
    <p class="subtitle">Everything you need to run, deploy and manage projects with Crush.</p>
  </header>

  <!-- Quickstart -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="rocket" size={15} /><h2>Quickstart</h2></div>
    <ol class="steps">
      <li>
        <span class="step-n">1</span> Point Crush at any project folder — it auto-detects the stack:
        <div class="cmd"><code>crush run ./my-app</code><button class="copy" onclick={() => copy('crush run ./my-app', 'q1')}>{copied === 'q1' ? 'Copied' : 'Copy'}</button></div>
      </li>
      <li><span class="step-n">2</span> Crush installs deps, builds an image, and starts the dev server. The dashboard shows a live preview with the port, status, and logs.</li>
      <li>
        <span class="step-n">3</span> Ship when ready — Crush picks the right cloud target for your stack:
        <div class="cmd"><code>crush deploy</code><button class="copy" onclick={() => copy('crush deploy', 'q2')}>{copied === 'q2' ? 'Copied' : 'Copy'}</button></div>
      </li>
      <li>
        <span class="step-n">4</span> Add native services (Postgres, Redis, etc.) without Docker:
        <div class="cmd"><code>crush services start postgres</code><button class="copy" onclick={() => copy('crush services start postgres', 'q3')}>{copied === 'q3' ? 'Copied' : 'Copy'}</button></div>
      </li>
    </ol>
    <p class="note">No Docker Desktop. No WSL2. Native services, a content-addressed image store, branch previews, and AI log diagnosis — all built in.</p>
  </div>

  <!-- Detected stacks -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="box" size={15} /><h2>Auto-detected stacks</h2></div>
    <p class="sec-desc">Crush reads your project files and picks the right runtime, package manager, start command, and port — no config needed.</p>
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
    <p class="note">Override anything with a <code class="inline-code">Crushfile</code> in the project root. See the Feature guides section below.</p>
  </div>

  <!-- Command reference -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="logs" size={15} /><h2>Command reference</h2></div>
    <div class="cmds">
      {#each COMMANDS as c}
        <div class="cmd-row">
          <code class="cmd-name">{c.cmd}</code>
          <span class="cmd-desc">{c.desc}</span>
          <button class="copy sm" onclick={() => copy(c.cmd, c.cmd)}>{copied === c.cmd ? '✓' : 'Copy'}</button>
        </div>
      {/each}
    </div>
  </div>

  <!-- Native services -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="services" size={15} /><h2>Native services</h2></div>
    <p class="sec-desc">Crush runs portable database and storage binaries directly on your machine — no Docker, no VM. Connection strings are injected automatically as env vars.</p>
    <div class="features">
      {#each SERVICES as s}
        <div class="feat">
          <div class="feat-ico"><Icon name={s.icon} size={16} /></div>
          <div class="feat-body">
            <h3>{s.name}</h3>
            <p>{s.desc}</p>
          </div>
        </div>
      {/each}
    </div>
    <div class="cmd-block">
      <div class="cmd"><code>crush services start postgres</code><button class="copy" onclick={() => copy('crush services start postgres', 'svc1')}>{copied === 'svc1' ? 'Copied' : 'Copy'}</button></div>
      <div class="cmd"><code>crush services stop postgres</code><button class="copy" onclick={() => copy('crush services stop postgres', 'svc2')}>{copied === 'svc2' ? 'Copied' : 'Copy'}</button></div>
      <div class="cmd"><code>crush services ls</code><button class="copy" onclick={() => copy('crush services ls', 'svc3')}>{copied === 'svc3' ? 'Copied' : 'Copy'}</button></div>
    </div>
  </div>

  <!-- Deploy targets -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="rocket" size={15} /><h2>Cloud deploy — 11 targets</h2></div>
    <p class="sec-desc">Crush wraps each provider's official CLI. Stack detection is used to filter targets (e.g., static sites never see backend-only options).</p>
    <div class="target-grid">
      {#each DEPLOY_TARGETS as t}
        <span class="target-chip">{t}</span>
      {/each}
    </div>
    <div class="cmd-block">
      <div class="cmd"><code>crush deploy --target railway</code><button class="copy" onclick={() => copy('crush deploy --target railway', 'd1')}>{copied === 'd1' ? 'Copied' : 'Copy'}</button></div>
      <div class="cmd"><code>crush eject</code><button class="copy" onclick={() => copy('crush eject', 'd2')}>{copied === 'd2' ? 'Copied' : 'Copy'}</button></div>
    </div>
    <p class="note"><code class="inline-code">crush eject</code> writes a Dockerfile + compose.yml so you can leave Crush anytime with zero lock-in.</p>
  </div>

  <!-- Feature guides -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="box" size={15} /><h2>Feature guides</h2></div>
    <div class="features">
      {#each FEATURES as f}
        <div class="feat">
          <div class="feat-ico"><Icon name={f.icon} size={16} /></div>
          <div class="feat-body">
            <h3>{f.title}</h3>
            <p>{f.body}</p>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Crushfile reference -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="services" size={15} /><h2>Crushfile reference</h2></div>
    <p class="sec-desc">Drop a <code class="inline-code">Crushfile</code> in your project root to override any detected value.</p>
    <div class="cmd">
      <code style="white-space: pre; font-size: 12px; line-height: 1.6">{`runtime: node
version: "20"
port: 3000
start: pnpm run start
dev_start: pnpm run dev

services:
  - postgres
  - redis

env:
  NODE_ENV: production`}</code>
    </div>
    <p class="note">Compose files (<code class="inline-code">docker-compose.yml</code> / <code class="inline-code">compose.yml</code>) are also read automatically — services, volumes, and port mappings are preserved.</p>
  </div>

  <!-- Keyboard shortcuts -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="rocket" size={15} /><h2>Keyboard shortcuts (GUI)</h2></div>
    <div class="shortcuts">
      <div class="sc-row"><kbd>R</kbd><span>Run / restart current project</span></div>
      <div class="sc-row"><kbd>S</kbd><span>Stop the running project</span></div>
      <div class="sc-row"><kbd>Ctrl+K</kbd><span>Open project picker</span></div>
      <div class="sc-row"><kbd>Esc</kbd><span>Close detail pane / terminal</span></div>
      <div class="sc-row"><kbd>1–7</kbd><span>Navigate to Overview, Containers, Images, Services, Logs, Deployments, Settings</span></div>
    </div>
  </div>

  <!-- Resources -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="github" size={15} /><h2>Resources</h2></div>
    <div class="res">
      <button class="res-btn" onclick={() => open(SITE)}><Icon name="github" size={14} /> GitHub repository</button>
      <button class="res-btn" onclick={() => open(`${SITE}#readme`)}><Icon name="logs" size={14} /> Full README</button>
      <button class="res-btn" onclick={() => open(`${SITE}/releases`)}><Icon name="box" size={14} /> Releases &amp; changelog</button>
      <button class="res-btn" onclick={() => open(`${SITE}/issues`)}><Icon name="logs" size={14} /> Report an issue</button>
    </div>
  </div>
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; max-width: 960px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .subtitle { font-size: 13px; color: var(--color-crush-text-muted); margin: 4px 0 0; }

  .sec { padding: 16px 18px; }
  .sec-head { display: flex; align-items: center; gap: 9px; margin-bottom: 10px; color: var(--color-crush-text-muted); }
  .sec-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; margin: 0; }
  .sec-desc { font-size: 13px; color: var(--color-crush-text-muted); margin: 0 0 14px; line-height: 1.5; }

  .steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
  .steps li { font-size: 13.5px; line-height: 1.55; color: var(--color-crush-text); }
  .step-n { display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 50%; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); font-size: 11px; font-family: var(--font-mono); margin-right: 8px; color: var(--color-crush-text-muted); }
  .note { font-size: 12px; color: var(--color-crush-text-muted); margin: 14px 0 0; line-height: 1.5; }

  .cmd { display: flex; align-items: flex-start; gap: 8px; margin: 8px 0 0; background: rgba(9,9,11,0.6); border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 8px 10px; }
  .cmd code { flex: 1; font-family: var(--font-mono); font-size: 12.5px; color: var(--color-crush-text); }
  .copy { background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 6px; padding: 3px 10px; font-size: 11px; cursor: pointer; flex-shrink: 0; }
  .copy:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .copy.sm { padding: 2px 8px; }
  .cmd-block { display: flex; flex-direction: column; gap: 4px; margin-top: 14px; }
  .inline-code { font-family: var(--font-mono); font-size: 11.5px; background: rgba(255,255,255,0.07); border: 1px solid var(--color-crush-border); border-radius: 4px; padding: 1px 5px; }

  .cmds { display: flex; flex-direction: column; }
  .cmd-row { display: grid; grid-template-columns: 210px 1fr auto; align-items: center; gap: 12px; padding: 9px 0; border-bottom: 1px solid rgba(42,42,53,0.4); }
  .cmd-row:last-child { border-bottom: none; }
  .cmd-name { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text); }
  .cmd-desc { font-size: 12.5px; color: var(--color-crush-text-muted); line-height: 1.45; }

  .stack-grid { display: flex; flex-direction: column; gap: 10px; }
  .stack-row { display: flex; align-items: flex-start; gap: 12px; padding: 8px 0; border-bottom: 1px solid rgba(42,42,53,0.3); }
  .stack-row:last-child { border-bottom: none; }
  .stack-row :global(svg) { flex-shrink: 0; margin-top: 2px; }
  .stack-name { font-size: 13px; font-weight: 500; display: block; margin-bottom: 2px; }
  .stack-note { font-size: 12px; color: var(--color-crush-text-muted); display: block; line-height: 1.45; }

  .features { display: flex; flex-direction: column; gap: 14px; }
  .feat { display: flex; gap: 12px; }
  .feat-ico { width: 34px; height: 34px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; border-radius: 8px; background: rgba(255,255,255,0.06); color: var(--color-crush-text); }
  .feat-body h3 { font-size: 13.5px; font-weight: 600; margin: 0 0 4px; }
  .feat-body p { font-size: 12.5px; color: var(--color-crush-text-muted); margin: 0; line-height: 1.55; }

  .target-grid { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 14px; }
  .target-chip { font-size: 12px; padding: 3px 10px; border-radius: 9999px; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); background: rgba(255,255,255,0.02); }

  .shortcuts { display: flex; flex-direction: column; gap: 6px; }
  .sc-row { display: flex; align-items: center; gap: 12px; font-size: 13px; padding: 6px 0; border-bottom: 1px solid rgba(42,42,53,0.3); }
  .sc-row:last-child { border-bottom: none; }
  kbd { font-family: var(--font-mono); font-size: 11px; padding: 2px 7px; border: 1px solid var(--color-crush-border); border-radius: 5px; background: var(--color-crush-surface); color: var(--color-crush-text); white-space: nowrap; }

  .res { display: flex; flex-wrap: wrap; gap: 8px; }
  .res-btn { display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 8px 13px; cursor: pointer; }
  .res-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
</style>
