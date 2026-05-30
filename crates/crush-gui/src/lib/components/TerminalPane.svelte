<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import * as api from '$lib/tauri';

  let {
    runId,
    onStatus,
    onPort,
    onUrl,
    onLog,
  }: {
    runId: string;
    /** report run status up to the parent (Run Overview owns the actions) */
    onStatus?: (s: 'running' | 'exited' | 'failed') => void;
    /** report the bound port up so the parent can render a live preview */
    onPort?: (p: number) => void;
    /** report the best preview URL (prefers swagger/docs for backends) */
    onUrl?: (url: string) => void;
    /** stream each log line up so the parent can persist it per deployment */
    onLog?: (phase: 'build' | 'runtime', text: string) => void;
  } = $props();

  type Seg = { text: string; style: string };
  type Phase = 'build' | 'runtime';
  type Line = { segs: Seg[]; kind: 'out' | 'err' | 'meta' | 'ok' | 'warn'; phase: Phase };
  let lines = $state<Line[]>([]);
  let status = $state<'running' | 'exited' | 'failed'>('running');
  let unlisten: (() => void) | null = null;
  let el: HTMLDivElement | undefined = $state();

  // Vercel-style split: build/setup output vs the running app's runtime output.
  // Jamming both into one stream gets jumbled — keep them as separate tabs.
  let tab = $state<Phase>('build');
  let userPickedTab = $state(false);
  let buildLines = $derived(lines.filter(l => l.phase === 'build'));
  let runtimeLines = $derived(lines.filter(l => l.phase === 'runtime'));
  let shown = $derived(tab === 'build' ? buildLines : runtimeLines);

  // Minimal ANSI SGR → styled segments so program output (vite, etc.) renders
  // with real colors instead of raw escape codes.
  const FG: Record<number, string> = {
    30: '#6b6b80', 31: '#fca5a5', 32: '#4ade80', 33: '#eab308', 34: '#60a5fa',
    35: '#c084fc', 36: '#22d3ee', 37: '#e8e8ed',
    90: '#9a9ab0', 91: '#fca5a5', 92: '#86efac', 93: '#fde047', 94: '#93c5fd',
    95: '#d8b4fe', 96: '#67e8f9', 97: '#ffffff',
  };
  function parseAnsi(input: string): Seg[] {
    const segs: Seg[] = [];
    // eslint-disable-next-line no-control-regex
    const re = /\x1b\[([0-9;]*)m/g;
    let last = 0; let m: RegExpExecArray | null;
    let fg = ''; let bold = false; let dim = false; let italic = false; let underline = false;
    const style = () => {
      const p: string[] = [];
      if (fg) p.push(`color:${fg}`);
      if (bold) p.push('font-weight:700');
      if (dim) p.push('opacity:0.6');
      if (italic) p.push('font-style:italic');
      if (underline) p.push('text-decoration:underline');
      return p.join(';');
    };
    const emit = (t: string) => { if (t) segs.push({ text: t, style: style() }); };
    while ((m = re.exec(input)) !== null) {
      emit(input.slice(last, m.index));
      last = re.lastIndex;
      const codes = m[1] === '' ? [0] : m[1].split(';').map(Number);
      for (let i = 0; i < codes.length; i++) {
        const c = codes[i];
        if (c === 0) { fg = ''; bold = dim = italic = underline = false; }
        else if (c === 1) bold = true;
        else if (c === 2) dim = true;
        else if (c === 22) { bold = false; dim = false; }
        else if (c === 3) italic = true;
        else if (c === 23) italic = false;
        else if (c === 4) underline = true;
        else if (c === 24) underline = false;
        else if (c === 39) fg = '';
        else if (FG[c]) fg = FG[c];
        else if (c === 38 || c === 48) { i += codes[i + 1] === 5 ? 2 : codes[i + 1] === 2 ? 4 : 0; }
      }
    }
    emit(input.slice(last));
    return segs.length ? segs : [{ text: input, style: '' }];
  }

  async function push(text: string, kind: Line['kind'], phase: Phase = 'build') {
    const segs = kind === 'out' || kind === 'err' ? parseAnsi(text) : [{ text, style: '' }];
    lines = [...lines.slice(-1500), { segs, kind, phase }];
    onLog?.(phase, text);
    if (tab === phase) { await tick(); el?.scrollTo({ top: el.scrollHeight }); }
  }
  // Once the app starts running, jump to the Runtime tab (unless the user
  // has manually chosen a tab).
  function enterRuntime() { if (!userPickedTab) tab = 'runtime'; }
  function pickTab(t: Phase) { tab = t; userPickedTab = true; }

  function setStatus(s: 'running' | 'exited' | 'failed') { status = s; onStatus?.(s); }
  function setPort(p: number | undefined) { if (p) onPort?.(p); }

  // The detector emits the app's entry URLs (e.g. a backend's swagger-ui path).
  // Prefer a docs/swagger URL, else the first URL — that's the meaningful preview.
  function pickUrl(urls: [string, string][]): string | null {
    if (!urls || !urls.length) return null;
    const docs = urls.find(([, u]) => /swagger|\/docs|redoc|openapi|\/api-docs/i.test(u));
    return (docs ?? urls[0])[1] ?? null;
  }

  // Dev servers (Vite, etc.) may bump to a free port (5173 → 5177). The detected
  // port can be wrong, so scan stdout for the *actual* bound URL and trust that.
  function scanPort(line: string) {
    const clean = line.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '').replace(/\x1b\]8;;.*?\x1b\\/g, '');
    const m = /https?:\/\/(?:localhost|127\.0\.0\.1|0\.0\.0\.0):(\d+)/i.exec(clean);
    if (m) {
      setPort(Number(m[1]));
    }
  }

  function fmtMB(b: number): string {
    return b < 1_000_000 ? `${(b / 1000).toFixed(0)} KB` : `${(b / 1_000_000).toFixed(0)} MB`;
  }

  $effect(() => {
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
        case 'build-output': scanPort(e.line); push(e.line, e.stream === 'stderr' ? 'err' : 'out'); break;
        case 'build-finished': push(`build finished${e.duration_ms ? ` in ${(e.duration_ms / 1000).toFixed(1)}s` : ''}`, 'meta'); break;
        case 'spawning': enterRuntime(); push(`spawning${e.command ? `: ${e.command}` : ''}${e.port ? ` on :${e.port}` : ''}`, 'meta', 'runtime'); break;
        case 'app-output': scanPort(e.line); enterRuntime(); push(e.line, e.stream === 'stderr' ? 'err' : 'out', 'runtime'); break;
        case 'port-bound': {
          setPort(e.port);
          enterRuntime();
          const best = pickUrl(e.urls ?? []);
          if (best) onUrl?.(best);
          const urls = (e.urls ?? []).map((u: [string, string]) => u[1]).join('  ');
          push(`✓ ready on :${e.port}${urls ? ` — ${urls}` : ''}`, 'ok', 'runtime'); break;
        }
        case 'warning': push(`! ${e.message ?? e.text ?? ''}`, 'warn', 'runtime'); break;
        case 'exited':
          setStatus(e.code === 0 ? 'exited' : 'failed');
          push(`process exited (code ${e.code})`, e.code === 0 ? 'meta' : 'err', 'runtime'); break;
        default: break;
      }
    }).then(fn => { unlisten = fn; });
  });

  onDestroy(() => { unlisten?.(); });
