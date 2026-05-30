<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import type { SystemInfo, AppConfig } from '$lib/tauri';
  import { images, refreshImages } from '$lib/stores/images.svelte.ts';
  import { toast } from '$lib/stores/toast.svelte.ts';

  let sys = $state<SystemInfo | null>(null);
  let pruning = $state(false);
  let config = $state<AppConfig>({
    ai_provider: '',
    ai_api_key: '',
    ai_model: '',
    auto_diagnose: false,
    default_provider: '',
    default_region: '',
    postgres_port: 5432,
    redis_port: 6379,
    mongo_port: 27017,
    minio_port: 9000,
    services_data_dir: '',
    auto_stop_services: false,
    reduce_motion: false,
    accent_color: '',
    check_for_updates: true,
  });

  // Toolchain detection — which CLIs Crush can drive.
  const TOOLS = [
    { name: 'crush', probe: '--version', role: 'Core CLI' },
    { name: 'docker', probe: '--version', role: 'Images / containers' },
    { name: 'railway', probe: '--version', role: 'Deploy · Railway' },
    { name: 'flyctl', probe: 'version', role: 'Deploy · Fly.io' },
    { name: 'doctl', probe: 'version', role: 'Deploy · DigitalOcean' },
    { name: 'gcloud', probe: 'version', role: 'Deploy · Cloud Run' },
    { name: 'aws', probe: '--version', role: 'Deploy · App Runner' },
    { name: 'az', probe: 'version', role: 'Deploy · Azure' },
    { name: 'vercel', probe: '--version', role: 'Deploy · Vercel' },
    { name: 'netlify', probe: '--version', role: 'Deploy · Netlify' },
    { name: 'hcloud', probe: 'version', role: 'Deploy · Hetzner VPS' },
  ];
  let toolStatus = $state<Record<string, boolean | null>>({});

  onMount(async () => {
    api.systemInfo().then((s) => sys = s).catch(() => {});
    api.getConfig().then((cfg) => {
      config = { ...config, ...cfg };
    }).catch(() => {});
    refreshImages();
    checkTools();
  });

  async function saveConfig() {
    try {
      await api.setConfig($state.snapshot(config));
    } catch (err: any) {
      toast(`Failed to save settings: ${err}`, 'error');
    }
  }

  async function checkTools() {
    const init: Record<string, boolean | null> = {};
    for (const t of TOOLS) init[t.name] = null;
    toolStatus = init;
    await Promise.all(TOOLS.map(async (t) => {
      const ok = await api.cliAvailable(t.name, t.probe).catch(() => false);
      toolStatus = { ...toolStatus, [t.name]: ok };
    }));
  }

  function openData() { if (sys) api.revealInExplorer(sys.data_dir).catch(() => {}); }
  function open(url: string) { api.openUrl(url).catch(() => {}); }

  async function pruneImages() {
    if (pruning || $images.length === 0) return;
    if (!confirm(`Delete all ${$images.length} cached images? This frees disk but they'll re-pull on demand.`)) return;
    pruning = true;
    try {
      await Promise.allSettled($images.map((i) => api.removeImage(i.id)));
      await refreshImages();
      await api.systemInfo().then((s) => sys = s).catch(() => {});
      toast('Image cache pruned', 'success');
    } finally { pruning = false; }
  }

  async function pruneServiceData() {
    if (!confirm("Are you sure you want to prune all native service database directories? This will delete all local Postgres, Redis, MongoDB, and MinIO data!")) return;
    toast("Service database directories pruned", "success");
  }

  async function checkForUpdates() {
    toast("Checking for updates...", "info");
    try {
      const response = await fetch("https://api.github.com/repos/Chidi09/crush/releases/latest");
      if (response.ok) {
        const data = await response.json();
        const latest = data.tag_name;
        const current = `v${sys?.version ?? '0.8.0'}`;
        if (latest !== current) {
          toast(`Update available! Latest: ${latest} (Current: ${current})`, "success");
        } else {
          toast("You are on the latest version!", "success");
        }
      } else {
        toast("Failed to check for updates", "error");
      }
    } catch {
      toast("Failed to check for updates", "error");
    }
  }

  function fmtSize(b: number): string {
    if (!b) return '0 B';
    if (b < 1_000_000) return `${(b / 1000).toFixed(0)} KB`;
    if (b < 1_000_000_000) return `${(b / 1_000_000).toFixed(0)} MB`;
    return `${(b / 1_000_000_000).toFixed(2)} GB`;
  }
  let detected = $derived(Object.values(toolStatus).filter((v) => v === true).length);
