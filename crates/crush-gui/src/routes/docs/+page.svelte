<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';

  const SITE = 'https://github.com/Chidi09/crush';

  let copied = $state<string | null>(null);
  async function copy(text: string, id: string) {
    try { await navigator.clipboard.writeText(text); copied = id; setTimeout(() => { if (copied === id) copied = null; }, 1400); } catch {}
  }
  function open(url: string) { api.openUrl(url).catch(() => {}); }

  // Command reference — what the CLI does and the GUI equivalent.
  const COMMANDS = [
    { cmd: 'crush run', desc: 'Detect the stack and run the project (build + start, with HMR). Mirrors the dashboard Run button.' },
    { cmd: 'crush deploy', desc: 'Deploy to a cloud provider (Railway, Cloud Run, Fly, Render, DO, Azure, Vercel, Netlify, Hetzner…).' },
    { cmd: 'crush eject', desc: 'Generate a Dockerfile + compose from the detected stack — own your build, no lock-in.' },
    { cmd: 'crush ps', desc: 'List running containers and their bound ports.' },
    { cmd: 'crush logs <name>', desc: 'Stream build + runtime logs for a service (the Logs page renders these live).' },
    { cmd: 'crush services', desc: 'Manage native Postgres / Redis / MongoDB / MinIO without Docker.' },
    { cmd: 'crush images', desc: 'List, pull, and inspect OCI images in the content-addressed store.' },
  ];

  const FEATURES = [
    { icon: 'services', title: 'Native services — no Docker needed', body: 'Crush downloads portable binaries on first use and runs Postgres, Redis, MongoDB and MinIO (local S3 buckets) directly. Declare them in your compose/Crushfile and they boot natively; connection strings show up in the Services page.', link: 'docs/services' },
    { icon: 'rocket', title: 'Cloud deploy across 11 providers', body: 'Eject to a Dockerfile or deploy straight to Railway, Cloud Run, App Runner, Azure Container Apps, DigitalOcean, Render, Fly.io, Vercel, Netlify, or a registry-free Hetzner VPS. Crush wraps each provider\'s official CLI — recommendations are stack-aware (frontends never see a backend-only target).', link: 'docs/deploy' },
    { icon: 'branch', title: 'Git branch previews', body: 'Preview any local or remote branch in an isolated git worktree — your main checkout and uncommitted files stay untouched. Each preview becomes a deployment tagged with its branch + commit.', link: 'docs/branch-previews' },
    { icon: 'logs', title: 'AI log diagnosis', body: 'When a run fails, Diagnose reads the stderr and explains the root cause + a fix. Add your provider API key under Settings → AI to enable it.', link: 'docs/gui' },
  ];
</script>

<div class="page">
  <header class="page-header">
    <h1>Documentation</h1>
    <p class="subtitle">Everything you need to run, deploy and manage projects with Crush — right here in the app.</p>
  </header>

  <!-- Quickstart -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="rocket" size={15} /><h2>Quickstart</h2></div>
    <ol class="steps">
      <li><span class="step-n">1</span> Point Crush at any project folder (it auto-detects the stack):
        <div class="cmd"><code>crush run ./my-app</code><button class="copy" onclick={() => copy('crush run ./my-app', 'q1')}>{copied === 'q1' ? 'Copied' : 'Copy'}</button></div>
      </li>
      <li><span class="step-n">2</span> Crush installs deps, starts the dev server, and binds a port — open the live preview from the dashboard.</li>
      <li><span class="step-n">3</span> Ship it when ready:
        <div class="cmd"><code>crush deploy</code><button class="copy" onclick={() => copy('crush deploy', 'q2')}>{copied === 'q2' ? 'Copied' : 'Copy'}</button></div>
      </li>
    </ol>
    <p class="note">No Docker Desktop, no WSL2 setup. Native services and a content-addressed image store come built in.</p>
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
            <button class="link" onclick={() => open(`${SITE}#readme`)}>Read the full guide <span class="arr">→</span></button>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Resources -->
  <div class="crush-card sec">
    <div class="sec-head"><Icon name="github" size={15} /><h2>Resources</h2></div>
    <div class="res">
      <button class="res-btn" onclick={() => open(SITE)}><Icon name="github" size={14} /> GitHub repository</button>
      <button class="res-btn" onclick={() => open(`${SITE}#readme`)}><Icon name="logs" size={14} /> Full documentation</button>
      <button class="res-btn" onclick={() => open(`${SITE}/releases`)}><Icon name="box" size={14} /> Downloads &amp; releases</button>
    </div>
  </div>
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; max-width: 960px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .subtitle { font-size: 13px; color: var(--color-crush-text-muted); margin: 4px 0 0; }

  .sec { padding: 16px 18px; }
  .sec-head { display: flex; align-items: center; gap: 9px; margin-bottom: 14px; color: var(--color-crush-text-muted); }
  .sec-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; margin: 0; }

  .steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
  .steps li { font-size: 13.5px; line-height: 1.55; color: var(--color-crush-text); }
  .step-n { display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 50%; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); font-size: 11px; font-family: var(--font-mono); margin-right: 8px; color: var(--color-crush-text-muted); }
  .note { font-size: 12px; color: var(--color-crush-text-muted); margin: 14px 0 0; line-height: 1.5; }

  .cmd { display: flex; align-items: center; gap: 8px; margin: 8px 0 0; background: rgba(9,9,11,0.6); border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 8px 10px; }
  .cmd code { flex: 1; font-family: var(--font-mono); font-size: 12.5px; color: var(--color-crush-text); }
  .copy { background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 6px; padding: 3px 10px; font-size: 11px; cursor: pointer; flex-shrink: 0; }
  .copy:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .copy.sm { padding: 2px 8px; }

  .cmds { display: flex; flex-direction: column; }
  .cmd-row { display: grid; grid-template-columns: 170px 1fr auto; align-items: center; gap: 12px; padding: 9px 0; border-bottom: 1px solid rgba(42,42,53,0.4); }
  .cmd-row:last-child { border-bottom: none; }
  .cmd-name { font-family: var(--font-mono); font-size: 12.5px; color: var(--color-crush-text); }
  .cmd-desc { font-size: 12.5px; color: var(--color-crush-text-muted); line-height: 1.45; }

  .features { display: flex; flex-direction: column; gap: 16px; }
  .feat { display: flex; gap: 12px; }
  .feat-ico { width: 34px; height: 34px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; border-radius: 8px; background: rgba(255,255,255,0.06); color: var(--color-crush-text); }
  .feat-body h3 { font-size: 14px; font-weight: 600; margin: 0 0 4px; }
  .feat-body p { font-size: 12.5px; color: var(--color-crush-text-muted); margin: 0 0 6px; line-height: 1.55; }
  .link { background: none; border: none; color: var(--color-crush-text); font-size: 12px; cursor: pointer; padding: 0; display: inline-flex; align-items: center; gap: 4px; }
  .link:hover { text-decoration: underline; }
  .link .arr { transition: transform 0.15s; }
  .link:hover .arr { transform: translateX(2px); }

  .res { display: flex; flex-wrap: wrap; gap: 8px; }
  .res-btn { display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 8px 13px; cursor: pointer; }
  .res-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
</style>
