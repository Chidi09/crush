<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import DonutGauge from '$lib/components/DonutGauge.svelte';
  import AreaChart from '$lib/components/AreaChart.svelte';
  import UptimeBars from '$lib/components/UptimeBars.svelte';
  import LogStream from '$lib/components/LogStream.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import JsonTree from '$lib/components/JsonTree.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import * as api from '$lib/tauri';
  import { confirmAction } from '$lib/stores/confirm.svelte.ts';
  import type { ServerHealth, ServerContainer, NativeServerService, ServerContainerStat } from '$lib/tauri';

  let alias = $derived(decodeURIComponent(($page.params as Record<string, string>).alias ?? ''));
  let health = $state<ServerHealth | null>(null);
  let containers = $state<ServerContainer[]>([]);
  let services = $state<NativeServerService[]>([]);
  let stats = $state<Record<string, ServerContainerStat>>({});
  let loading = $state(true);
  let acting = $state<string | null>(null);

  // Live history (ring buffers, sampled each poll) for the telemetry charts.
  let memHistory = $state<number[]>([]);
  let uptimeSegs = $state<{ status: 'up' | 'down' | 'degraded'; tooltip?: string }[]>([]);
  const HISTORY_CAP = 48;

  // Logs panel
  let logFor = $state<ServerContainer | null>(null);
  let logText = $state('');
  let logLoading = $state(false);
  let logFollowing = $state(false);
  let logUnlisten: import('@tauri-apps/api/event').UnlistenFn | null = null;

  // Inspect modal
  let inspectFor = $state<ServerContainer | null>(null);
  let inspectData = $state<any>(null);
  let inspectLoading = $state(false);
  let inspectErr = $state<string | null>(null);
  let inspectOpen = $state(false);

  // Destructive-confirm modal (stop)
  let confirmFor = $state<ServerContainer | null>(null);
  let confirmOpen = $state(false);

  // Right-click context menu
  let ctx = $state<{ open: boolean; x: number; y: number; c: ServerContainer | null }>({ open: false, x: 0, y: 0, c: null });

  let memPct = $derived(health && health.mem_total_mb > 0 ? Math.round((health.mem_used_mb / health.mem_total_mb) * 100) : 0);
  let diskPct = $derived(health ? (parseInt(health.disk_pct) || 0) : 0);
  let memColor = $derived(memPct > 85 ? 'var(--status-error)' : memPct > 65 ? 'var(--status-warn)' : 'var(--viz-3)');
  let diskColor = $derived(diskPct > 85 ? 'var(--status-error)' : diskPct > 65 ? 'var(--status-warn)' : 'var(--viz-5)');

  let logLines = $derived(
    (logText || '')
      .split('\n')
      .filter((l) => l.length > 0)
      .map((text, i) => ({
        id: String(i),
        text,
        severity: (/error|fail|fatal|exception/i.test(text)
          ? 'error'
          : /warn/i.test(text)
            ? 'warn'
            : 'info') as 'info' | 'warn' | 'error',
      }))
  );

  function sampleHistory(h: ServerHealth) {
    const status: 'up' | 'down' | 'degraded' = h.reachable ? 'up' : 'down';
    uptimeSegs = [...uptimeSegs, { status, tooltip: new Date().toLocaleTimeString() }].slice(-HISTORY_CAP);
    if (h.reachable && h.mem_total_mb > 0) {
      const pct = Math.round((h.mem_used_mb / h.mem_total_mb) * 100);
      memHistory = [...memHistory, pct].slice(-HISTORY_CAP);
    }
  }

  async function load() {
    loading = true;
    try {
      const h = await api.serverHealth(alias);
      health = h;
      sampleHistory(h);
      if (h?.reachable) {
        if (h.has_docker) {
          containers = await api.serverContainers(alias);
          const st = await api.serverContainerStats(alias).catch(() => []);
          const statMap: Record<string, ServerContainerStat> = {};
          for (const s of st) statMap[s.name] = s;
          stats = statMap;
        } else {
          services = await api.serverServices(alias);
        }
      } else {
        containers = [];
        services = [];
        stats = {};
      }
    } catch (e) {
      const h: ServerHealth = { reachable: false, os: '', uptime: '', cpus: 0, mem_total_mb: 0, mem_used_mb: 0, disk_size: '', disk_used: '', disk_pct: '', has_docker: false, error: String(e) };
      health = h;
      sampleHistory(h);
    } finally {
      loading = false;
    }
  }
  async function restart(c: ServerContainer) {
    acting = c.id;
    try { await api.serverContainerRestart(alias, c.id); await load(); }
    catch (e) { alert(String(e)); } finally { acting = null; }
  }
  async function stop(c: ServerContainer) {
    acting = c.id;
    try { await api.serverContainerStop(alias, c.id); await load(); }
    catch (e) { alert(String(e)); } finally { acting = null; }
  }
  async function restartService(s: NativeServerService) {
    if (!await confirmAction({ title: 'Restart service', message: `Restart ${s.kind} service ${s.name}?`, confirmText: 'Restart' })) return;
    acting = s.name;
    try { await api.serverServiceRestart(alias, s.name, s.kind); await load(); }
    catch (e) { alert(`Failed to restart: ${String(e)}`); } finally { acting = null; }
  }
  async function execContainer(c: ServerContainer) {
    try { await api.serverContainerExec(alias, c.id); }
    catch (e) { alert(`Failed to open terminal: ${String(e)}`); }
  }
  async function showLogs(c: ServerContainer) {
    if (logFor) await closeLogs();
    logFor = c; logText = ''; logLoading = true; logFollowing = false;
    try {
      let initial = await api.serverContainerLogs(alias, c.id, 300);
      logText = initial ? initial + (initial.endsWith('\n') ? '' : '\n') : '(no output)\n';
    }
    catch (e) { logText = String(e) + '\n'; } finally { logLoading = false; }
  }

  async function closeLogs() {
    if (logFollowing && logFor) {
      await api.serverContainerLogsUnfollow(alias, logFor.id).catch(() => {});
      if (logUnlisten) { logUnlisten(); logUnlisten = null; }
    }
    logFor = null;
    logFollowing = false;
    logUnlisten = null;
  }

  async function toggleFollow() {
    if (!logFor) return;
    if (logFollowing) {
      logFollowing = false;
      await api.serverContainerLogsUnfollow(alias, logFor.id);
      if (logUnlisten) { logUnlisten(); logUnlisten = null; }
    } else {
      logFollowing = true;
      logUnlisten = await api.onLogLine(`${alias}:${logFor.id}`, (line) => {
        logText += line.text + '\n';
      });
      await api.serverContainerLogsFollow(alias, logFor.id);
    }
  }
  function isUp(status: string) { return /^up/i.test(status.trim()); }

  // ── Inspect ────────────────────────────────────────────────────────────────
  async function openInspect(c: ServerContainer) {
    inspectFor = c; inspectData = null; inspectErr = null; inspectLoading = true; inspectOpen = true;
    try {
      const raw = await api.serverContainerInspect(alias, c.id);
      const parsed = JSON.parse(raw);
      inspectData = Array.isArray(parsed) ? parsed[0] : parsed;
    } catch (e) {
      inspectErr = String(e);
    } finally {
      inspectLoading = false;
    }
  }

  let inspectSummary = $derived.by(() => {
    const d = inspectData;
    if (!d) return [] as { k: string; v: string }[];
    const ports = Object.keys(d.NetworkSettings?.Ports ?? {}).join(', ');
    const mounts = (d.Mounts ?? []).map((m: any) => `${m.Source} → ${m.Destination}`);
    return [
      { k: 'Image', v: d.Config?.Image ?? '—' },
      { k: 'State', v: `${d.State?.Status ?? '—'}${d.State?.Health ? ` · ${d.State.Health.Status}` : ''}` },
      { k: 'Restart policy', v: d.HostConfig?.RestartPolicy?.Name || 'no' },
      { k: 'Ports', v: ports || '—' },
      { k: 'Networks', v: Object.keys(d.NetworkSettings?.Networks ?? {}).join(', ') || '—' },
      { k: 'Mounts', v: mounts.length ? mounts.join('\n') : '—' },
      { k: 'Created', v: d.Created ? new Date(d.Created).toLocaleString() : '—' },
    ];
  });

  // ── Right-click menu ─────────────────────────────────────────────────────────
  function openCtx(e: MouseEvent, c: ServerContainer) {
    e.preventDefault();
    ctx = { open: true, x: e.clientX, y: e.clientY, c };
  }
  function onCtxAction(action: string) {
    const c = ctx.c;
    if (!c) return;
    if (action === 'inspect') openInspect(c);
    else if (action === 'logs') showLogs(c);
    else if (action === 'exec') execContainer(c);
    else if (action === 'restart') restart(c);
    else if (action === 'stop') askStop(c);
  }

  // ── Destructive confirm ──────────────────────────────────────────────────────
  function askStop(c: ServerContainer) { confirmFor = c; confirmOpen = true; }
  async function doStop() {
    const c = confirmFor;
    confirmOpen = false;
    if (!c) return;
    await stop(c);
    confirmFor = null;
  }

  let timer: ReturnType<typeof setInterval> | null = null;
  onMount(() => { load(); timer = setInterval(load, 15000); });
  onDestroy(() => { if (timer) clearInterval(timer); });
