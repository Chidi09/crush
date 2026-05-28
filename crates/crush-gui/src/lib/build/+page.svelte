<script lang="ts">
  import { onMount } from 'svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import * as api from '$lib/tauri';
  import type { BuildSummary } from '$lib/tauri';

  let builds = $state<BuildSummary[]>([]);
  let loading = $state(true);

  onMount(async () => {
    try {
      builds = await api.listBuildHistory(50);
    } catch (e) {
      console.error('Failed to load build history', e);
    } finally {
      loading = false;
    }
  });

  function ago(ms: number): string {
    const secs = (Date.now() - ms / 1_000_000) / 1000;
    if (secs < 60) return 'just now';
    if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
    return `${Math.floor(secs / 86400)}d ago`;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(0)} KB`;
    return `${(bytes / 1_000_000).toFixed(0)} MB`;
  }

  function durationStr(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }
</script>

<div class="page">
  <header class="page-header">
    <h1>Build History</h1>
  </header>

  {#if loading}
    <p class="muted">Loading…</p>
  {:else if builds.length === 0}
    <EmptyState title="No builds yet" description="Run a project to see build history" />
  {:else}
    <div class="build-list">
      {#each builds as b}
        <div class="crush-card build-card">
          <div class="build-header">
            <span class="build-project">{b.project_name}</span>
            <span class="build-duration">{durationStr(b.duration_ms)}</span>
            <span class="build-type" class:cached={b.was_cached} class:fresh={!b.was_cached}>
              {b.was_cached ? '● cached' : '○ fresh'}
            </span>
          </div>
          <div class="build-meta">
            {ago(b.timestamp_ms)} · {b.language}
          </div>
          <div class="build-bar">
            <div class="bar-fill" style="width: {Math.min(b.duration_ms / 200, 100)}%;"
                 class:bar-cached={b.was_cached} class:bar-fresh={!b.was_cached}></div>
          </div>
          <div class="build-footer">
            <span>{formatSize(b.size_bytes)}</span>
            <span class="mono">{b.digest.substring(0, 19)}…</span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page-header { margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }

  .build-list { display: flex; flex-direction: column; gap: 8px; }
  .build-card { padding: 16px 20px; }

  .build-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .build-project { font-size: 14px; font-weight: 600; flex: 1; }
  .build-duration { font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }
  .build-type { font-size: 11px; }
  .build-type.cached { color: var(--color-crush-muted); }
  .build-type.fresh { color: var(--color-crush-orange); }

  .build-meta { font-size: 12px; color: var(--color-crush-text-muted); margin-bottom: 8px; }

  .build-bar { height: 6px; background: var(--color-crush-border); border-radius: 3px; overflow: hidden; margin-bottom: 8px; }
  .bar-cached { background: var(--color-crush-border); }
  .bar-fresh { background: var(--color-crush-orange); }
  .bar-fill { height: 100%; border-radius: 3px; transition: width 0.3s; }

  .build-footer { display: flex; justify-content: space-between; font-size: 12px; color: var(--color-crush-text-muted); }
  .mono { font-family: var(--font-mono); }
</style>
