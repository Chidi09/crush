<script lang="ts">
  import { onMount, tick } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import CopyField from '$lib/components/CopyField.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import { services, refreshServices } from '$lib/stores/services.svelte.ts';
  import * as api from '$lib/tauri';

  let grouped = $derived.by(() => {
    const map = new Map<string, typeof $services>();
    for (const s of $services) {
      if (!map.has(s.project)) map.set(s.project, []);
      map.get(s.project)!.push(s);
    }
    return map;
  });

  onMount(refreshServices);

  async function stopService(name: string, project: string) {
    try {
      await api.stopNativeService(name, project);
      await refreshServices();
    } catch (e) {
      console.error('Failed to stop service', e);
    }
  }

  // ── Start a service on demand (no project required) ──────────────────────
  const STARTABLE = [
    { kind: 'postgres', label: 'PostgreSQL', tech: 'postgres' },
    { kind: 'redis',    label: 'Redis',      tech: 'redis' },
    { kind: 'mongodb',  label: 'MongoDB',    tech: 'mongodb' },
    { kind: 'minio',    label: 'MinIO',      tech: 'minio' },
  ];
  let starting = $state<string | null>(null);
  let startErr = $state<string | null>(null);
  async function startService(kind: string) {
    starting = kind; startErr = null;
    try {
      await api.startNativeService(kind);
      await refreshServices();
    } catch (e) {
      startErr = String(e);
    } finally {
      starting = null;
    }
  }

  // ── Inspector (tables / connections / keys) ──────────────────────────────
  let openInspect = $state<string | null>(null);
  let pg = $state<api.PgInspect | null>(null);
  let rd = $state<api.RedisInspect | null>(null);
  let mo = $state<api.MongoInspect | null>(null);
  let mi = $state<api.MinioInspect | null>(null);
  let inspLoading = $state(false);
  let inspErr = $state<string | null>(null);

  type Svc = { project: string; name: string; kind: string; port: number };
  function svcKey(s: Svc) { return `${s.project}/${s.name}`; }
  function canInspect(kind: string) {
    return kind === 'postgres' || kind.startsWith('redis') || kind === 'mongodb' || kind === 'minio';
  }

  // ── Logs ──────────────────────────────────────────────────────────────────
  // Every native service redirects stdout+stderr to a log file; surface it here
  // so any service (not just the inspectable ones) is debuggable from the GUI.
  let openLog = $state<string | null>(null);
  let logText = $state('');
  let logLoading = $state(false);
  let logErr = $state<string | null>(null);
  let logEl: HTMLPreElement | undefined = $state();

  async function toggleLog(svc: Svc) {
    const key = svcKey(svc);
    if (openLog === key) { openLog = null; return; }
    openLog = key; logText = ''; logErr = null; logLoading = true;
    try {
      logText = await api.readServiceLog(svc.project, svc.name, 800);
      if (!logText) logText = '(log is empty — the service may have just started)';
    } catch (e) { logErr = String(e); }
    finally {
      logLoading = false;
      await tick();
      logEl?.scrollTo({ top: logEl.scrollHeight });
    }
  }
  async function refreshLog(svc: Svc) {
    logLoading = true;
    try { logText = await api.readServiceLog(svc.project, svc.name, 800) || '(log is empty)'; }
    catch (e) { logErr = String(e); }
    finally { logLoading = false; await tick(); logEl?.scrollTo({ top: logEl.scrollHeight }); }
  }

  async function toggleInspect(svc: Svc) {
    const key = svcKey(svc);
    if (openInspect === key) { openInspect = null; return; }
    openInspect = key; pg = null; rd = null; mo = null; mi = null; inspErr = null; inspLoading = true;
    try {
      if (svc.kind === 'postgres') pg = await api.inspectPostgres(svc.port);
      else if (svc.kind === 'mongodb') mo = await api.inspectMongo(svc.port);
      else if (svc.kind === 'minio') mi = await api.inspectMinio(svc.port);
      else rd = await api.inspectRedis(svc.port);
    } catch (e) { inspErr = String(e); }
    finally { inspLoading = false; }
  }
  function fmtBytes(b: number): string {
    if (!b) return '0 B';
    if (b < 1024) return `${b} B`;
    if (b < 1048576) return `${(b / 1024).toFixed(0)} KB`;
    if (b < 1073741824) return `${(b / 1048576).toFixed(1)} MB`;
    return `${(b / 1073741824).toFixed(2)} GB`;
  }
  async function loadDb(svc: Svc, db: string) {
    inspLoading = true; inspErr = null;
    try { pg = await api.inspectPostgres(svc.port, undefined, undefined, db); }
    catch (e) { inspErr = String(e); }
    finally { inspLoading = false; }
  }
  function ttlLabel(t: number) { return t < 0 ? '∞' : `${t}s`; }
