<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import * as api from '$lib/tauri';
  import { containers, loading, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import type { ContainerSummary } from '$lib/tauri';

  let selectedContainer: ContainerSummary | null = $state(null);
  let searchQuery = $state('');

  let filtered = $derived(
    $containers.filter(c =>
      !searchQuery || c.name.toLowerCase().includes(searchQuery.toLowerCase()) || c.image.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  onMount(() => startPolling());
  onDestroy(() => stopPolling());

  function selectContainer(c: ContainerSummary) {
    selectedContainer = c;
    api.subscribeLogs(c.id).catch(() => {});
  }

  function closeDrawer() {
    if (selectedContainer) {
      api.unsubscribeLogs(selectedContainer.id).catch(() => {});
    }
    selectedContainer = null;
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
  <div class="drawer-overlay" onclick={closeDrawer}></div>
  <div class="drawer">
    <div class="drawer-header">
      <h2>{selectedContainer.name}</h2>
      <button class="close-btn" onclick={closeDrawer}>×</button>
    </div>
    <div class="drawer-body">
      <div class="drawer-info">
        <span>Image: {selectedContainer.image}</span>
        <span>Status: <StatusBadge status={selectedContainer.status} /></span>
        <span>ID: {selectedContainer.id.substring(0, 12)}</span>
      </div>
      <div class="crush-mono-panel logs-panel">
        <p class="muted" style="padding: 12px;">Streaming logs… (open Logs page for full view)</p>
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

  .drawer-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 40; }
  .drawer { position: fixed; top: 0; right: 0; width: 400px; height: 100vh; background: var(--color-crush-dark); border-left: 1px solid var(--color-crush-border); z-index: 50; display: flex; flex-direction: column; }
  .drawer-header { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--color-crush-border); }
  .drawer-header h2 { font-size: 16px; font-weight: 600; margin: 0; }
  .close-btn { background: none; border: none; color: var(--color-crush-text-muted); font-size: 20px; cursor: pointer; }
  .drawer-body { flex: 1; padding: 16px 20px; overflow-y: auto; }
  .drawer-info { display: flex; flex-direction: column; gap: 6px; margin-bottom: 16px; font-size: 13px; color: var(--color-crush-text-muted); }
  .logs-panel { height: 200px; }
</style>
