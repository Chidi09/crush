<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import TerminalPane from '$lib/components/TerminalPane.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Sparkline from '$lib/components/Sparkline.svelte';
  import * as api from '$lib/tauri';
  import type { ProjectInfo, SystemInfo, BuildSummary } from '$lib/tauri';
  import { containers, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import { services, refreshServices } from '$lib/stores/services.svelte.ts';
  import { images, refreshImages } from '$lib/stores/images.svelte.ts';

  const LAST_PROJECT = 'crush:lastProject';

  let projectPath = $state<string | null>(null);
  let project = $state<ProjectInfo | null>(null);
  let detecting = $state(false);
  let sys = $state<SystemInfo | null>(null);
  let builds = $state<BuildSummary[]>([]);
  let activeRunId = $state<string | null>(null);
  let stoppingAll = $state(false);
  let refreshing = $state(false);

  let runningCount = $derived($containers.filter(c => c.status === 'running').length);
  let pollId: ReturnType<typeof setInterval> | null = null;

  // rolling histories powering the stat-card sparklines (sampled live)
  let hC = $state<number[]>([]);
  let hI = $state<number[]>([]);
  let hS = $state<number[]>([]);
  let hD = $state<number[]>([]);

  onMount(async () => {
    startPolling();
    const saved = localStorage.getItem(LAST_PROJECT);
    if (saved) { projectPath = saved; detectStack(saved); }
    await loadAll();
    sample();
    pollId = setInterval(async () => { await refreshLight(); sample(); }, 5000);
  });
  onDestroy(() => { stopPolling(); if (pollId) clearInterval(pollId); });

  function sample() {
    hC = [...hC.slice(-23), runningCount];
    hI = [...hI.slice(-23), $images.length];
    hS = [...hS.slice(-23), $services.length];
    hD = [...hD.slice(-23), sys?.disk_used_bytes ?? 0];
  }

  async function refreshLight() {
    await Promise.allSettled([
      refreshServices(),
      refreshImages(),
      api.listBuildHistory(20).then(b => builds = b.slice().reverse()).catch(() => {}),
    ]);
  }
  async function loadAll() {
    refreshing = true;
    await Promise.allSettled([refreshLight(), api.systemInfo().then(s => sys = s).catch(() => {})]);
    refreshing = false;
  }

  async function detectStack(path: string) {
    detecting = true; project = null;
    try { project = await api.detectProject(path); } catch (e) { console.error(e); } finally { detecting = false; }
  }
  async function openProject() {
    const p = await api.pickProjectDirectory();
    if (p) { projectPath = p; localStorage.setItem(LAST_PROJECT, p); detectStack(p); }
  }
  async function runProject() {
    if (!projectPath) { await openProject(); return; }
    try { activeRunId = await api.runProject(projectPath); } catch (e) { console.error(e); }
  }
  async function stopAllServices() {
    stoppingAll = true;
    try { await Promise.allSettled($services.map(s => api.stopNativeService(s.name, s.project))); await refreshServices(); }
    finally { stoppingAll = false; }
  }
  function revealData() { if (sys) api.revealInExplorer(sys.data_dir).catch(() => {}); }
  function closeTerminal() { activeRunId = null; }

  function fmtSize(b: number): string {
    if (!b) return '0 B';
    if (b < 1_000_000) return `${(b / 1000).toFixed(0)} KB`;
    if (b < 1_000_000_000) return `${(b / 1_000_000).toFixed(0)} MB`;
    return `${(b / 1_000_000_000).toFixed(2)} GB`;
  }
  function timeAgo(ms: number): string {
    const s = Math.floor((Date.now() - ms) / 1000);
    if (s < 60) return `${s}s ago`;
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    return `${Math.floor(s / 86400)}d ago`;
  }
  function fmtMs(ms: number): string { return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`; }
  function fmtUptime(secs: number): string {
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  }
  function baseName(p: string): string { return p.split(/[\\/]/).filter(Boolean).pop() ?? p; }
</script>

<div class="dashboard">
  <header class="hero">
    <div class="hero-left">
      <svg class="logo" width="22" height="22" viewBox="0 0 32 32" aria-hidden="true"><path d="M16 3 L28 16 L16 29 L4 16 Z" fill="var(--color-crush-orange)"/></svg>
      <span class="wordmark">crush</span>
      {#if sys}<span class="ver-badge">v{sys.version}</span><span class="os-badge">{sys.os}/{sys.arch}</span>{/if}
    </div>
    <div class="hero-right">
      <span class="running-ind"><span class="rdot" class:live={runningCount > 0}></span>{runningCount} running</span>
      <button class="ghost-btn" onclick={loadAll} title="Refresh" class:spinning={refreshing}><Icon name="refresh" size={14} /></button>
      <button class="run-btn" onclick={runProject}>crush <Icon name="play" size={12} fill /></button>
    </div>
  </header>

  {#if activeRunId}
    <div class="animate-slide-up"><TerminalPane runId={activeRunId} onClose={closeTerminal} /></div>
  {/if}

  <!-- Current project -->
  <div class="crush-card project-card">
    {#if projectPath}
      <div class="proj-top">
        <div class="proj-id">
          <div class="proj-label">Current project</div>
          <div class="proj-name">{project?.name ?? baseName(projectPath)}</div>
          <div class="proj-path">{projectPath}</div>
        </div>
        <div class="proj-actions">
          <button class="ghost-btn sm" onclick={openProject}>Change</button>
          <button class="btn-primary" onclick={runProject}>Run <Icon name="play" size={13} fill /></button>
        </div>
      </div>
      {#if detecting}
        <div class="chips"><span class="chip skel">detecting stack…</span></div>
      {:else if project}
        <div class="chips">
          <span class="chip accent">{project.runtime}{project.version ? ` ${project.version}` : ''}</span>
          {#if project.framework}<span class="chip">{project.framework}</span>{/if}
          <span class="chip">:{project.port}</span>
          {#if project.is_monorepo}<span class="chip">monorepo · {project.service_count} svc</span>{/if}
          {#if project.env_required.length}<span class="chip muted-chip">{project.env_required.length} env</span>{/if}
          <span class="chip muted-chip">{Math.round(project.confidence * 100)}% match</span>
        </div>
      {/if}
    {:else}
      <EmptyState title="No project open" description="Open a Crush project to detect its stack and run it" action="Open project…" onAction={openProject} />
    {/if}
  </div>

  <!-- Stat cards with sparklines -->
  <div class="stats">
    <div class="crush-card stat">
      <div class="stat-top"><div class="stat-icon"><Icon name="containers" size={16} /></div><span class="stat-label">Containers</span><span class="stat-value">{runningCount}</span></div>
      <div class="stat-spark"><Sparkline data={hC} color="#e05540" height={38} /></div>
    </div>
    <div class="crush-card stat">
      <div class="stat-top"><div class="stat-icon cyan"><Icon name="images" size={16} /></div><span class="stat-label">Images</span><span class="stat-value">{$images.length}</span></div>
      <div class="stat-spark"><Sparkline data={hI} color="#22d3ee" height={38} /></div>
    </div>
    <div class="crush-card stat">
      <div class="stat-top"><div class="stat-icon green"><Icon name="services" size={16} /></div><span class="stat-label">Services</span><span class="stat-value">{$services.length}</span></div>
      <div class="stat-spark"><Sparkline data={hS} color="#4ade80" height={38} /></div>
    </div>
    <div class="crush-card stat">
      <div class="stat-top"><div class="stat-icon purple"><Icon name="disk" size={16} /></div><span class="stat-label">Disk used</span><span class="stat-value sm">{sys ? fmtSize(sys.disk_used_bytes) : '—'}</span></div>
      <div class="stat-spark"><Sparkline data={hD} color="#c084fc" height={38} /></div>
    </div>
  </div>

  <!-- Main (builds table) + side column -->
  <div class="grid-main">
    <div class="crush-card panel">
      <div class="panel-head"><h2>Recent builds</h2><span class="count">{builds.length}</span></div>
      {#if builds.length}
        <table class="tbl">
          <thead><tr><th>Project</th><th>Stack</th><th class="r">Duration</th><th>Status</th><th class="r">When</th></tr></thead>
          <tbody>
            {#each builds as b}
              <tr>
                <td class="strong">{b.project_name}</td>
                <td class="dim">{b.language}{b.framework && b.framework !== 'none' ? ` · ${b.framework}` : ''}</td>
                <td class="r mono dim">{fmtMs(b.duration_ms)}</td>
                <td><span class="tag" class:cached={b.was_cached} class:fail={!b.success}>{!b.success ? 'failed' : b.was_cached ? 'cached' : 'fresh'}</span></td>
                <td class="r dim sm">{timeAgo(b.timestamp_ms)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">No builds yet — run a project to populate history.</p>
      {/if}
    </div>

    <div class="side">
      <div class="crush-card panel">
        <div class="panel-head"><h2>System</h2></div>
        <dl class="kv">
          <dt>Version</dt><dd class="mono">{sys?.version ?? '—'}</dd>
          <dt>Platform</dt><dd class="mono">{sys ? `${sys.os}/${sys.arch}` : '—'}</dd>
          <dt>Disk</dt><dd class="mono">{sys ? fmtSize(sys.disk_used_bytes) : '—'}</dd>
          <dt>Data dir</dt><dd class="mono path" title={sys?.data_dir}>{sys?.data_dir ?? '—'}</dd>
        </dl>
        {#if sys}<button class="ghost-btn sm full" onclick={revealData}><Icon name="folder" size={13} /> Open data dir</button>{/if}
      </div>

      <div class="crush-card panel">
        <div class="panel-head"><h2>Services</h2>{#if $services.length}<button class="ghost-btn xs" onclick={stopAllServices} disabled={stoppingAll}>{stoppingAll ? '…' : 'Stop all'}</button>{/if}</div>
        {#if $services.length}
          <div class="list">
            {#each $services as svc}
              <div class="srow">
                <StatusBadge status="running" />
                <span class="strong">{svc.name}</span>
                <span class="dim sm">{svc.project}</span>
                <span class="mono port">:{svc.port}</span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="muted">No native services running</p>
        {/if}
      </div>
    </div>
  </div>

  <!-- Containers table -->
  <div class="crush-card panel">
    <div class="panel-head"><h2>Containers</h2><span class="count">{$containers.length}</span></div>
    {#if $containers.length}
      <table class="tbl">
        <thead><tr><th>Name</th><th>Image</th><th>Status</th><th class="r">Ports</th><th class="r">Uptime</th></tr></thead>
        <tbody>
          {#each $containers as c}
            <tr>
              <td class="strong">{c.name}</td>
              <td class="dim mono sm">{c.image}</td>
              <td><StatusBadge status={c.status} /></td>
              <td class="r mono port">{c.ports.length ? c.ports.map(p => `:${p.host_port}`).join(' ') : '—'}</td>
              <td class="r dim sm">{c.uptime_secs ? fmtUptime(c.uptime_secs) : '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="muted">No containers. Run a project to create one.</p>
    {/if}
  </div>
</div>

<style>
  .dashboard { display: flex; flex-direction: column; gap: 14px; padding-bottom: 24px; }

  .hero { display: flex; align-items: center; justify-content: space-between; }
  .hero-left { display: flex; align-items: center; gap: 10px; }
  .wordmark { font-family: var(--font-mono); font-size: 20px; font-weight: 700; letter-spacing: -0.02em; }
  .ver-badge { font-family: var(--font-mono); font-size: 11px; color: var(--color-crush-orange); border: 1px solid rgba(224,85,64,0.25); border-radius: 9999px; padding: 1px 8px; }
  .os-badge { font-family: var(--font-mono); font-size: 11px; color: var(--color-crush-text-muted); }
  .hero-right { display: flex; align-items: center; gap: 12px; }
  .running-ind { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--color-crush-text-muted); }
  .rdot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-crush-muted); }
  .rdot.live { background: var(--color-crush-green); box-shadow: 0 0 6px rgba(74,222,128,0.5); }
  .run-btn { display: inline-flex; align-items: center; gap: 6px; background: var(--color-crush-orange); color: white; border: none; border-radius: 0.75rem; padding: 7px 16px; font-size: 13px; cursor: pointer; font-family: var(--font-mono); transition: background 0.15s; }
  .run-btn:hover { background: var(--color-crush-orange-light); }
  .ghost-btn { display: inline-flex; align-items: center; justify-content: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.75rem; padding: 7px 10px; font-size: 13px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn.sm { padding: 5px 12px; } .ghost-btn.xs { padding: 3px 10px; font-size: 12px; } .ghost-btn.full { width: 100%; margin-top: 10px; }
  .ghost-btn.spinning :global(svg) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .project-card { padding: 20px; }
  .proj-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; margin-bottom: 14px; }
  .proj-id { min-width: 0; }
  .proj-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .proj-name { font-size: 20px; font-weight: 600; margin-top: 2px; }
  .proj-path { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-muted); margin-top: 4px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .proj-actions { display: flex; gap: 8px; flex-shrink: 0; }
  .btn-primary { display: inline-flex; align-items: center; gap: 6px; background: var(--color-crush-orange); color: white; border: none; border-radius: 0.75rem; padding: 7px 18px; font-size: 13px; cursor: pointer; transition: background 0.15s; }
  .btn-primary:hover { background: var(--color-crush-orange-light); }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .chip { font-size: 12px; padding: 3px 10px; border-radius: 9999px; border: 1px solid var(--color-crush-border); color: var(--color-crush-text); background: rgba(255,255,255,0.02); }
  .chip.accent { border-color: rgba(224,85,64,0.3); background: rgba(224,85,64,0.08); color: var(--color-crush-orange); font-weight: 500; }
  .chip.muted-chip, .chip.skel { color: var(--color-crush-text-muted); }

  .stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 14px; }
  .stat { padding: 14px 16px 0; display: flex; flex-direction: column; overflow: hidden; }
  .stat-top { display: flex; align-items: center; gap: 8px; }
  .stat-icon { width: 30px; height: 30px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; border-radius: 8px; background: rgba(224,85,64,0.12); color: var(--color-crush-orange); }
  .stat-icon.cyan { background: rgba(34,211,238,0.12); color: #22d3ee; }
  .stat-icon.green { background: rgba(74,222,128,0.12); color: #4ade80; }
  .stat-icon.purple { background: rgba(192,132,252,0.12); color: #c084fc; }
  .stat-label { font-size: 11px; color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
  .stat-value { margin-left: auto; font-size: 22px; font-weight: 700; line-height: 1; }
  .stat-value.sm { font-size: 17px; }
  .stat-spark { margin: 8px -16px 0; }

  .grid-main { display: grid; grid-template-columns: 2fr 1fr; gap: 14px; align-items: start; }
  .side { display: flex; flex-direction: column; gap: 14px; }
  .panel { padding: 16px 18px; }
  .panel-head { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
  .panel-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0; }
  .count { font-size: 11px; color: var(--color-crush-muted); background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 9999px; padding: 0 8px; line-height: 18px; }
  .panel-head .ghost-btn { margin-left: auto; }

  .tbl { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tbl th { text-align: left; font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); font-weight: 500; padding: 0 10px 8px; border-bottom: 1px solid var(--color-crush-border); }
  .tbl th.r, .tbl td.r { text-align: right; }
  .tbl td { padding: 9px 10px; border-bottom: 1px solid rgba(42,42,53,0.4); }
  .tbl tbody tr:last-child td { border-bottom: none; }
  .tbl tbody tr:hover { background: rgba(224,85,64,0.03); }
  .strong { font-weight: 500; }
  .dim { color: var(--color-crush-text-muted); }
  .sm { font-size: 12px; }
  .mono { font-family: var(--font-mono); }
  .port { color: var(--color-crush-orange); }

  .tag { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; padding: 1px 8px; border-radius: 9999px; background: rgba(224,85,64,0.1); color: var(--color-crush-orange); border: 1px solid rgba(224,85,64,0.2); }
  .tag.cached { background: rgba(74,222,128,0.1); color: var(--color-crush-green); border-color: rgba(74,222,128,0.2); }
  .tag.fail { background: rgba(239,68,68,0.1); color: var(--color-crush-red); border-color: rgba(239,68,68,0.2); }

  .kv { display: grid; grid-template-columns: 70px 1fr; gap: 8px 10px; margin: 0; font-size: 12px; }
  .kv dt { color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.04em; font-size: 10px; align-self: center; }
  .kv dd { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kv .path { color: var(--color-crush-muted); }

  .list { display: flex; flex-direction: column; }
  .srow { display: flex; align-items: center; gap: 8px; padding: 8px 0; border-bottom: 1px solid rgba(42,42,53,0.4); font-size: 13px; }
  .srow:last-child { border-bottom: none; }
  .srow .dim { flex: 1; }
  .srow .port { margin-left: auto; }

  .muted { color: var(--color-crush-text-muted); font-size: 13px; padding: 4px 0; }
</style>