</script>

<div class="page">
  <header class="page-header">
    <h1>Native Services</h1>
  </header>

  <!-- Start any native service on demand — no project needed. -->
  <div class="start-bar">
    <span class="start-label">Start a service</span>
    {#each STARTABLE as s}
      <button class="start-btn" disabled={starting !== null} onclick={() => startService(s.kind)}>
        <TechIcon name={s.tech} size={15} />
        {starting === s.kind ? 'starting…' : s.label}
      </button>
    {/each}
  </div>
  {#if startErr}<div class="start-err">{startErr}</div>{/if}

  {#if $services.length === 0}
    <EmptyState title="No services running" description="Pick a service above to spin one up — Postgres, Redis, MongoDB or MinIO, running natively with no project or Docker." />
  {:else}
    {#each [...grouped.entries()] as [project, svcs]}
      <div class="service-group">
        <h2 class="group-title">{project}</h2>
        {#each svcs as svc}
          <div class="crush-card svc-card">
            <div class="svc-header">
              <StatusBadge status="running" />
              <TechIcon name={svc.kind || svc.name} size={18} />
              <span class="svc-name">{svc.name}</span>
              <span class="svc-kind">{svc.kind}</span>
            </div>
            <div class="svc-details">
              Port <span class="mono">{svc.port}</span>
              · Data <span class="mono">{svc.data_dir}</span>
            </div>
            {#if svc.connection_string}
              <div class="svc-conn">
                <span class="conn-label">Connection string</span>
                <CopyField value={svc.connection_string} />
              </div>
            {/if}
            {#if svc.console_url}
              <div class="svc-conn">
                <span class="conn-label">Console</span>
                <a href={svc.console_url} target="_blank" onclick={(e) => { e.preventDefault(); api.openUrl(svc.console_url!); }} class="console-link">
                  Open Console ({svc.console_url})
                </a>
              </div>
            {/if}
            <div class="svc-actions">
              {#if canInspect(svc.kind)}
                <button class="insp-btn" onclick={() => toggleInspect(svc)}>
                  <Icon name="activity" size={12} /> {openInspect === svcKey(svc) ? 'Hide' : 'Inspect'}
                </button>
              {/if}
              <button class="insp-btn" onclick={() => toggleLog(svc)}>
                <Icon name="logs" size={12} /> {openLog === svcKey(svc) ? 'Hide logs' : 'Logs'}
              </button>
              <button class="stop-btn" onclick={() => stopService(svc.name, svc.project)}><Icon name="stop" size={12} fill /> Stop</button>
            </div>

            {#if openLog === svcKey(svc)}
              <div class="insp-panel">
                <div class="log-head">
                  <span class="insp-h">Service log</span>
                  <button class="log-refresh" onclick={() => refreshLog(svc)} disabled={logLoading}>
                    <Icon name="refresh" size={11} /> Refresh
                  </button>
                </div>
                {#if logLoading && !logText}
                  <p class="muted sm">Loading…</p>
                {:else if logErr}
                  <p class="insp-err">Couldn’t read log: {logErr}</p>
                {:else}
                  <pre class="log-body" bind:this={logEl}>{logText}</pre>
                {/if}
              </div>
            {/if}

            {#if openInspect === svcKey(svc)}
              <div class="insp-panel">
                {#if inspLoading}
                  <p class="muted sm">Inspecting…</p>
                {:else if inspErr}
                  <p class="insp-err">Couldn’t connect: {inspErr}</p>
                {:else if pg}
                  <div class="insp-meta">{pg.version.split(' on ')[0]}</div>
                  <div class="insp-sec">
                    <span class="insp-h">Databases</span>
                    <div class="db-chips">
                      {#each pg.databases as db}
                        <button class="db-chip" class:active={db === pg.current_db} onclick={() => loadDb(svc, db)}>{db}</button>
                      {/each}
                    </div>
                  </div>
                  <div class="insp-sec">
                    <span class="insp-h">Tables in <span class="mono">{pg.current_db}</span> ({pg.tables.length})</span>
                    {#if pg.tables.length === 0}<p class="muted sm">No user tables.</p>{:else}
                      <div class="rows">
                        {#each pg.tables as t}
                          <div class="irow"><span class="mono">{t.schema}.{t.name}</span><span class="irow-r mono">{t.rows.toLocaleString()} rows</span></div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                  <div class="insp-sec">
                    <span class="insp-h">Connections ({pg.connections.length})</span>
                    <div class="rows">
                      {#each pg.connections as c}
                        <div class="irow"><span class="cdot {c.state}"></span><span class="mono">{c.user}@{c.db}</span><span class="irow-r dim sm">{c.state}{c.query ? ` · ${c.query}` : ''}</span></div>
                      {/each}
                    </div>
                  </div>
                {:else if rd}
                  <div class="insp-meta">{rd.total.toLocaleString()} keys total · showing {rd.keys.length}</div>
                  {#if rd.keys.length === 0}<p class="muted sm">Keyspace is empty.</p>{:else}
                    <div class="rows">
                      {#each rd.keys as k}
                        <div class="irow"><span class="mono">{k.key}</span><span class="irow-r"><span class="rkind">{k.kind}</span><span class="dim sm mono">TTL {ttlLabel(k.ttl)}</span></span></div>
                      {/each}
                    </div>
                  {/if}
                {:else if mo}
                  {#if mo.databases.length === 0}<p class="muted sm">No databases.</p>{/if}
                  {#each mo.databases as db}
                    <div class="insp-sec">
                      <span class="insp-h"><span class="mono">{db.name}</span> · {db.collections.length} collection{db.collections.length === 1 ? '' : 's'}</span>
                      {#if db.collections.length}
                        <div class="rows">
                          {#each db.collections as c}
                            <div class="irow"><span class="mono">{c.name}</span><span class="irow-r mono">{c.count.toLocaleString()} docs</span></div>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  {/each}
                {:else if mi}
                  <div class="insp-meta">{mi.buckets.length} bucket{mi.buckets.length === 1 ? '' : 's'}</div>
                  {#if mi.buckets.length === 0}<p class="muted sm">No buckets yet.</p>{:else}
                    <div class="rows">
                      {#each mi.buckets as b}
                        <div class="irow"><Icon name="images" size={12} /><span class="mono">{b.name}</span><span class="irow-r mono">{b.objects.toLocaleString()} obj · {fmtBytes(b.size)}</span></div>
                      {/each}
                    </div>
                  {/if}
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  {/if}
</div>

<style>
  .page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }

  .start-bar { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-bottom: 18px; padding: 12px 14px; border: 1px solid var(--color-crush-border); border-radius: 12px; background: var(--color-crush-surface); }
  .start-label { font-size: 12px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); font-weight: 600; margin-right: 4px; }
  .start-btn { display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--color-crush-text); background: rgba(99,102,241,0.06); border: 1px solid rgba(99,102,241,0.2); border-radius: 8px; padding: 6px 12px; cursor: pointer; transition: background 0.15s, border-color 0.15s; }
  .start-btn:hover:not(:disabled) { background: rgba(99,102,241,0.14); border-color: rgba(99,102,241,0.45); }
  .start-btn:disabled { opacity: 0.5; cursor: default; }
  .start-err { color: var(--color-crush-red); font-size: 13px; margin: -8px 0 16px; }

  .service-group { margin-bottom: 24px; }
  .group-title { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0 0 12px; }

  .svc-card { padding: 20px; margin-bottom: 12px; }
  .svc-header { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
  .svc-name { font-size: 16px; font-weight: 600; flex: 1; }
  .svc-kind { font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }

  .svc-details { font-size: 13px; color: var(--color-crush-text-muted); margin-bottom: 12px; }
  .mono { font-family: var(--font-mono); }

  .svc-conn { margin-bottom: 12px; }
  .conn-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); display: block; margin-bottom: 4px; }

  .stop-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: #ef4444; background: none; border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; padding: 6px 16px; cursor: pointer; }
  .stop-btn:hover { background: rgba(239,68,68,0.1); }

  .console-link { display: inline-flex; align-items: center; font-size: 13px; color: #6366f1; text-decoration: none; font-weight: 500; }
  .console-link:hover { text-decoration: underline; }

  .svc-actions { display: flex; gap: 8px; }
  .insp-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 6px 16px; cursor: pointer; }
  .insp-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }

  .insp-panel { margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--color-crush-border); display: flex; flex-direction: column; gap: 14px; }
  .insp-meta { font-size: 11.5px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }
  .insp-err { font-size: 12.5px; color: var(--color-crush-red); }
  .insp-sec { display: flex; flex-direction: column; gap: 7px; }
  .insp-h { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .db-chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .db-chip { font-size: 12px; font-family: var(--font-mono); padding: 3px 10px; border-radius: 9999px; border: 1px solid var(--color-crush-border); background: var(--color-crush-surface); color: var(--color-crush-text-muted); cursor: pointer; }
  .db-chip.active { color: var(--color-crush-text); border-color: var(--color-crush-muted); background: rgba(255,255,255,0.06); }
  .rows { display: flex; flex-direction: column; max-height: 220px; overflow-y: auto; border: 1px solid var(--color-crush-border); border-radius: 8px; }
  .irow { display: flex; align-items: center; gap: 8px; padding: 6px 10px; font-size: 12.5px; border-bottom: 1px solid rgba(42,42,53,0.4); }
  .irow:last-child { border-bottom: none; }
  .irow-r { margin-left: auto; display: inline-flex; align-items: center; gap: 8px; color: var(--color-crush-text-muted); }
  .dim { color: var(--color-crush-text-muted); }
  .sm { font-size: 11.5px; }
  .cdot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-crush-muted); flex-shrink: 0; }
  .cdot.active { background: var(--color-crush-green); }
  .cdot.idle { background: var(--color-crush-muted); }
  .rkind { font-size: 10px; text-transform: uppercase; letter-spacing: 0.04em; padding: 1px 7px; border-radius: 9999px; border: 1px solid var(--color-crush-border); color: var(--color-crush-text); }
  .muted { color: var(--color-crush-text-muted); }

  .log-head { display: flex; align-items: center; justify-content: space-between; }
  .log-refresh { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 4px 10px; cursor: pointer; }
  .log-refresh:hover:not(:disabled) { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .log-refresh:disabled { opacity: 0.5; cursor: default; }
  .log-body { margin: 0; padding: 12px 14px; max-height: 320px; overflow: auto; background: rgba(9,9,11,0.97); border: 1px solid var(--color-crush-border); border-radius: 8px; font-family: var(--font-mono); font-size: 11.5px; line-height: 1.6; color: var(--color-crush-text); white-space: pre-wrap; word-break: break-word; }
</style>
