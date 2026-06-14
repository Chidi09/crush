<script lang="ts">
  import { tick } from 'svelte';
  import { run } from '$lib/stores/run.svelte.ts';

  type Seg = { text: string; style: string };
  type Phase = 'build' | 'runtime';
  // The run store owns the event listener + raw log buffer (so it survives
  // navigation); this pane is now a pure view over `run.lines` / `run.status`.
  let status = $derived(run.status);
  let el: HTMLDivElement | undefined = $state();

  // Vercel-style split: build/setup output vs the running app's runtime output.
  // Jamming both into one stream gets jumbled — keep them as separate tabs.
  let tab = $state<Phase>('build');
  let userPickedTab = $state(false);
  type RenderLine = { segs: Seg[]; kind: 'out' | 'err' | 'meta' | 'ok' | 'warn'; phase: Phase };
  let rendered = $derived<RenderLine[]>(run.lines.map((l) => ({
    segs: l.kind === 'out' || l.kind === 'err' ? parseAnsi(l.text) : [{ text: l.text, style: '' }],
    kind: l.kind,
    phase: l.phase,
  })));
  let buildLines = $derived(rendered.filter(l => l.phase === 'build'));
  let runtimeLines = $derived(rendered.filter(l => l.phase === 'runtime'));
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

  function pickTab(t: Phase) { tab = t; userPickedTab = true; }

  // Once the app produces runtime output, jump to the Runtime tab (unless the
  // user has manually chosen a tab). Driven by the store's buffer.
  $effect(() => {
    if (!userPickedTab && runtimeLines.length > 0) tab = 'runtime';
  });

  // Auto-scroll the active tab to the bottom as new lines arrive.
  $effect(() => {
    shown.length; // track
    tick().then(() => el?.scrollTo({ top: el.scrollHeight }));
  });
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
