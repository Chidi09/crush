<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import TerminalPane from '$lib/components/TerminalPane.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import { containers, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import { services, refreshServices } from '$lib/stores/services.svelte.ts';

  let projectPath: string | null = $state(null);
  let runningCount = $derived($containers.filter(c => c.status === 'running').length);
  let imageCount = $derived(0);
  let activeRunId: string | null = $state(null);
  let diskUsed = $state('—');

  onMount(async () => {
    startPolling();
    await refreshServices();

    try {
      const imgs = await api.listImages();
      imageCount = imgs.length;
      const totalBytes = imgs.reduce((s, i) => s + i.size_bytes, 0);
      diskUsed = totalBytes > 1_000_000_000
        ? `${(totalBytes / 1_000_000_000).toFixed(1)} GB`
        : `${(totalBytes / 1_000_000).toFixed(0)} MB`;
    } catch {}
  });

  onDestroy(() => {
    stopPolling();
  });

  async function openProject() {
    projectPath = await api.pickProjectDirectory();
  }

  async function runProject() {
    if (!projectPath) {
      projectPath = await api.pickProjectDirectory();
    }
    if (projectPath) {
      try {
        const runId = await api.runProject(projectPath);
        activeRunId = runId;
      } catch (e) {
        console.error('Failed to run project', e);
      }
    }
  }

  function closeTerminal() {
    activeRunId = null;
  }
</script>

<div class="dashboard">
  <header class="page-header">
    <h1>Dashboard</h1>
    <div class="header-meta">
      <span class="running-indicator">● {$containers.filter(c => c.status === 'running').length} running</span>
      <button class="run-btn" onclick={runProject}>crush <Icon name="play" size={12} fill /></button>
    </div>
  </header>

  {#if activeRunId}
    <div class="terminal-section">
      <TerminalPane runId={activeRunId} onClose={closeTerminal} />
    </div>
  {/if}

  <div class="grid">
    <div class="crush-card card-project">
      <h2>Current project</h2>
      {#if projectPath}
        <div class="project-info">
          <span class="project-name">{projectPath.split('\\').pop()?.split('/').pop()}</span>
          <span class="project-path">{projectPath}</span>
          <button class="btn-primary" onclick={runProject}>Run <Icon name="play" size={13} fill /></button>
        </div>
      {:else}
        <EmptyState
          title="No project open"
          description="Open a Crush project to get started"
          action="Open project…"
          onAction={openProject}
        />
      {/if}
    </div>

    <div class="crush-card card-services">
      <h2>Services</h2>
      {#if $services.length > 0}
        <div class="service-list">
          {#each $services as svc}
            <div class="service-item">
              <StatusBadge status="running" />
              <span class="svc-name">{svc.name}</span>
              <span class="svc-port">{svc.port}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted">No services running</p>
      {/if}
    </div>

    <div class="crush-card card-stats">
      <h2>Quick stats</h2>
      <div class="stats-grid">
        <div class="stat">
          <span class="stat-value">{runningCount}</span>
          <span class="stat-label">containers running</span>
        </div>
        <div class="stat">
          <span class="stat-value">{imageCount}</span>
          <span class="stat-label">images cached</span>
        </div>
        <div class="stat">
          <span class="stat-value">{diskUsed}</span>
          <span class="stat-label">disk used</span>
        </div>
      </div>
    </div>
  </div>

  {#if $containers.length > 0}
    <div class="crush-card card-recent">
      <h2>Running containers</h2>
      <div class="container-list">
        {#each $containers.filter(c => c.status === 'running') as c}
          <div class="container-row">
            <StatusBadge status={c.status} />
            <span class="c-name">{c.name}</span>
            <span class="c-image">{c.image}</span>
            <span class="c-up">
              {c.uptime_secs ? formatDuration(c.uptime_secs) : '—'}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<script lang="ts" module>
  function formatDuration(secs: number): string {
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m`;
    return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  }
</script>

<style>
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  .page-header h1 {
    font-size: 20px;
    font-weight: 600;
    margin: 0;
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .running-indicator {
    font-size: 13px;
    color: #4ade80;
  }

  .run-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-crush-orange);
    color: white;
    border: none;
    border-radius: 8px;
    padding: 6px 16px;
    font-size: 13px;
    cursor: pointer;
    font-family: var(--font-mono);
  }

  .terminal-section {
    margin-bottom: 24px;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 24px;
  }

  .crush-card {
    padding: 20px;
  }

  .crush-card h2 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-crush-text-muted);
    margin: 0 0 12px;
  }

  .project-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .project-name {
    font-size: 18px;
    font-weight: 600;
  }

  .project-path {
    font-size: 12px;
    color: var(--color-crush-text-muted);
    font-family: var(--font-mono);
  }

  .btn-primary {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-crush-orange);
    color: white;
    border: none;
    border-radius: 8px;
    padding: 8px 20px;
    font-size: 13px;
    cursor: pointer;
  }

  .service-list { display: flex; flex-direction: column; gap: 8px; }
  .service-item { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .svc-name { flex: 1; }
  .svc-port { color: var(--color-crush-text-muted); font-family: var(--font-mono); }

  .muted { color: var(--color-crush-text-muted); font-size: 13px; }

  .stats-grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }
  .stat { display: flex; flex-direction: column; }
  .stat-value { font-size: 24px; font-weight: 700; }
  .stat-label { font-size: 11px; color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.05em; }

  .card-recent { padding: 20px; }
  .container-list { display: flex; flex-direction: column; gap: 8px; }
  .container-row { display: flex; align-items: center; gap: 12px; font-size: 13px; padding: 8px 0; border-bottom: 1px solid var(--color-crush-border); }
  .c-name { flex: 1; font-weight: 500; }
  .c-image { color: var(--color-crush-text-muted); }
  .c-up { color: var(--color-crush-text-muted); font-family: var(--font-mono); font-size: 12px; }
</style>
