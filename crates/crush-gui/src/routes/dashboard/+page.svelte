<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import TerminalPane from '$lib/components/TerminalPane.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
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

  onMount(async () => {
    startPolling();
    const saved = localStorage.getItem(LAST_PROJECT);
    if (saved) { projectPath = saved; detectStack(saved); }
    await loadAll();
  });
  onDestroy(() => stopPolling());

  async function loadAll() {
    refreshing = true;
    await Promise.allSettled([
      refreshServices(),
      refreshImages(),
      api.systemInfo().then(s => sys = s).catch(() => {}),
      api.listBuildHistory(8).then(b => builds = b).catch(() => {}),
    ]);
    refreshing = false;
  }

  async function detectStack(path: string) {
    detecting = true;
    project = null;
    try {
      project = await api.detectProject(path);
    } catch (e) {
      console.error('detect failed', e);
    } finally {
      detecting = false;
    }
  }

  async function openProject() {
    const p = await api.pickProjectDirectory();
    if (p) {
      projectPath = p;
      localStorage.setItem(LAST_PROJECT, p);
      detectStack(p);
    }
  }

  async function runProject() {
    if (!projectPath) { await openProject(); return; }
    try {
      activeRunId = await api.runProject(projectPath);
    } catch (e) {
      console.error('run failed', e);
    }
  }

  async function stopAllServices() {
    stoppingAll = true;
    try {
      await Promise.allSettled($services.map(s => api.stopNativeService(s.name, s.project)));
      await refreshServices();
    } finally {
      stoppingAll = false;
    }
  }

  function closeTerminal() { activeRunId = null; }

  function fmtSize(bytes: number): string {
    if (!bytes) return '0 B';
    if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(0)} KB`;
    if (bytes < 1_000_000_000) return `${(bytes / 1_000_000).toFixed(0)} MB`;
    return `${(bytes / 1_000_000_000).toFixed(2)} GB`;
  }
  function timeAgo(ms: number): string {
    const s = Math.floor((Date.now() - ms) / 1000);
    if (s < 60) return `${s}s ago`;
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    return `${Math.floor(s / 86400)}d ago`;
  }
  function fmtMs(ms: number): string {
    return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
  }
  function baseName(p: string): string {
    return p.split(/[\\/]/).filter(Boolean).pop() ?? p;
  }
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
    <div class="terminal-section animate-slide-up">
      <TerminalPane runId={activeRunId} onClose={closeTerminal} />
    </div>
  {/if}

  <!-- Current project -->
  <div class="crush-card project-card">
    {#if projectPath}
      <div class="proj-top">
        <div>
          <div class="proj-label">Current project</div>
          <div class="proj-name">{project?.name ?? baseName(projectPath)}</div>
        </div>
        <div class="proj-actions">
          <button class="ghost-btn sm" onclick={openProject}>Change</button>
          <button class="btn-primary" onclick={runProject}>Run <Icon name="play" size={13} fill /></button>
        </div>
      </div>
      <div class="proj-path">{projectPath}</div>
      {#if detecting}
        <div class="chips"><span class="chip skel">detecting…</span></div>
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

  <!-- Stat cards -->
  <div class="stats">
    <div class="crush-card stat">
      <div class="stat-icon"><Icon name="containers" size={18} /></div>
      <div><div class="stat-value">{runningCount}</div><div class="stat-label">Containers running</div></div>
    </div>
    <div class="crush-card stat">
      <div class="stat-icon"><Icon name="images" size={18} /></div>
      <div><div class="stat-value">{$images.length}</div><div class="stat-label">Images cached</div></div>
    </div>
    <div class="crush-card stat">
      <div class="stat-icon"><Icon name="services" size={18} /></div>
      <div><div class="stat-value">{$services.length}</div><div class="stat-label">Services running</div></div>
    </div>
    <div class="crush-card stat">
      <div class="stat-icon"><Icon name="disk" size={18} /></div>
      <div><div class="stat-value">{sys ? fmtSize(sys.disk_used_bytes) : '—'}</div><div class="stat-label">Disk used</div></div>
    </div>
  </div>

  <!-- Services + Recent builds -->
  <div class="two-col">
    <div class="crush-card panel">
      <div class="panel-head">
        <h2>Services</h2>
        {#if $services.length}<button class="ghost-btn sm" onclick={stopAllServices} disabled={stoppingAll}>{stoppingAll ? 'Stopping…' : 'Stop all'}</button>{/if}
      </div>
      {#if $services.length}
        <div class="list">
          {#each $services as svc}
            <div class="list-row">
              <StatusBadge status="running" />
              <span class="lr-name">{svc.name}</span>
              <span class="lr-kind">{svc.kind}</span>
              <span class="lr-port mono">:{svc.port}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted">No native services running</p>
      {/if}
    </div>

    <div class="crush-card panel">
      <div class="panel-head"><h2>Recent builds</h2></div>
      {#if builds.length}
        <div class="list">
          {#each builds as b}
            <div class="list-row build-row">
              <span class="lr-name">{b.project_name}</span>
              <span class="lr-lang">{b.language}{b.framework && b.framework !== 'none' ? ` · ${b.framework}` : ''}</span>
              <span class="build-dur mono">{fmtMs(b.duration_ms)}</span>
              <span class="build-tag" class:cached={b.was_cached} class:fail={!b.success}>
                {!b.success ? 'failed' : b.was_cached ? 'cached' : 'fresh'}
              </span>
              <span class="lr-ago">{timeAgo(b.timestamp_ms)}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted">No builds yet — run a project to see history</p>
      {/if}
    </div>
  </div>

  <!-- Running containers -->
  {#if runningCount > 0}
    <div class="crush-card panel">
      <div class="panel-head"><h2>Running containers</h2></div>
      <div class="list">
        {#each $containers.filter(c => c.status === 'running') as c}
          <div class="list-row">
            <StatusBadge status={c.status} />
            <span class="lr-name">{c.name}</span>
            <span class="lr-kind mono">{c.image}</span>
            {#if c.ports.length}<span class="lr-port mono">:{c.ports[0].host_port}</span>{/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .dashboard { display: flex; flex-direction: column; gap: 16px; }

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
  .ghost-btn { display: inline-flex; align-items: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.75rem; padding: 7px 10px; font-size: 13px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn.sm { padding: 5px 12px; }
  .ghost-btn.spinning :global(svg) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .terminal-section { margin: 0; }

  .project-card { padding: 20px; }
  .proj-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
  .proj-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .proj-name { font-size: 20px; font-weight: 600; margin-top: 2px; }
  .proj-actions { display: flex; gap: 8px; flex-shrink: 0; }
  .proj-path { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-muted); margin: 6px 0 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .btn-primary { display: inline-flex; align-items: center; gap: 6px; background: var(--color-crush-orange); color: white; border: none; border-radius: 0.75rem; padding: 7px 18px; font-size: 13px; cursor: pointer; transition: background 0.15s; }
  .btn-primary:hover { background: var(--color-crush-orange-light); }

  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .chip { font-size: 12px; padding: 3px 10px; border-radius: 9999px; border: 1px solid var(--color-crush-border); color: var(--color-crush-text); background: rgba(255,255,255,0.02); }
  .chip.accent { border-color: rgba(224,85,64,0.3); background: rgba(224,85,64,0.08); color: var(--color-crush-orange); font-weight: 500; }
  .chip.muted-chip { color: var(--color-crush-text-muted); }
  .chip.skel { color: var(--color-crush-text-muted); }

  .stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
  .stat { padding: 16px; display: flex; align-items: center; gap: 14px; }
  .stat-icon { width: 38px; height: 38px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; border-radius: 10px; background: rgba(224,85,64,0.1); color: var(--color-crush-orange); }
  .stat-value { font-size: 22px; font-weight: 700; line-height: 1.1; }
  .stat-label { font-size: 11px; color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.04em; margin-top: 2px; }

  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .panel { padding: 18px 20px; }
  .panel-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .panel-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0; }

  .list { display: flex; flex-direction: column; }
  .list-row { display: flex; align-items: center; gap: 12px; padding: 9px 0; font-size: 13px; border-bottom: 1px solid rgba(42,42,53,0.5); }
  .list-row:last-child { border-bottom: none; }
  .lr-name { font-weight: 500; }
  .lr-kind, .lr-lang { color: var(--color-crush-text-muted); font-size: 12px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lr-port { color: var(--color-crush-orange); margin-left: auto; }
  .lr-ago { color: var(--color-crush-muted); font-size: 11px; }
  .mono { font-family: var(--font-mono); }

  .build-row { gap: 10px; }
  .build-dur { color: var(--color-crush-text-muted); font-size: 12px; }
  .build-tag { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; padding: 1px 8px; border-radius: 9999px; background: rgba(224,85,64,0.1); color: var(--color-crush-orange); border: 1px solid rgba(224,85,64,0.2); }
  .build-tag.cached { background: rgba(74,222,128,0.1); color: var(--color-crush-green); border-color: rgba(74,222,128,0.2); }
  .build-tag.fail { background: rgba(239,68,68,0.1); color: var(--color-crush-red); border-color: rgba(239,68,68,0.2); }

  .muted { color: var(--color-crush-text-muted); font-size: 13px; }
</style>
