<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import * as api from '$lib/tauri';
  import { parseAnsi } from '$lib/ansi';
  import { containers, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import { services, refreshServices } from '$lib/stores/services.svelte.ts';
  import type { LogLine, DiagnosisResult, DeploymentRecord, DeploymentDetail } from '$lib/tauri';

  type Source = 'deployments' | 'containers' | 'services';
  let source = $state<Source>('deployments');

  // ── Native service logs (postgres / redis / mongo / minio …) — tailed from
  // the per-service log file the driver writes at spawn time.
  let selService = $state<{ project: string; name: string } | null>(null);
  let serviceLogText = $state('');
  let svcPoll: ReturnType<typeof setInterval> | null = null;

  // ── Deployment logs (persisted; the populated source on native Windows runs)
  let deps = $state<DeploymentRecord[]>([]);
  let loadingDeps = $state(true);
  let selDep = $state<DeploymentDetail | null>(null);
  let depTab = $state<'runtime' | 'build'>('runtime');
  let depText = $derived(selDep ? (depTab === 'build' ? selDep.build_log : selDep.runtime_log) : '');

  // ── Live container logs
  let selectedContainerId = $state<string | null>(null);
  let logLines = $state<LogLine[]>([]);
  let unlistenLogs: (() => void) | null = null;
  let autoscroll = $state(true);
  let logEl: HTMLDivElement | undefined = $state();

  // ── Shared filter + AI diagnose
  let filterLevel = $state<'all' | 'error'>('all');
  let diagnosis = $state<DiagnosisResult | null>(null);
  let diagnosing = $state(false);
  let diagnoseError = $state<string | null>(null);

  const ERR_RE = /error|exception|fatal|panic|traceback|failed|\bERR\b/i;

  let liveDisplay = $derived(
    filterLevel === 'error' ? logLines.filter(l => l.stream === 'stderr' || ERR_RE.test(l.text)) : logLines
  );
  let depDisplay = $derived.by(() => {
    const all = depText ? depText.split('\n') : [];
    return filterLevel === 'error' ? all.filter(l => ERR_RE.test(l)) : all;
  });
  let serviceDisplay = $derived.by(() => {
    const all = serviceLogText ? serviceLogText.split('\n') : [];
    return filterLevel === 'error' ? all.filter(l => ERR_RE.test(l)) : all;
  });
  let errorCount = $derived(
    source === 'containers'
      ? logLines.filter(l => l.stream === 'stderr' || ERR_RE.test(l.text)).length
      : source === 'services'
        ? (serviceLogText ? serviceLogText.split('\n').filter(l => ERR_RE.test(l)).length : 0)
        : (depText ? depText.split('\n').filter(l => ERR_RE.test(l)).length : 0)
  );

  onMount(() => { startPolling(); loadDeps(); refreshServices(); });
  onDestroy(() => { stopPolling(); unlistenLogs?.(); stopSvcPoll(); });

  async function loadDeps() {
    loadingDeps = true;
    try { deps = await api.listAllDeployments(); }
    catch { deps = []; }
    finally { loadingDeps = false; }
  }

  function setSource(s: Source) {
    source = s;
    diagnosis = null; diagnoseError = null;
  }

  function stopSvcPoll() { if (svcPoll) { clearInterval(svcPoll); svcPoll = null; } }

  async function selectDeployment(d: DeploymentRecord) {
    unlistenLogs?.(); unlistenLogs = null;
    selectedContainerId = null;
    selService = null; stopSvcPoll();
    diagnosis = null; diagnoseError = null;
    try {
      selDep = await api.getDeployment(d.project, d.id);
      depTab = (selDep.runtime_log && selDep.runtime_log.trim()) ? 'runtime' : 'build';
    } catch (e) { console.error(e); }
  }

  async function selectContainer(id: string) {
    unlistenLogs?.();
    selDep = null;
    selService = null; stopSvcPoll();
    selectedContainerId = id;
    logLines = [];
    diagnosis = null; diagnoseError = null;
    try {
      await api.subscribeLogs(id);
      unlistenLogs = await api.onLogLine(id, async (line) => {
        logLines = [...logLines.slice(-1000), line];
        if (autoscroll) { await tick(); logEl?.scrollTo({ top: logEl.scrollHeight }); }
      });
    } catch (e) { console.error('Failed to subscribe', e); }
  }

  async function selectService(project: string, name: string) {
    unlistenLogs?.(); unlistenLogs = null;
    selDep = null; selectedContainerId = null;
    selService = { project, name };
    serviceLogText = '';
    diagnosis = null; diagnoseError = null;
    await refreshServiceLog();
    stopSvcPoll();
    svcPoll = setInterval(refreshServiceLog, 1500);
  }
  async function refreshServiceLog() {
    if (!selService) return;
    try {
      serviceLogText = await api.readServiceLog(selService.project, selService.name);
      if (autoscroll) { await tick(); logEl?.scrollTo({ top: logEl.scrollHeight }); }
    } catch { /* file may not exist yet */ }
  }

  function onScroll() {
    if (!logEl) return;
    autoscroll = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight < 24;
  }

  async function diagnose() {
    let lines: string[];
    if (source === 'containers') {
      lines = logLines.filter(l => l.stream === 'stderr' || ERR_RE.test(l.text)).map(l => l.text);
    } else if (source === 'services') {
      lines = serviceLogText.split('\n').filter(l => ERR_RE.test(l));
    } else {
      lines = depText.split('\n').filter(l => ERR_RE.test(l));
    }
    if (lines.length === 0 || diagnosing) return;
    diagnosing = true; diagnosis = null; diagnoseError = null;
    try { diagnosis = await api.diagnoseLogs(lines.slice(-200)); }
    catch (e) { diagnoseError = String(e); }
    finally { diagnosing = false; }
  }

  function ago(ms: number): string {
    const s = Math.floor((Date.now() - ms) / 1000);
    if (s < 60) return `${s}s`;
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    if (s < 86400) return `${Math.floor(s / 3600)}h`;
    return `${Math.floor(s / 86400)}d`;
  }
  let hasSelection = $derived(source === 'deployments' ? !!selDep : source === 'services' ? !!selService : !!selectedContainerId);
</script>

<div class="logs-page">
  <header class="page-header">
    <h1>Logs</h1>
    <div class="src-toggle">
      <button class="src" class:active={source === 'deployments'} onclick={() => setSource('deployments')}>Deployments</button>
      <button class="src" class:active={source === 'containers'} onclick={() => setSource('containers')}>Containers</button>
      <button class="src" class:active={source === 'services'} onclick={() => setSource('services')}>Services</button>
    </div>
  </header>

  <div class="logs-layout">
    <div class="sidebar-list">
      {#if source === 'deployments'}
        <h3>Deployments</h3>
        {#if loadingDeps}
          <p class="muted sm">Loading…</p>
        {:else if deps.length === 0}
          <p class="muted sm">No deployments yet. Run a project to record one.</p>
        {:else}
          {#each deps as d}
            <button class="sidebar-item" class:active={selDep?.id === d.id} onclick={() => selectDeployment(d)}>
              <span class="sdot {d.status}"></span>
              <span class="si-main">
                <span class="si-name">{d.project}</span>
                <span class="si-sub">{#if d.framework || d.runtime}<TechIcon name={d.framework ?? d.runtime} size={11} />{/if}{ago(d.created_ms)} ago</span>
              </span>
            </button>
          {/each}
        {/if}
      {:else if source === 'containers'}
        <h3>Containers</h3>
        {#if $containers.length === 0}
          <p class="muted sm">No containers running. Native runs appear under Deployments.</p>
        {:else}
          {#each $containers as c}
            <button class="sidebar-item" class:active={selectedContainerId === c.id} onclick={() => selectContainer(c.id)}>
              <StatusBadge status={c.status} />
              <span class="si-name">{c.name}</span>
            </button>
          {/each}
        {/if}
      {:else}
        <h3>Services</h3>
        {#if $services.length === 0}
          <p class="muted sm">No native services running. Postgres, Redis, MongoDB &amp; MinIO logs appear here when running.</p>
        {:else}
          {#each $services as svc}
            <button class="sidebar-item" class:active={selService?.project === svc.project && selService?.name === svc.name} onclick={() => selectService(svc.project, svc.name)}>
              <TechIcon name={svc.kind || svc.name} size={14} />
              <span class="si-main">
                <span class="si-name">{svc.name}</span>
                <span class="si-sub">{svc.project} · :{svc.port}</span>
              </span>
            </button>
          {/each}
        {/if}
      {/if}
    </div>

    <div class="logs-main">
      {#if hasSelection}
        <div class="log-controls term-chrome">
          <div class="chrome-dots"><span class="dot red"></span><span class="dot yellow"></span><span class="dot green"></span></div>
          <div class="left-controls">
            {#if source === 'deployments' && selDep}
              <div class="filter-tabs">
                <button class="filter-tab" class:active={depTab === 'build'} onclick={() => depTab = 'build'}>Build</button>
                <button class="filter-tab" class:active={depTab === 'runtime'} onclick={() => depTab = 'runtime'}>Runtime</button>
              </div>
            {/if}
            <div class="filter-tabs">
              <button class="filter-tab" class:active={filterLevel === 'all'} onclick={() => filterLevel = 'all'}>All</button>
              <button class="filter-tab" class:active={filterLevel === 'error'} onclick={() => filterLevel = 'error'}>Errors</button>
            </div>
          </div>
          <button class="diagnose-btn" onclick={diagnose} disabled={errorCount === 0 || diagnosing} title={errorCount === 0 ? 'No errors to diagnose' : ''}>
            <Icon name="sparkles" size={13} />
            {diagnosing ? 'Diagnosing…' : 'AI Diagnose'}
            {#if errorCount > 0}<span class="err-pill">{errorCount}</span>{/if}
          </button>
        </div>

        <div class="log-output" bind:this={logEl} onscroll={onScroll}>
          {#if source === 'containers'}
            {#each liveDisplay as line}
              <div class="t-line" class:stderr={line.stream === 'stderr'}>{#each parseAnsi(line.text) as s}<span style={s.style}>{s.text}</span>{/each}</div>
            {/each}
            {#if liveDisplay.length === 0}<p class="muted" style="padding:12px;">No log lines yet</p>{/if}
          {:else if source === 'services'}
            {#each serviceDisplay as line}
              <div class="t-line" class:stderr={ERR_RE.test(line)}>{#each parseAnsi(line) as s}<span style={s.style}>{s.text}</span>{/each}</div>
            {/each}
            {#if serviceDisplay.length === 0}<p class="muted" style="padding:12px;">No log output yet for this service.</p>{/if}
          {:else}
            {#each depDisplay as line}
              <div class="t-line" class:stderr={ERR_RE.test(line)}>{#each parseAnsi(line) as s}<span style={s.style}>{s.text}</span>{/each}</div>
            {/each}
            {#if depDisplay.length === 0}<p class="muted" style="padding:12px;">No {depTab} logs for this deployment.</p>{/if}
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
              {#if diagnosis.fix}<div class="diag-fix"><span class="diag-fix-label">Fix</span> <code>{diagnosis.fix}</code></div>{/if}
            {/if}
          </div>
        {/if}
      {:else}
        <div class="select-prompt">
          <p class="muted">{source === 'deployments' ? 'Select a deployment to view its build & runtime logs' : source === 'services' ? 'Select a service to tail its logs' : 'Select a container from the sidebar'}</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .src-toggle { display: flex; gap: 2px; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 2px; }
  .src { font-size: 12px; padding: 5px 14px; border-radius: 6px; background: none; border: none; color: var(--color-crush-text-muted); cursor: pointer; }
  .src.active { background: var(--color-crush-primary); color: var(--color-crush-on-primary); }

  .logs-layout { display: flex; gap: 16px; height: calc(100vh - 120px); }
  .sidebar-list { width: 220px; flex-shrink: 0; display: flex; flex-direction: column; gap: 2px; overflow-y: auto; }
  .sidebar-list h3 { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0 0 8px; }
  .sidebar-item { display: flex; align-items: center; gap: 8px; padding: 8px 10px; background: none; border: none; border-radius: 8px; cursor: pointer; font-size: 13px; color: var(--color-crush-text); text-align: left; width: 100%; }
  .sidebar-item.active { background: rgba(255,255,255,0.08); }
  .sidebar-item:hover { background: var(--color-crush-surface); }
  .sdot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .sdot.ready, .sdot.running { background: var(--color-crush-green); }
  .sdot.failed { background: var(--color-crush-red); }
  .si-main { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .si-name { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .si-sub { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-crush-text-muted); }

  .logs-main { flex: 1; display: flex; flex-direction: column; border: 1px solid var(--color-crush-border); border-radius: 0.75rem; overflow: hidden; background: rgba(9,9,11,0.97); box-shadow: 0 10px 40px rgba(0,0,0,0.3); }
  .log-controls { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 9px 14px; border-bottom: 1px solid var(--color-crush-border); }
  .term-chrome { background: linear-gradient(180deg, rgba(34,34,44,0.9), rgba(26,26,34,0.9)); }
  .chrome-dots { display: flex; gap: 6px; flex-shrink: 0; }
  .chrome-dots .dot { width: 11px; height: 11px; border-radius: 50%; }
  .chrome-dots .dot.red { background: #ff5f56; }
  .chrome-dots .dot.yellow { background: #ffbd2e; }
  .chrome-dots .dot.green { background: #27c93f; }
  .left-controls { display: flex; gap: 10px; }
  .filter-tabs { display: flex; gap: 2px; background: rgba(0,0,0,0.25); border-radius: 6px; padding: 2px; }
  .filter-tab { font-size: 12px; padding: 4px 11px; border-radius: 5px; background: none; border: none; color: var(--color-crush-text-muted); cursor: pointer; }
  .filter-tab.active { background: var(--color-crush-dark); color: var(--color-crush-text); }
  .diagnose-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; padding: 5px 12px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.22); background: none; color: var(--color-crush-text); cursor: pointer; }
  .diagnose-btn:hover:not(:disabled) { background: rgba(255,255,255,0.08); }
  .diagnose-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .err-pill { background: var(--color-crush-red); color: white; border-radius: 9999px; font-size: 10px; padding: 0 6px; line-height: 16px; min-width: 16px; text-align: center; }

  .log-output { flex: 1; overflow-y: auto; padding: 12px 16px; font-family: var(--font-mono); font-size: 11.5px; line-height: 1.65; background: transparent; }
  .t-line { white-space: pre-wrap; word-break: break-word; color: var(--color-crush-text); }
  .t-line.stderr { color: #fca5a5; }

  .diag-card { margin: 12px; border: 1px solid rgba(255,255,255,0.16); background: rgba(255,255,255,0.04); border-radius: 0.75rem; padding: 12px 14px; }
  .diag-head { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 700; color: var(--color-crush-text); text-transform: uppercase; letter-spacing: 0.05em; }
  .diag-dismiss { margin-left: auto; background: none; border: none; color: var(--color-crush-text-muted); font-size: 16px; line-height: 1; cursor: pointer; }
  .diag-body { margin: 8px 0 0; font-size: 13px; color: var(--color-crush-text); }
  .diag-err { color: var(--color-crush-red); font-family: var(--font-mono); font-size: 12px; }
  .diag-fix { margin-top: 8px; font-size: 12px; }
  .diag-fix-label { color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.05em; font-size: 11px; margin-right: 6px; }
  .diag-fix code { font-family: var(--font-mono); background: rgba(9,9,11,0.6); border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 2px 8px; color: var(--color-crush-text); }

  .select-prompt { display: flex; align-items: center; justify-content: center; flex: 1; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }
  .muted.sm { font-size: 12px; padding: 4px 2px; }
</style>
