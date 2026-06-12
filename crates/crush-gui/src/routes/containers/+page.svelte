<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import { containers, loading, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import type { ContainerSummary, LogLine } from '$lib/tauri';

  let selectedContainer: ContainerSummary | null = $state(null);
  let searchQuery = $state('');
  let logLines = $state<LogLine[]>([]);
  let unlistenLogs: (() => void) | null = null;
  let unlistenReplay: (() => void) | null = null;
  let logEl: HTMLDivElement | undefined = $state();
  let stopping = $state(false);
  let copied = $state(false);

  let filtered = $derived(
    $containers.filter(c =>
      !searchQuery || c.name.toLowerCase().includes(searchQuery.toLowerCase()) || c.image.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  onMount(() => startPolling());
  onDestroy(() => { stopPolling(); unlistenLogs?.(); unlistenReplay?.(); });

  async function selectContainer(c: ContainerSummary) {
    unlistenLogs?.();
    unlistenReplay?.();
    selectedContainer = c;
    logLines = [];
    copied = false;
    try {
      await api.subscribeLogs(c.id);
      unlistenLogs = await api.onLogLine(c.id, async (line) => {
        logLines = [...logLines.slice(-500), line];
        await tick();
        logEl?.scrollTo({ top: logEl.scrollHeight });
      });
      unlistenReplay = await api.onLogReplay(c.id, async (lines) => {
        logLines = [...logLines, ...lines].slice(-500);
        await tick();
        logEl?.scrollTo({ top: logEl.scrollHeight });
      });
    } catch { /* container may have no logs yet */ }
  }

  function closeDrawer() {
    if (selectedContainer) api.unsubscribeLogs(selectedContainer.id).catch(() => {});
    unlistenLogs?.();
    unlistenLogs = null;
    unlistenReplay?.();
    unlistenReplay = null;
    selectedContainer = null;
  }

  async function stopContainer() {
    if (!selectedContainer || stopping) return;
    stopping = true;
    try {
      await api.stopContainer(selectedContainer.id);
    } catch (e) {
      console.error('Stop failed', e);
    } finally {
      stopping = false;
    }
  }

  function copyId() {
    if (!selectedContainer) return;
    navigator.clipboard.writeText(selectedContainer.id).then(() => {
      copied = true;
      setTimeout(() => copied = false, 1500);
    });
  }
</script>

<div class="page">
  <header class="page-header">
    <h1>Containers</h1>
    <div class="header-actions">
      <input class="crush-input" type="text" placeholder="Search…" bind:value={searchQuery} />
    </div>
  </header>

  {#if $loading}
    <p class="muted">Loading…</p>
  {:else if filtered.length === 0}
    <p class="muted">No containers found</p>
  {:else}
    <div class="table">
      <div class="table-header">
        <span>NAME</span>
        <span>IMAGE</span>
        <span>STATUS</span>
        <span>UPTIME</span>
      </div>
      {#each filtered as c}
        <button class="table-row" onclick={() => selectContainer(c)}>
          <span class="cell-name">{c.name}</span>
          <span class="cell-image">{c.image}</span>
          <span><StatusBadge status={c.status} /></span>
          <span class="cell-up">{c.uptime_secs ? formatDuration(c.uptime_secs) : '—'}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if selectedContainer}
  <button class="drawer-overlay" onclick={closeDrawer} aria-label="Close"></button>
  <div class="drawer animate-slide-up">
    <div class="drawer-header">
      <div class="drawer-title">
        <StatusBadge status={selectedContainer.status} />
        <h2>{selectedContainer.name}</h2>
      </div>
      <button class="close-btn" onclick={closeDrawer}>×</button>
    </div>
    <div class="drawer-body">
      <div class="drawer-actions">
        <button class="stop-action" onclick={stopContainer} disabled={stopping || selectedContainer.status !== 'running'}>
          <Icon name="stop" size={12} fill /> {stopping ? 'Stopping…' : 'Stop'}
        </button>
        <button class="copy-id" onclick={copyId}>
          <Icon name={copied ? 'check' : 'copy'} size={12} /> {copied ? 'Copied' : 'Copy ID'}
        </button>
      </div>

      <dl class="info-grid">
        <dt>Image</dt><dd class="mono">{selectedContainer.image}</dd>
        <dt>ID</dt><dd class="mono">{selectedContainer.id.substring(0, 16)}</dd>
        <dt>Uptime</dt><dd>{selectedContainer.uptime_secs ? formatDuration(selectedContainer.uptime_secs) : '—'}</dd>
        {#if selectedContainer.ports.length > 0}
          <dt>Ports</dt>
          <dd class="mono">{#each selectedContainer.ports as p}<span class="port">{p.host_port}→{p.container_port}</span>{/each}</dd>
        {/if}
      </dl>

      <div class="logs-head">Logs</div>
      <div class="crush-mono-panel logs-panel" bind:this={logEl}>
        {#each logLines as line}
          <div class="dlog-line" class:stderr={line.stream === 'stderr'}>{line.text}</div>
        {/each}
        {#if logLines.length === 0}
          <p class="muted" style="padding: 12px;">No log output yet…</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<script lang="ts" module>
  function formatDuration(secs: number): string {
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
    return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  }
</script>

<style>
  .page { position: relative; }
  .page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .header-actions { display: flex; gap: 8px; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }

  .table { border: 1px solid var(--color-crush-border); border-radius: 1rem; overflow: hidden; }
  .table-header { display: grid; grid-template-columns: 2fr 2fr 1fr 1fr; gap: 12px; padding: 12px 16px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); background: var(--color-crush-surface); border-bottom: 1px solid var(--color-crush-border); }
  .table-row { display: grid; grid-template-columns: 2fr 2fr 1fr 1fr; gap: 12px; padding: 12px 16px; font-size: 13px; background: none; border: none; border-bottom: 1px solid var(--color-crush-border); color: var(--color-crush-text); cursor: pointer; text-align: left; width: 100%; }
  .table-row:last-child { border-bottom: none; }
  .table-row:hover { background: rgba(224, 85, 64, 0.03); }
  .cell-name { font-weight: 500; }
  .cell-image { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text-muted); }
  .cell-up { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text-muted); }

  .drawer-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 40; border: none; padding: 0; cursor: default; }
  .drawer { position: fixed; top: 0; right: 0; width: 440px; height: 100vh; background: var(--color-crush-dark); border-left: 1px solid var(--color-crush-border); z-index: 50; display: flex; flex-direction: column; }
  .drawer-header { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--color-crush-border); }
  .drawer-title { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .drawer-header h2 { font-size: 16px; font-weight: 600; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .close-btn { background: none; border: none; color: var(--color-crush-text-muted); font-size: 20px; cursor: pointer; flex-shrink: 0; }
  .drawer-body { flex: 1; padding: 16px 20px; overflow-y: auto; display: flex; flex-direction: column; min-height: 0; }

  .drawer-actions { display: flex; gap: 8px; margin-bottom: 16px; }
  .stop-action { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-red); background: none; border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; padding: 6px 14px; cursor: pointer; }
  .stop-action:hover:not(:disabled) { background: rgba(239,68,68,0.1); }
  .stop-action:disabled { opacity: 0.4; cursor: not-allowed; }
  .copy-id { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 6px 14px; cursor: pointer; }
  .copy-id:hover { color: var(--color-crush-text); }

  .info-grid { display: grid; grid-template-columns: 80px 1fr; gap: 6px 12px; margin: 0 0 20px; font-size: 13px; }
  .info-grid dt { color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.05em; font-size: 11px; align-self: center; }
  .info-grid dd { margin: 0; overflow: hidden; text-overflow: ellipsis; }
  .info-grid .mono { font-family: var(--font-mono); font-size: 12px; }
  .port { display: inline-block; font-family: var(--font-mono); font-size: 11px; background: rgba(224,85,64,0.08); border: 1px solid rgba(224,85,64,0.2); color: var(--color-crush-orange); border-radius: 6px; padding: 1px 8px; margin-right: 6px; }

  .logs-head { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin-bottom: 8px; }
  .logs-panel { flex: 1; min-height: 120px; overflow-y: auto; padding: 8px 12px; }
  .dlog-line { color: var(--color-crush-text); padding: 1px 0; white-space: pre-wrap; word-break: break-all; }
  .dlog-line.stderr { color: #fca5a5; }
</style>