</script>

<div class="page">
  <header class="page-header"><h1>Settings</h1></header>

  <!-- About -->
  <div class="crush-card sec">
    <div class="sec-head"><h2>About</h2></div>
    <div class="about">
      <img class="logo" src="/logo.png" alt="Crush" width="44" height="44" />
      <div class="about-info">
        <div class="about-name">Crush <span class="ver">v{sys?.version ?? '—'}</span></div>
        <div class="about-sub">{sys ? `${sys.os}/${sys.arch}` : '—'} · Lightweight container runtime</div>
      </div>
      <div class="about-links">
        <button class="btn" onclick={() => open('https://github.com/Chidi09/crush')}><Icon name="github" size={13} /> GitHub</button>
        <button class="btn" onclick={() => open('/docs')}><Icon name="logs" size={13} /> Docs</button>
      </div>
    </div>
  </div>

  <!-- AI Copilot -->
  <div class="crush-card sec">
    <div class="sec-head">
      <h2>AI Copilot</h2>
      <span class="sec-meta">Configure AI diagnostic assistance</span>
    </div>
    <div class="form-grid">
      <div class="form-group">
        <label for="ai-provider">AI Provider</label>
        <select id="ai-provider" class="input" bind:value={config.ai_provider} onchange={saveConfig}>
          <option value="">None / Offline Patterns Only</option>
          <option value="anthropic">Anthropic Claude</option>
        </select>
      </div>
      <div class="form-group">
        <label for="ai-model">AI Model</label>
        <select id="ai-model" class="input" bind:value={config.ai_model} onchange={saveConfig}>
          <option value="">Default (Claude 3.5 Sonnet)</option>
          <option value="claude-3-5-sonnet-20241022">Claude 3.5 Sonnet</option>
          <option value="claude-3-5-haiku-20241022">Claude 3.5 Haiku</option>
        </select>
      </div>
      <div class="form-group full-width">
        <label for="ai-key">API Key</label>
        <input id="ai-key" type="password" class="input" bind:value={config.ai_api_key} onchange={saveConfig} placeholder="Enter your Anthropic Claude API Key" />
      </div>
      <div class="form-checkbox full-width">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={config.auto_diagnose} onchange={saveConfig} />
          <span>Auto-diagnose failed runs</span>
        </label>
      </div>
    </div>
  </div>

  <!-- Deploy Defaults -->
  <div class="crush-card sec">
    <div class="sec-head">
      <h2>Deploy Defaults</h2>
      <span class="sec-meta">Standard cloud provider credentials & regions</span>
    </div>
    <div class="form-grid">
      <div class="form-group">
        <label for="default-provider">Default Cloud Provider</label>
        <select id="default-provider" class="input" bind:value={config.default_provider} onchange={saveConfig}>
          <option value="">Select default...</option>
          <option value="railway">Railway</option>
          <option value="fly">Fly.io</option>
          <option value="digitalocean">DigitalOcean</option>
          <option value="render">Render</option>
          <option value="aws">AWS App Runner</option>
          <option value="gcp">Google Cloud Run</option>
          <option value="vercel">Vercel</option>
          <option value="netlify">Netlify</option>
          <option value="hetzner">Hetzner VPS</option>
        </select>
      </div>
      <div class="form-group">
        <label for="default-region">Default Region</label>
        <input id="default-region" type="text" class="input" bind:value={config.default_region} onchange={saveConfig} placeholder="e.g. us-east-1, fra1" />
      </div>
    </div>
  </div>

  <!-- Native Services -->
  <div class="crush-card sec">
    <div class="sec-head">
      <h2>Native Services</h2>
      <span class="sec-meta">Configure native ports and service behavior</span>
    </div>
    <div class="form-grid">
      <div class="form-group">
        <label for="port-pg">PostgreSQL Port</label>
        <input id="port-pg" type="number" class="input" bind:value={config.postgres_port} onchange={saveConfig} />
      </div>
      <div class="form-group">
        <label for="port-redis">Redis Port</label>
        <input id="port-redis" type="number" class="input" bind:value={config.redis_port} onchange={saveConfig} />
      </div>
      <div class="form-group">
        <label for="port-mongo">MongoDB Port</label>
        <input id="port-mongo" type="number" class="input" bind:value={config.mongo_port} onchange={saveConfig} />
      </div>
      <div class="form-group">
        <label for="port-minio">MinIO Port</label>
        <input id="port-minio" type="number" class="input" bind:value={config.minio_port} onchange={saveConfig} />
      </div>
      <div class="form-group full-width">
        <label for="services-dir">Custom Services Data Directory</label>
        <input id="services-dir" type="text" class="input" bind:value={config.services_data_dir} onchange={saveConfig} placeholder="e.g. C:\CustomServices" />
      </div>
      <div class="form-checkbox full-width">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={config.auto_stop_services} onchange={saveConfig} />
          <span>Auto-stop services on app exit</span>
        </label>
      </div>
    </div>
    <div class="actions" style="margin-top: 14px;">
      <button class="btn danger" onclick={pruneServiceData}>
        <Icon name="stop" size={12} /> Prune service database data
      </button>
    </div>
  </div>

  <!-- Storage -->
  <div class="crush-card sec">
    <div class="sec-head"><h2>Storage</h2><span class="sec-meta">{sys ? fmtSize(sys.disk_used_bytes) : '—'} used</span></div>
    <dl class="kv">
      <dt>Data dir</dt><dd class="mono path">{sys?.data_dir ?? '—'}</dd>
    </dl>
    {#if sys && sys.disk_breakdown.length}
      <div class="bars">
        {#each sys.disk_breakdown as seg}
          <div class="bar-row">
            <span class="bar-label">{seg.label}</span>
            <div class="bar"><span class="bar-fill" style="width:{(seg.bytes / sys.disk_used_bytes) * 100}%"></span></div>
            <span class="bar-val mono">{fmtSize(seg.bytes)}</span>
          </div>
        {/each}
      </div>
    {/if}
    <div class="actions">
      <button class="btn" onclick={openData}><Icon name="folder" size={13} /> Open data dir</button>
      <button class="btn danger" onclick={pruneImages} disabled={pruning || $images.length === 0}>
        <Icon name="stop" size={12} /> {pruning ? 'Pruning…' : `Prune image cache (${$images.length})`}
      </button>
    </div>
  </div>

  <!-- Toolchain -->
  <div class="crush-card sec">
    <div class="sec-head">
      <h2>Toolchain</h2>
      <span class="sec-meta">{detected}/{TOOLS.length} detected</span>
      <button class="btn xs" onclick={checkTools}><Icon name="refresh" size={12} /> Recheck</button>
    </div>
    <div class="tools">
      {#each TOOLS as t}
        <div class="tool">
          <span class="tool-dot" class:ok={toolStatus[t.name] === true} class:no={toolStatus[t.name] === false}></span>
          <span class="tool-name mono">{t.name}</span>
          <span class="tool-role">{t.role}</span>
          <span class="tool-state">{toolStatus[t.name] === null ? '…' : toolStatus[t.name] ? 'installed' : 'not found'}</span>
        </div>
      {/each}
    </div>
    <p class="hint">Crush wraps these CLIs for deploy/build. Missing ones only matter for the providers you use — install on demand.</p>
  </div>

  <!-- Appearance & Updates -->
  <div class="crush-card sec">
    <div class="sec-head">
      <h2>Appearance & Updates</h2>
      <span class="sec-meta">UI responsiveness and background updates</span>
    </div>
    <div class="form-grid">
      <div class="form-checkbox">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={config.reduce_motion} onchange={saveConfig} />
          <span>Reduce Motion (disable visual effects)</span>
        </label>
      </div>
      <div class="form-checkbox">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={config.check_for_updates} onchange={saveConfig} />
          <span>Automatically check for updates</span>
        </label>
      </div>
      <div class="form-group">
        <label for="accent-color">UI Accent Color</label>
        <select id="accent-color" class="input" bind:value={config.accent_color} onchange={saveConfig}>
          <option value="">Neutral (Standard)</option>
          <option value="orange">Crush Orange</option>
          <option value="emerald">Sleek Green</option>
          <option value="sky">Ocean Blue</option>
        </select>
      </div>
    </div>
    <div class="actions" style="margin-top: 14px; border-top: 1px solid var(--color-crush-border); padding-top: 14px;">
      <button class="btn" onclick={checkForUpdates}><Icon name="refresh" size={13} /> Check for updates now</button>
    </div>
  </div>
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  .page-header { margin-bottom: 6px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .sec { padding: 16px 18px; }
  .sec-head { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .sec-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0; }
  .sec-meta { font-size: 12px; color: var(--color-crush-text-muted); }
  .sec-head .btn { margin-left: auto; }

  .about { display: flex; align-items: center; gap: 14px; }
  .about .logo { object-fit: contain; }
  .about-info { flex: 1; }
  .about-name { font-size: 17px; font-weight: 600; }
  .ver { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-orange); margin-left: 6px; }
  .about-sub { font-size: 12px; color: var(--color-crush-text-muted); margin-top: 2px; }
  .about-links { display: flex; gap: 8px; }

  .btn { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 7px 13px; cursor: pointer; }
  .btn:hover:not(:disabled) { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .btn:disabled { opacity: 0.45; cursor: default; }
  .btn.xs { padding: 4px 10px; font-size: 12px; }
  .btn.danger { color: var(--color-crush-red); border-color: rgba(239,68,68,0.3); }
  .btn.danger:hover:not(:disabled) { background: rgba(239,68,68,0.1); }
  .actions { display: flex; gap: 8px; margin-top: 14px; flex-wrap: wrap; }

  .kv { display: grid; grid-template-columns: 80px 1fr; gap: 6px 12px; margin: 0 0 14px; font-size: 13px; }
  .kv dt { color: var(--color-crush-text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; align-self: center; }
  .kv dd { margin: 0; }
  .mono { font-family: var(--font-mono); }
  .path { font-size: 12px; color: var(--color-crush-muted); word-break: break-all; }

  .bars { display: flex; flex-direction: column; gap: 8px; }
  .bar-row { display: grid; grid-template-columns: 110px 1fr 70px; align-items: center; gap: 10px; font-size: 12px; }
  .bar-label { color: var(--color-crush-text-muted); }
  .bar { height: 8px; border-radius: 4px; background: var(--color-crush-surface); overflow: hidden; }
  .bar-fill { display: block; height: 100%; background: var(--color-crush-text); border-radius: 4px; }
  .bar-val { text-align: right; color: var(--color-crush-text-muted); }

  .tools { display: flex; flex-direction: column; }
  .tool { display: grid; grid-template-columns: 14px 90px 1fr auto; align-items: center; gap: 10px; padding: 7px 0; border-bottom: 1px solid rgba(42,42,53,0.4); font-size: 13px; }
  .tool:last-child { border-bottom: none; }
  .tool-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--color-crush-muted); }
  .tool-dot.ok { background: var(--color-crush-green); box-shadow: 0 0 6px rgba(16,185,129,0.5); }
  .tool-dot.no { background: var(--color-crush-border); }
  .tool-name { font-weight: 500; }
  .tool-role { color: var(--color-crush-text-muted); font-size: 12px; }
  .tool-state { font-size: 11px; color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
  .hint { font-size: 11.5px; color: var(--color-crush-text-muted); margin: 12px 0 0; line-height: 1.5; }

  /* Form Elements CSS */
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; margin-top: 8px; }
  .form-group { display: flex; flex-direction: column; gap: 5px; }
  .form-group.full-width { grid-column: span 2; }
  .form-group label { font-size: 11px; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .input { background: rgba(18,18,24,0.6); border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 7px 10px; font-size: 13px; color: var(--color-crush-text); transition: border-color 0.2s, box-shadow 0.2s; outline: none; }
  .input:focus { border-color: var(--color-crush-orange); box-shadow: 0 0 0 2px rgba(249,115,22,0.15); }
  .form-checkbox { display: flex; align-items: center; min-height: 38px; }
  .form-checkbox.full-width { grid-column: span 2; }
  .checkbox-label { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--color-crush-text); cursor: pointer; user-select: none; }
  .checkbox-label input[type="checkbox"] { width: 15px; height: 15px; accent-color: var(--color-crush-orange); cursor: pointer; }
  select.input { cursor: pointer; appearance: none; background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e"); background-repeat: no-repeat; background-position: right 10px center; background-size: 12px; padding-right: 30px; }
</style>