</script>

<div class="page">
  <header class="head">
    <button class="back" onclick={() => goto('/servers')}><Icon name="branch" size={14} /> Servers</button>
    <h1><Icon name="server" size={18} /> {alias}</h1>
    <div class="spacer"></div>
    <button class="ghost-btn" onclick={load}><Icon name="refresh" size={14} /> Refresh</button>
    <button class="ghost-btn" onclick={() => api.sshConnect(alias)}><Icon name="play" size={13} /> Terminal</button>
  </header>

  {#if loading && !health}
    <p class="muted">Connecting to {alias}…</p>
  {:else if health && !health.reachable}
    <div class="unreachable">
      <Icon name="server" size={20} />
      <div>
        <strong>Can't reach {alias}</strong>
        <p class="muted">{health.error ?? 'SSH connection failed.'} — key auth is required (BatchMode); make sure you can <code>ssh {alias}</code> from a terminal.</p>
      </div>
    </div>
    {#if uptimeSegs.length > 1}
      <div class="hcard" style="margin-top:14px;">
        <span class="hlabel">Availability</span>
        <UptimeBars segments={uptimeSegs} />
      </div>
    {/if}
  {:else if health}
    <!-- Health -->
    <div class="health-grid">
      <div class="hcard"><span class="hlabel">OS</span><span class="hval">{health.os || '—'}</span></div>
      <div class="hcard"><span class="hlabel">Uptime</span><span class="hval">{health.uptime || '—'}</span></div>
      <div class="hcard"><span class="hlabel">CPU</span><span class="hval">{health.cpus || '—'} cores</span></div>
      <div class="hcard gauge-card">
        <span class="hlabel">Memory</span>
        <DonutGauge value={memPct} size={72} color={memColor} />
        <span class="gsub">{(health.mem_used_mb / 1024).toFixed(1)} / {(health.mem_total_mb / 1024).toFixed(1)} GB</span>
      </div>
      <div class="hcard gauge-card">
        <span class="hlabel">Disk</span>
        <DonutGauge value={diskPct} size={72} color={diskColor} />
        <span class="gsub">{health.disk_used} / {health.disk_size}</span>
      </div>
    </div>

    <div class="health-grid wide-grid">
      <div class="hcard span2">
        <span class="hlabel">Memory usage history</span>
        {#if memHistory.length > 1}
          <AreaChart data={memHistory} width={520} height={70} color="var(--viz-5)" />
        {:else}
          <span class="muted sm">collecting samples…</span>
        {/if}
      </div>
      <div class="hcard">
        <span class="hlabel">Availability</span>
        {#if uptimeSegs.length > 1}
          <UptimeBars segments={uptimeSegs} />
        {:else}
          <span class="muted sm">collecting…</span>
        {/if}
      </div>
    </div>

    <!-- Containers or Services -->
    {#if !health.has_docker}
      <div class="sec-head">
        <h2>Native Services</h2>
        <span class="count">{services.length}</span>
      </div>
      {#if !services.length}
        <p class="muted">Docker isn't installed on this server, and no native services (systemd/pm2) were found.</p>
      {:else}
        <div class="ctable">
          <div class="crow chead" style="grid-template-columns: 2fr 1fr 1fr auto;"><span>Name</span><span>Kind</span><span>Status</span><span></span></div>
          {#each services as s}
            <div class="crow" style="grid-template-columns: 2fr 1fr 1fr auto;">
              <span class="cname">{s.name}</span>
              <span class="mono dim">{s.kind}</span>
              <span class="cstatus">{s.status}</span>
              <span class="cactions">
                <button class="mini" disabled={acting !== null} onclick={() => restartService(s)} title="Restart">{acting === s.name ? '…' : 'restart'}</button>
              </span>
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      <div class="sec-head">
        <h2>Containers</h2>
        <span class="count">{containers.length}</span>
      </div>
      {#if !containers.length}
        <p class="muted">No containers running.</p>
      {:else}
        <div class="ctable">
          <div class="crow chead" style="grid-template-columns: 1.4fr 1.6fr 60px 80px 1fr auto;"><span>Name</span><span>Image</span><span>CPU</span><span>Mem</span><span>Ports</span><span></span></div>
          {#each containers as c (c.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="crow" style="grid-template-columns: 1.4fr 1.6fr 60px 80px 1fr auto;" oncontextmenu={(e) => openCtx(e, c)}>
              <button class="cname linkish" title="Inspect {c.name}" onclick={() => openInspect(c)}><span class="sdot" class:up={isUp(c.status)}></span>{c.name}</button>
              <span class="mono dim">{c.image}</span>
              <span class="mono dim" style="font-size: 11px;">{stats[c.name]?.cpu || '—'}</span>
              <span class="mono dim" style="font-size: 11px;">{stats[c.name]?.mem || '—'}</span>
              <span class="mono dim ports">{c.ports || '—'}</span>
              <span class="cactions">
                <button class="mini" disabled={acting !== null} onclick={() => execContainer(c)} title="Terminal">exec</button>
                <button class="mini" disabled={acting !== null} onclick={() => showLogs(c)} title="Logs">logs</button>
                <button class="mini" disabled={acting !== null} onclick={() => restart(c)} title="Restart">{acting === c.id ? '…' : 'restart'}</button>
                <button class="mini danger" disabled={acting !== null} onclick={() => askStop(c)} title="Stop">stop</button>
              </span>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}

  {#if logFor}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
    <div class="logs-overlay" role="button" tabindex="0" onclick={closeLogs} onkeydown={(e) => { if (e.key === 'Escape') closeLogs(); }}>
      <div class="logs-panel" role="document" onclick={(e) => e.stopPropagation()}>
        <div class="logs-head">
          <span><Icon name="logs" size={14} /> {logFor.name}</span>
          <div style="display: flex; gap: 8px;">
            <label style="font-size: 13px; display: inline-flex; align-items: center; gap: 6px; cursor: pointer;">
              <input type="checkbox" checked={logFollowing} onchange={toggleFollow} /> Stream
            </label>
            <button class="ghost-btn sm" onclick={closeLogs}><Icon name="x" size={13} /></button>
          </div>
        </div>
        <div class="logs-body">
          {#if logLoading}
            <p class="muted" style="padding:16px;">loading…</p>
          {:else}
            <LogStream logs={logLines} showHeader={false} />
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Right-click container menu -->
  <ContextMenu
    bind:open={ctx.open}
    x={ctx.x}
    y={ctx.y}
    items={[
      { label: 'Inspect', action: 'inspect' },
      { label: 'Logs', action: 'logs' },
      { label: 'Exec', action: 'exec' },
      { label: 'Restart', action: 'restart' },
      { label: 'Stop', action: 'stop' },
    ]}
    onaction={onCtxAction}
  />

  <!-- Inspect modal (reuses the Modal shell, JsonTree body) -->
  <Modal bind:open={inspectOpen} title={inspectFor ? `Inspect · ${inspectFor.name}` : 'Inspect'}>
    {#if inspectLoading}
      <p class="muted">Running docker inspect…</p>
    {:else if inspectErr}
      <p class="muted">{inspectErr}</p>
    {:else if inspectData}
      <div class="inspect-summary">
        {#each inspectSummary as row}
          <div class="isum-k">{row.k}</div>
          <div class="isum-v">{row.v}</div>
        {/each}
      </div>
      <div class="inspect-json">
        <JsonTree data={inspectData} expanded={false} name="inspect" />
      </div>
    {/if}
  </Modal>

  <!-- Destructive confirm modal (reuses the Modal shell) -->
  <Modal bind:open={confirmOpen} title="Stop container">
    {#if confirmFor}
      <p class="confirm-text">Stop <strong>{confirmFor.name}</strong>? In-flight requests will be dropped.</p>
      <div class="modal-actions">
        <button class="ghost-btn" onclick={() => (confirmOpen = false)}>Cancel</button>
        <button class="danger-btn" onclick={doStop}>Stop container</button>
      </div>
    {/if}
  </Modal>
</div>

<style>
  .page { padding: 20px 24px; max-width: 1100px; margin: 0 auto; }
  .head { display: flex; align-items: center; gap: 12px; margin-bottom: 18px; }
  .head h1 { font-size: 18px; font-weight: 600; margin: 0; display: inline-flex; align-items: center; gap: 8px; }
  .back { background: none; border: none; color: var(--color-crush-text-muted); cursor: pointer; display: inline-flex; align-items: center; gap: 5px; font-size: 13px; }
  .back:hover { color: var(--color-crush-text); }
  .spacer { flex: 1; }
  .ghost-btn { display: inline-flex; align-items: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.7rem; padding: 5px 11px; font-size: 13px; cursor: pointer; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn.sm { padding: 4px 8px; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }
  .muted.sm { font-size: 12px; }
  .unreachable { display: flex; gap: 14px; align-items: flex-start; border: 1px solid rgba(239,68,68,0.3); background: rgba(239,68,68,0.05); border-radius: 12px; padding: 16px; color: #f87171; }
  .unreachable code { font-family: var(--font-mono); }

  .health-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 14px; }
  .wide-grid { grid-template-columns: 2fr 1fr; margin-bottom: 26px; }
  .hcard { border: 1px solid var(--color-crush-border); background: var(--color-crush-surface); border-radius: 12px; padding: 14px; display: flex; flex-direction: column; gap: 4px; }
  .gauge-card { align-items: center; text-align: center; gap: 8px; }
  .span2 { grid-column: span 1; }
  .hlabel { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); align-self: flex-start; }
  .hval { font-size: 14px; font-weight: 600; }
  .gsub { font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }

  .sec-head { display: flex; align-items: center; gap: 9px; margin-bottom: 12px; }
  .sec-head h2 { font-size: 15px; font-weight: 600; margin: 0; }
  .count { font-size: 12px; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 9999px; padding: 1px 9px; color: var(--color-crush-text-muted); }

  .ctable { display: flex; flex-direction: column; border: 1px solid var(--color-crush-border); border-radius: 10px; overflow: hidden; }
  .crow { display: grid; grid-template-columns: 1.4fr 1.6fr 1.4fr 1.2fr auto; gap: 10px; align-items: center; padding: 9px 12px; border-bottom: 1px solid var(--color-crush-border); font-size: 13px; }
  .crow:last-child { border-bottom: none; }
  .chead { background: var(--color-crush-surface); color: var(--color-crush-text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; }
  .cname { display: inline-flex; align-items: center; gap: 7px; font-weight: 500; }
  .sdot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-crush-text-muted); flex-shrink: 0; }
  .sdot.up { background: #4ade80; box-shadow: 0 0 6px rgba(74,222,128,0.6); }
  .mono { font-family: var(--font-mono); font-size: 12px; }
  .dim { color: var(--color-crush-text-muted); }
  .ports { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cactions { display: inline-flex; gap: 6px; justify-content: flex-end; }
  .mini { font-size: 11.5px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 3px 9px; cursor: pointer; }
  .mini:hover:not(:disabled) { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .mini.danger:hover:not(:disabled) { color: var(--color-crush-red); border-color: rgba(239,68,68,0.5); }
  .mini:disabled { opacity: 0.5; cursor: default; }

  .logs-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; padding: 40px; }
  .logs-panel { width: 100%; max-width: 900px; height: 80vh; background: var(--color-crush-dark); border: 1px solid var(--color-crush-border); border-radius: 12px; display: flex; flex-direction: column; overflow: hidden; }
  .logs-head { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--color-crush-border); font-size: 13px; }
  .logs-head span { display: inline-flex; align-items: center; gap: 7px; font-weight: 600; }
  .logs-body { flex: 1; overflow: hidden; padding: 12px; display: flex; }
  .logs-body :global(.crush-card) { width: 100%; }

  .cname.linkish { background: none; border: none; padding: 0; cursor: pointer; color: var(--color-crush-text); font: inherit; }
  .cname.linkish:hover { color: var(--color-crush-orange); }

  .inspect-summary { display: grid; grid-template-columns: auto 1fr; gap: 6px 14px; margin-bottom: 14px; font-size: 13px; }
  .isum-k { color: var(--color-crush-text-muted); font-size: 12px; }
  .isum-v { font-family: var(--font-mono); font-size: 12px; white-space: pre-wrap; word-break: break-all; }
  .inspect-json { max-height: 46vh; overflow: auto; border-top: 1px solid var(--color-crush-border); padding-top: 12px; }

  .confirm-text { font-size: 14px; margin: 0 0 16px; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; }
  .danger-btn { background: var(--color-crush-red, #ef4444); color: white; border: none; border-radius: 0.6rem; padding: 7px 14px; font-size: 13px; cursor: pointer; }
  .danger-btn:hover { filter: brightness(1.1); }
</style>