</script>

<div class="terminal-pane">
  <div class="terminal-chrome">
    <div class="chrome-dots">
      <span class="dot red"></span>
      <span class="dot yellow"></span>
      <span class="dot green"></span>
    </div>
    <div class="log-tabs">
      <button class="ltab" class:active={tab === 'build'} onclick={() => pickTab('build')}>Build{#if buildLines.length}<span class="lcount">{buildLines.length}</span>{/if}</button>
      <button class="ltab" class:active={tab === 'runtime'} onclick={() => pickTab('runtime')}>Runtime{#if runtimeLines.length}<span class="lcount">{runtimeLines.length}</span>{/if}</button>
    </div>
    <span class="chrome-title">crush run · <span class="st {status}">{status}</span></span>
    <span class="chrome-spacer"></span>
  </div>
  <div class="terminal-content" bind:this={el}>
    {#each shown as line}
      <div class="line {line.kind}">{#each line.segs as s}<span style={s.style}>{s.text}</span>{/each}</div>
    {/each}
    {#if shown.length === 0}
      <div class="line meta"><span class="cursor">▋</span> {tab === 'build' ? 'starting build…' : 'waiting for the app to start…'}</div>
    {/if}
  </div>
</div>

<style>
  .terminal-pane { border: 1px solid var(--color-crush-border); border-radius: 0.75rem; overflow: hidden; background: rgba(9, 9, 11, 0.97); box-shadow: 0 10px 40px rgba(0,0,0,0.4); }
  .terminal-chrome { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: linear-gradient(180deg, rgba(34,34,44,0.9), rgba(26,26,34,0.9)); border-bottom: 1px solid var(--color-crush-border); }
  .chrome-dots { display: flex; gap: 6px; }
  .dot { width: 11px; height: 11px; border-radius: 50%; }
  .dot.red { background: #ff5f56; }
  .dot.yellow { background: #ffbd2e; }
  .dot.green { background: #27c93f; }
  .log-tabs { display: flex; gap: 2px; background: rgba(0,0,0,0.35); border-radius: 6px; padding: 2px; }
  .ltab { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-crush-text-muted); background: none; border: none; border-radius: 4px; padding: 3px 10px; cursor: pointer; font-family: var(--font-mono); }
  .ltab.active { background: rgba(255,255,255,0.08); color: var(--color-crush-text); }
  .lcount { font-size: 9px; background: rgba(255,255,255,0.1); border-radius: 9999px; padding: 0 5px; line-height: 14px; }
  .chrome-title { flex: 1; text-align: center; font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }
  .chrome-spacer { width: 100px; flex-shrink: 0; }
  .st { text-transform: uppercase; letter-spacing: 0.05em; font-size: 10px; }
  .st.running { color: var(--color-crush-green); }
  .st.exited { color: var(--color-crush-text-muted); }
  .st.failed { color: var(--color-crush-red); }

  .terminal-content { padding: 14px 16px; max-height: 320px; min-height: 130px; overflow-y: auto; font-family: var(--font-mono); font-size: 11.5px; line-height: 1.65; }
  .line { white-space: pre-wrap; word-break: break-word; }
  .line.out { color: var(--color-crush-text); }
  .line.err { color: #fca5a5; }
  .line.meta { color: var(--color-crush-text-muted); }
  .line.ok { color: var(--color-crush-green); font-weight: 500; }
  .line.warn { color: #eab308; }
  .cursor { color: var(--color-crush-text); animation: blink 1s step-end infinite; }
  @keyframes blink { 50% { opacity: 0; } }
</style>
