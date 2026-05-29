<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import { containers, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import type { LogLine, DiagnosisResult } from '$lib/tauri';
  import { tick } from 'svelte';

  let selectedContainerId = $state<string | null>(null);
  let logLines = $state<LogLine[]>([]);
  let filterLevel = $state<'all' | 'error'>('all');
  let unlistenLogs: (() => void) | null = null;

  let diagnosis = $state<DiagnosisResult | null>(null);
  let diagnosing = $state(false);
  let diagnoseError = $state<string | null>(null);
  let autoscroll = $state(true);
  let logEl: HTMLDivElement | undefined = $state();

  let displayLines = $derived(
    filterLevel === 'error' ? logLines.filter(l => l.text.includes('ERROR') || l.stream === 'stderr') : logLines
  );
  let errorCount = $derived(logLines.filter(l => l.stream === 'stderr' || l.text.includes('ERROR')).length);

  onMount(() => startPolling());
  onDestroy(() => {
    stopPolling();
    unlistenLogs?.();
  });

  async function selectContainer(id: string) {
    unlistenLogs?.();
    selectedContainerId = id;
    logLines = [];
    diagnosis = null;
    diagnoseError = null;

    try {
      await api.subscribeLogs(id);
      unlistenLogs = await api.onLogLine(id, async (line) => {
        logLines = [...logLines.slice(-1000), line];
        if (autoscroll) { await tick(); logEl?.scrollTo({ top: logEl.scrollHeight }); }
      });
    } catch (e) {
      console.error('Failed to subscribe', e);
    }
  }

  function onScroll() {
    if (!logEl) return;
    // re-enable autoscroll only when the user is pinned to the bottom
    autoscroll = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight < 24;
  }

  async function diagnose() {
    const lines = logLines.filter(l => l.stream === 'stderr' || l.text.includes('ERROR')).map(l => l.text);
    if (lines.length === 0 || diagnosing) return;
    diagnosing = true;
    diagnosis = null;
    diagnoseError = null;
    try {
      diagnosis = await api.diagnoseLogs(lines.slice(-200));
    } catch (e) {
      diagnoseError = String(e);
    } finally {
      diagnosing = false;
    }
  }
</script>

<div class="logs-page">
  <header class="page-header">
    <h1>Logs</h1>
  </header>

  <div class="logs-layout">
    <div class="sidebar-list">
      <h3>Containers</h3>
      {#each $containers as c}
        <button
          class="sidebar-item"
          class:active={selectedContainerId === c.id}
          onclick={() => selectContainer(c.id)}
        >
          <StatusBadge status={c.status} />
          <span>{c.name}</span>
        </button>
      {/each}
    </div>

    <div class="logs-main">
      {#if selectedContainerId}
        <div class="log-controls">
          <div class="filter-tabs">
            {#each ['all', 'error'] as level}
              <button
                class="filter-tab"
                class:active={filterLevel === level}
                onclick={() => filterLevel = level as 'all' | 'error'}
              >
                {level === 'all' ? 'All' : 'ERROR'}
              </button>
            {/each}
          </div>
          <button class="diagnose-btn" onclick={diagnose} disabled={errorCount === 0 || diagnosing} title={errorCount === 0 ? 'No errors to diagnose' : ''}>
            <Icon name="sparkles" size={13} />
            {diagnosing ? 'Diagnosing…' : 'AI Diagnose'}
            {#if errorCount > 0}<span class="err-pill">{errorCount}</span>{/if}
          </button>
        </div>

        <div class="log-output" bind:this={logEl} onscroll={onScroll}>
          {#each displayLines as line}
            <div class="log-line" class:stderr={line.stream === 'stderr'}>
              <span class="log-ts">{line.ts}</span>
              <span class="log-stream">{line.stream === 'stdout' ? 'OUT' : 'ERR'}</span>
              <span class="log-text">{line.text}</span>
            </div>
          {/each}
          {#if displayLines.length === 0}
            <p class="muted" style="padding: 12px;">No log lines yet</p>
          {/if}
        </div>

        {#if diagnosing || diagnosis || diagnoseError}
          <div class="diag-card animate-slide-up">
            <div class="diag-head">
              <Icon name="sparkles" size={13} /> Crush Diagnostic
              {#if diagnosis}<button class="diag-dismiss" onclick={() => diagnosis = null}>×</button>{/if}
            </div>
            {#if diagnosing}
              <p class="diag-body">Analyzing {errorCount} error line{errorCount === 1 ? '' : 's'}…</p>
            {:else if diagnoseError}
              <p class="diag-body diag-err">{diagnoseError}</p>
            {:else if diagnosis}
              <p class="diag-body">{diagnosis.summary}</p>
              {#if diagnosis.fix}
                <div class="diag-fix"><span class="diag-fix-label">Fix</span> <code>{diagnosis.fix}</code></div>
              {/if}
            {/if}
          </div>
        {/if}
      {:else}
        <div class="select-prompt">
          <p class="muted">Select a container from the sidebar</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .page-header { margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }

  .logs-layout { display: flex; gap: 16px; height: calc(100vh - 120px); }

  .sidebar-list { width: 200px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px; }
  .sidebar-list h3 { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0 0 8px; }
  .sidebar-item { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: none; border: none; border-radius: 8px; cursor: pointer; font-size: 13px; color: var(--color-crush-text); text-align: left; width: 100%; }
  .sidebar-item.active { background: rgba(224,85,64,0.1); }
  .sidebar-item:hover { background: var(--color-crush-surface); }

  .logs-main { flex: 1; display: flex; flex-direction: column; border: 1px solid var(--color-crush-border); border-radius: 1rem; overflow: hidden; }

  .log-controls { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; background: var(--color-crush-surface); border-bottom: 1px solid var(--color-crush-border); }
  .filter-tabs { display: flex; gap: 4px; }
  .filter-tab { font-size: 12px; padding: 4px 12px; border-radius: 4px; background: none; border: none; color: var(--color-crush-text-muted); cursor: pointer; }
  .filter-tab.active { background: var(--color-crush-orange); color: white; }
  .diagnose-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; padding: 4px 12px; border-radius: 6px; border: 1px solid rgba(224,85,64,0.3); background: none; color: var(--color-crush-orange); cursor: pointer; transition: background 0.15s; }
  .diagnose-btn:hover:not(:disabled) { background: rgba(224,85,64,0.1); }
  .diagnose-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .err-pill { background: var(--color-crush-red); color: white; border-radius: 9999px; font-size: 10px; padding: 0 6px; line-height: 16px; min-width: 16px; text-align: center; }

  .log-output { flex: 1; overflow-y: auto; padding: 8px 0; font-family: var(--font-mono); font-size: 11px; background: rgba(9,9,11,0.9); }

  .diag-card { margin: 12px; border: 1px solid rgba(224,85,64,0.2); background: rgba(224,85,64,0.04); border-radius: 0.75rem; padding: 12px 14px; }
  .diag-head { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 700; color: var(--color-crush-orange); text-transform: uppercase; letter-spacing: 0.05em; }
  .diag-dismiss { margin-left: auto; background: none; border: none; color: var(--color-crush-text-muted); font-size: 16px; line-height: 1; cursor: pointer; }
  .diag-body { margin: 8px 0 0; font-size: 13px; color: var(--color-crush-text); }
  .diag-err { color: var(--color-crush-red); font-family: var(--font-mono); font-size: 12px; }
  .diag-fix { margin-top: 8px; font-size: 12px; }
  .diag-fix-label { color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.05em; font-size: 11px; margin-right: 6px; }
  .diag-fix code { font-family: var(--font-mono); background: rgba(9,9,11,0.6); border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 2px 8px; color: var(--color-crush-orange); }
  .log-line { display: flex; gap: 8px; padding: 1px 12px; }
  .log-line.stderr { background: rgba(239,68,68,0.03); }
  .log-ts { color: var(--color-crush-muted); flex-shrink: 0; }
  .log-stream { color: var(--color-crush-muted); flex-shrink: 0; width: 28px; }
  .log-text { color: var(--color-crush-text); }

  .select-prompt { display: flex; align-items: center; justify-content: center; flex: 1; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }
</style>
