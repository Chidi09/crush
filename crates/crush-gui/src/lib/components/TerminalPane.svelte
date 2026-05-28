<script lang="ts">
  import { onDestroy } from 'svelte';
  import * as api from '$lib/tauri';
  import Icon from './Icon.svelte';

  let { runId, onClose }: { runId: string; onClose?: () => void } = $props();

  let lines: string[] = $state([]);
  let unlisten: (() => void) | null = null;

  $effect(() => {
    api.onRunEvent(runId, (event) => {
      if (event.kind === 'build-output' || event.kind === 'app-output') {
        lines = [...lines, `[${event.stream}] ${event.line}`];
      } else if (event.kind === 'detected') {
        lines = [...lines, `[detect] ${event.language} · ${event.framework}`];
      } else if (event.kind === 'exited') {
        lines = [...lines, `[exit] code ${event.code}`];
      } else if (event.kind === 'port-bound') {
        lines = [...lines, `[ready] port ${event.port} — ${event.urls.map(u => u[1]).join(', ')}`];
      }
    }).then(fn => { unlisten = fn; });
  });

  onDestroy(() => {
    unlisten?.();
  });

  function stop() {
    api.abortRun(runId).catch(console.error);
    onClose?.();
  }
</script>

<div class="terminal-pane">
  <div class="terminal-chrome">
    <div class="chrome-dots">
      <span class="dot red"></span>
      <span class="dot yellow"></span>
      <span class="dot green"></span>
    </div>
    <span class="chrome-title">crush run</span>
    <button class="stop-btn" onclick={stop}><Icon name="stop" size={11} fill /> Stop</button>
  </div>
  <div class="terminal-content">
    {#each lines as line}
      <div class="line">{line}</div>
    {/each}
    {#if lines.length === 0}
      <div class="line dim">Waiting for output…</div>
    {/if}
  </div>
</div>

<style>
  .terminal-pane {
    border: 1px solid var(--color-crush-border);
    border-radius: 0.75rem;
    overflow: hidden;
    background: rgba(9, 9, 11, 0.95);
  }

  .terminal-chrome {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(26, 26, 34, 0.8);
    border-bottom: 1px solid var(--color-crush-border);
  }

  .chrome-dots {
    display: flex;
    gap: 6px;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .dot.red { background: #ff5f56; }
  .dot.yellow { background: #ffbd2e; }
  .dot.green { background: #27c93f; }

  .chrome-title {
    flex: 1;
    text-align: center;
    font-size: 12px;
    color: var(--color-crush-text-muted);
    font-family: var(--font-mono);
  }

  .stop-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: #ef4444;
    background: none;
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 4px;
    padding: 2px 8px;
    cursor: pointer;
  }

  .terminal-content {
    padding: 12px;
    max-height: 250px;
    overflow-y: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.6;
  }

  .line { color: var(--color-crush-text); }
  .line.dim { color: var(--color-crush-text-muted); }
</style>
