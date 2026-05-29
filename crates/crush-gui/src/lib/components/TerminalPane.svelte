<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import * as api from '$lib/tauri';
  import Icon from './Icon.svelte';

  let { runId, onClose }: { runId: string; onClose?: () => void } = $props();

  type Line = { text: string; kind: 'out' | 'err' | 'meta' | 'ok' | 'warn' };
  let lines = $state<Line[]>([]);
  let status = $state<'running' | 'exited' | 'failed'>('running');
  let unlisten: (() => void) | null = null;
  let el: HTMLDivElement | undefined = $state();

  async function push(text: string, kind: Line['kind']) {
    lines = [...lines.slice(-1000), { text, kind }];
    await tick();
    el?.scrollTo({ top: el.scrollHeight });
  }

  function fmtMB(b: number): string {
    return b < 1_000_000 ? `${(b / 1000).toFixed(0)} KB` : `${(b / 1_000_000).toFixed(0)} MB`;
  }

  $effect(() => {
    // single channel; kind is on the payload (RunEvent is internally tagged)
    api.onRunEvent(runId, (event) => {
      const e = event as any;
      switch (e.kind) {
        case 'detected':
          push(`↳ detected ${e.language}${e.framework ? ` · ${e.framework}` : ''} · :${e.port}${e.is_monorepo ? ` · monorepo (${e.dep_count} svc)` : ''}`, 'meta'); break;
        case 'warm-run': push('warm run — launching', 'meta'); break;
        case 'deps-fresh': push('dependencies fresh — node_modules up to date', 'meta'); break;
        case 'dep-started': push(`✓ ${e.name} started${e.native ? ' (native)' : ` · ${e.image}`}`, 'ok'); break;
        case 'dep-failed': push(`✗ ${e.name} failed: ${e.error}`, 'err'); break;
        case 'image-fresh': push('image fresh — skipping pack', 'meta'); break;
        case 'image-packed': push(`crushed to image${e.size_bytes ? ` (${fmtMB(e.size_bytes)})` : ''}`, 'ok'); break;
        case 'build-started': push(`build: ${e.command ?? ''}`, 'meta'); break;
        case 'build-output': push(e.line, e.stream === 'stderr' ? 'err' : 'out'); break;
        case 'build-finished': push(`build finished${e.duration_ms ? ` in ${(e.duration_ms / 1000).toFixed(1)}s` : ''}`, 'meta'); break;
        case 'spawning': push(`spawning${e.command ? `: ${e.command}` : ''}${e.port ? ` on :${e.port}` : ''}`, 'meta'); break;
        case 'app-output': push(e.line, e.stream === 'stderr' ? 'err' : 'out'); break;
        case 'port-bound': {
          const urls = (e.urls ?? []).map((u: [string, string]) => u[1]).join('  ');
          push(`✓ ready on :${e.port}${urls ? ` — ${urls}` : ''}`, 'ok'); break;
        }
        case 'warning': push(`! ${e.message ?? e.text ?? ''}`, 'warn'); break;
        case 'exited':
          status = e.code === 0 ? 'exited' : 'failed';
          push(`process exited (code ${e.code})`, e.code === 0 ? 'meta' : 'err'); break;
        default: break;
      }
    }).then(fn => { unlisten = fn; });
  });

  onDestroy(() => { unlisten?.(); });

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
    <span class="chrome-title">crush run · <span class="st {status}">{status}</span></span>
    <button class="stop-btn" onclick={stop}><Icon name="stop" size={11} fill /> {status === 'running' ? 'Stop' : 'Close'}</button>
  </div>
  <div class="terminal-content" bind:this={el}>
    {#each lines as line}
      <div class="line {line.kind}">{line.text}</div>
    {/each}
    {#if lines.length === 0}
      <div class="line meta"><span class="cursor">▋</span> starting run…</div>
    {/if}
  </div>
</div>

<style>
  .terminal-pane { border: 1px solid var(--color-crush-border); border-radius: 0.75rem; overflow: hidden; background: rgba(9, 9, 11, 0.95); }
  .terminal-chrome { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: rgba(26, 26, 34, 0.8); border-bottom: 1px solid var(--color-crush-border); }
  .chrome-dots { display: flex; gap: 6px; }
  .dot { width: 10px; height: 10px; border-radius: 50%; }
  .dot.red { background: #ff5f56; }
  .dot.yellow { background: #ffbd2e; }
  .dot.green { background: #27c93f; }
  .chrome-title { flex: 1; text-align: center; font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }
  .st { text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; }
  .st.running { color: var(--color-crush-green); }
  .st.exited { color: var(--color-crush-text-muted); }
  .st.failed { color: var(--color-crush-red); }
  .stop-btn { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: #ef4444; background: none; border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 4px; padding: 2px 8px; cursor: pointer; }

  .terminal-content { padding: 12px; max-height: 300px; min-height: 120px; overflow-y: auto; font-family: var(--font-mono); font-size: 11px; line-height: 1.6; }
  .line { white-space: pre-wrap; word-break: break-all; }
  .line.out { color: var(--color-crush-text); }
  .line.err { color: #fca5a5; }
  .line.meta { color: var(--color-crush-text-muted); }
  .line.ok { color: var(--color-crush-green); }
  .line.warn { color: #eab308; }
  .cursor { color: var(--color-crush-orange); animation: blink 1s step-end infinite; }
  @keyframes blink { 50% { opacity: 0; } }
</style>
