<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import { containers, loading, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import type { ContainerSummary, LogLine } from '$lib/tauri';

  let selected: ContainerSummary | null = $state(null);
  let searchQuery = $state('');
  let logLines = $state<LogLine[]>([]);
  let unlistenLogs: (() => void) | null = null;
  let unlistenReplay: (() => void) | null = null;
  let logEl: HTMLDivElement | undefined = $state();
  let stopping = $state(false);
  let copied = $state(false);

  // Live-preview state
  const PREVIEW_PATHS = [
    { label: 'App', path: '/' },
    { label: 'API docs', path: '/docs' },
    { label: 'Swagger', path: '/swagger' },
    { label: 'ReDoc', path: '/redoc' },
  ];
  let previewPath = $state('/');
  let previewKey = $state(0); // bump to force iframe reload

  let primaryPort = $derived((selected as any)?.ports[0]?.host_port ?? null);
  let previewBase = $derived(primaryPort ? `http://localhost:${primaryPort}` : null);
  let previewUrl = $derived(previewBase ? `${previewBase}${previewPath}` : null);

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
    selected = c;
    logLines = [];
    copied = false;
    previewPath = '/';
    previewKey++;
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

  function closeDetail() {
    if (selected) api.unsubscribeLogs(selected.id).catch(() => {});
    unlistenLogs?.();
    unlistenLogs = null;
    unlistenReplay?.();
    unlistenReplay = null;
    selected = null;
  }

  async function stopContainer() {
    if (!selected || stopping) return;
    stopping = true;
    try { await api.stopContainer(selected.id); }
    catch (e) { console.error('Stop failed', e); }
    finally { stopping = false; }
  }

  function copyId() {
    if (!selected) return;
    navigator.clipboard.writeText(selected.id).then(() => {
      copied = true;
      setTimeout(() => copied = false, 1500);
    });
  }
  function setPath(p: string) { if (p !== previewPath) { previewPath = p; previewKey++; } }
  function reloadPreview() { previewKey++; }
  function visit() { if (previewUrl) api.openUrl(previewUrl).catch(console.error); }
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
    <div class="empty-box">
      <Icon name="containers" size={26} />
      <p class="empty-title">No containers running</p>
      <p class="muted">On Windows, <code>crush run</code> launches dev servers as native processes (not containers), so they won't appear here — open the run's <strong>Preview</strong> tab on the Dashboard to see the live app. Containers show up here when an image is run in a container.</p>
    </div>
  {:else}
    <div class="table">
      <div class="table-header">
        <span>NAME</span>
        <span>IMAGE</span>
        <span>STATUS</span>
        <span class="r">PORTS</span>
        <span class="r">UPTIME</span>
      </div>
      {#each filtered as c}
        <button class="table-row" onclick={() => selectContainer(c)}>
          <span class="cell-name">{c.name}</span>
          <span class="cell-image">{c.image}</span>
          <span><StatusBadge status={c.status} /></span>
          <span class="r cell-port">{c.ports.length ? c.ports.map(p => `:${p.host_port}`).join(' ') : '—'}</span>
          <span class="r cell-up">{c.uptime_secs ? formatDuration(c.uptime_secs) : '—'}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if selected}
  <button class="overlay" onclick={closeDetail} aria-label="Close"></button>
  <div class="detail animate-slide-up">
    <!-- Header -->
    <div class="detail-head">
      <div class="dh-left">
        <StatusBadge status={selected.status} />
        <div class="dh-id">
          <h2>{selected.name}</h2>
          <span class="dh-image">{selected.image}</span>
        </div>
      </div>
      <div class="dh-actions">
        {#if previewUrl}<button class="btn primary" onclick={visit}><Icon name="play" size={12} fill /> Visit</button>{/if}
        <button class="btn danger" onclick={stopContainer} disabled={stopping || selected.status !== 'running'}><Icon name="stop" size={12} fill /> {stopping ? 'Stopping…' : 'Stop'}</button>
        <button class="btn" onclick={copyId}><Icon name={copied ? 'check' : 'copy'} size={12} /> {copied ? 'Copied' : 'Copy ID'}</button>
        <button class="close-btn" onclick={closeDetail} aria-label="Close">×</button>
      </div>
    </div>

    <div class="detail-body">
      <!-- Left: live preview -->
      <div class="preview-col">
        {#if previewBase}
          <div class="preview-bar">
            <div class="seg">
              {#each PREVIEW_PATHS as t}
                <button class="seg-btn" class:active={previewPath === t.path} onclick={() => setPath(t.path)}>{t.label}</button>
              {/each}
            </div>
            <div class="preview-url" title={previewUrl}>{previewUrl}</div>
            <button class="icon-btn" onclick={reloadPreview} title="Reload preview"><Icon name="refresh" size={13} /></button>
            <button class="icon-btn" onclick={visit} title="Open in browser"><Icon name="folder" size={13} /></button>
          </div>
          <div class="preview-frame">
            {#if selected.status === 'running'}
              {#key previewKey}
                <iframe src={previewUrl} title="Container preview" sandbox="allow-scripts allow-same-origin allow-forms allow-popups"></iframe>
              {/key}
            {:else}
              <div class="preview-empty"><Icon name="containers" size={28} /><p>Container is {selected.status}. Start it to see a live preview.</p></div>
            {/if}
          </div>
        {:else}
          <div class="preview-frame"><div class="preview-empty"><Icon name="containers" size={28} /><p>No published port — nothing to preview.</p></div></div>
        {/if}
      </div>

      <!-- Right: meta -->
      <div class="meta-col">
        <div class="meta-group">
          <span class="meta-k">Status</span>
          <span class="meta-v"><StatusBadge status={selected.status} /></span>
        </div>
        <div class="meta-group">
          <span class="meta-k">Uptime</span>
          <span class="meta-v">{selected.uptime_secs ? formatDuration(selected.uptime_secs) : '—'}</span>
        </div>
        <div class="meta-group">
          <span class="meta-k">Environment</span>
          <span class="meta-v">local</span>
        </div>
        <div class="meta-group">
          <span class="meta-k">Endpoints</span>
          <span class="meta-v">
            {#if selected.ports.length}
              {#each selected.ports as p}
                <button class="endpoint" onclick={() => api.openUrl(`http://localhost:${p.host_port}`)}>localhost:{p.host_port} <span class="ep-arrow">↗</span></button>
              {/each}
            {:else}—{/if}
          </span>
        </div>
        <div class="meta-group">
          <span class="meta-k">Image</span>
          <span class="meta-v mono small">{selected.image}</span>
        </div>
        <div class="meta-group">
          <span class="meta-k">Container ID</span>
          <span class="meta-v mono small">{selected.id.substring(0, 16)}</span>
        </div>
      </div>
    </div>

    <!-- Logs -->
    <div class="logs-section">
      <div class="logs-head"><Icon name="logs" size={13} /> Runtime logs</div>
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
  .empty-box { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 48px 24px; color: var(--color-crush-text-muted); border: 1px dashed var(--color-crush-border); border-radius: 1rem; }
  .empty-box .empty-title { font-size: 15px; font-weight: 600; color: var(--color-crush-text); margin: 4px 0 0; }
  .empty-box .muted { max-width: 460px; line-height: 1.6; }
  .empty-box code { font-family: var(--font-mono); font-size: 12px; background: var(--color-crush-surface); padding: 1px 5px; border-radius: 4px; color: var(--color-crush-text); }

  .table { border: 1px solid var(--color-crush-border); border-radius: 1rem; overflow: hidden; }
  .table-header { display: grid; grid-template-columns: 2fr 2fr 1fr 1fr 1fr; gap: 12px; padding: 12px 16px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); background: var(--color-crush-surface); border-bottom: 1px solid var(--color-crush-border); }
  .table-row { display: grid; grid-template-columns: 2fr 2fr 1fr 1fr 1fr; gap: 12px; padding: 12px 16px; font-size: 13px; background: none; border: none; border-bottom: 1px solid var(--color-crush-border); color: var(--color-crush-text); cursor: pointer; text-align: left; width: 100%; align-items: center; }
  .table-row:last-child { border-bottom: none; }
  .table-row:hover { background: rgba(255, 255, 255, 0.03); }
  .cell-name { font-weight: 500; }
  .cell-image { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cell-up, .cell-port { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text-muted); }
  .cell-port { color: var(--color-crush-text); }
  .r { text-align: right; }

  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); backdrop-filter: blur(2px); z-index: 40; border: none; padding: 0; cursor: default; }
  .detail { position: fixed; top: 3vh; left: 50%; transform: translateX(-50%); width: min(1040px, 94vw); max-height: 94vh; background: var(--color-crush-dark); border: 1px solid var(--color-crush-border); border-radius: 1rem; z-index: 50; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 24px 80px rgba(0,0,0,0.5); }

  .detail-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 16px 20px; border-bottom: 1px solid var(--color-crush-border); }
  .dh-left { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .dh-id { min-width: 0; }
  .dh-id h2 { font-size: 17px; font-weight: 600; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dh-image { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text-muted); }
  .dh-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 6px 12px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .btn:hover:not(:disabled) { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .btn.primary { color: var(--color-crush-on-primary); background: var(--color-crush-primary); border-color: var(--color-crush-primary); }
  .btn.primary:hover { background: var(--color-crush-primary-hover); border-color: var(--color-crush-primary-hover); }
  .btn.danger { color: var(--color-crush-red); border-color: rgba(239,68,68,0.3); }
  .btn.danger:hover:not(:disabled) { background: rgba(239,68,68,0.1); }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .close-btn { background: none; border: none; color: var(--color-crush-text-muted); font-size: 22px; line-height: 1; cursor: pointer; padding: 0 4px; }
  .close-btn:hover { color: var(--color-crush-text); }

  .detail-body { display: grid; grid-template-columns: 1fr 280px; gap: 0; border-bottom: 1px solid var(--color-crush-border); min-height: 0; }
  .preview-col { display: flex; flex-direction: column; border-right: 1px solid var(--color-crush-border); min-width: 0; }
  .preview-bar { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-bottom: 1px solid var(--color-crush-border); background: var(--color-crush-surface); }
  .seg { display: flex; gap: 2px; background: rgba(0,0,0,0.25); border-radius: 7px; padding: 2px; flex-shrink: 0; }
  .seg-btn { font-size: 11px; color: var(--color-crush-text-muted); background: none; border: none; border-radius: 5px; padding: 4px 9px; cursor: pointer; }
  .seg-btn.active { background: var(--color-crush-dark); color: var(--color-crush-text); }
  .preview-url { flex: 1; font-family: var(--font-mono); font-size: 11px; color: var(--color-crush-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .icon-btn { display: inline-flex; background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; color: var(--color-crush-text-muted); padding: 5px; cursor: pointer; }
  .icon-btn:hover { color: var(--color-crush-text); }
  .preview-frame { position: relative; height: 380px; background: #0a0a0c; }
  .preview-frame iframe { width: 100%; height: 100%; border: none; background: white; }
  .preview-empty { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: var(--color-crush-text-muted); font-size: 13px; text-align: center; padding: 20px; }

  .meta-col { display: flex; flex-direction: column; padding: 16px 18px; gap: 16px; overflow-y: auto; }
  .meta-group { display: flex; flex-direction: column; gap: 4px; }
  .meta-k { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .meta-v { font-size: 13px; }
  .meta-v.mono { font-family: var(--font-mono); }
  .meta-v.small { font-size: 11.5px; word-break: break-all; }
  .endpoint { display: inline-flex; align-items: center; gap: 5px; font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-text); background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.18); border-radius: 6px; padding: 3px 8px; margin: 0 6px 6px 0; cursor: pointer; }
  .endpoint:hover { background: rgba(255,255,255,0.12); }
  .ep-arrow { opacity: 0.7; }

  .logs-section { display: flex; flex-direction: column; min-height: 0; max-height: 240px; }
  .logs-head { display: flex; align-items: center; gap: 6px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); padding: 10px 18px 6px; }
  .logs-panel { flex: 1; min-height: 80px; overflow-y: auto; padding: 8px 16px; margin: 0 14px 14px; }
  .dlog-line { color: var(--color-crush-text); padding: 1px 0; white-space: pre-wrap; word-break: break-all; font-size: 12px; }
  .dlog-line.stderr { color: #fca5a5; }
</style>
