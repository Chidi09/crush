<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import type { ServerHealth, ServerContainer, NativeServerService, ServerContainerStat } from '$lib/tauri';

  let alias = $derived(decodeURIComponent(($page.params as Record<string, string>).alias ?? ''));
  let health = $state<ServerHealth | null>(null);
  let containers = $state<ServerContainer[]>([]);
  let services = $state<NativeServerService[]>([]);
  let stats = $state<Record<string, ServerContainerStat>>({});
  let loading = $state(true);
  let acting = $state<string | null>(null);

  // Logs panel
  let logFor = $state<ServerContainer | null>(null);
  let logText = $state('');
  let logLoading = $state(false);
  let logFollowing = $state(false);
  let logUnlisten: import('@tauri-apps/api/event').UnlistenFn | null = null;
  let logBodyEl = $state<HTMLElement | null>(null);

  $effect(() => {
    if (logText && logBodyEl && logFollowing) {
      logBodyEl.scrollTop = logBodyEl.scrollHeight;
    }
  });

  let memPct = $derived(health && health.mem_total_mb > 0 ? Math.round((health.mem_used_mb / health.mem_total_mb) * 100) : 0);

  async function load() {
    loading = true;
    try {
      health = await api.serverHealth(alias);
      if (health?.reachable) {
        if (health.has_docker) {
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
      health = { reachable: false, os: '', uptime: '', cpus: 0, mem_total_mb: 0, mem_used_mb: 0, disk_size: '', disk_used: '', disk_pct: '', has_docker: false, error: String(e) };
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
    if (!confirm(`Restart ${s.kind} service ${s.name}?`)) return;
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
  {:else if health}
    <!-- Health -->
    <div class="health-grid">
      <div class="hcard"><span class="hlabel">OS</span><span class="hval">{health.os || '—'}</span></div>
      <div class="hcard"><span class="hlabel">Uptime</span><span class="hval">{health.uptime || '—'}</span></div>
      <div class="hcard"><span class="hlabel">CPU</span><span class="hval">{health.cpus || '—'} cores</span></div>
      <div class="hcard">
        <span class="hlabel">Memory</span>
        <span class="hval">{(health.mem_used_mb/1024).toFixed(1)} / {(health.mem_total_mb/1024).toFixed(1)} GB</span>
        <div class="bar"><div class="fill" style="width:{memPct}%" class:hot={memPct>85}></div></div>
      </div>
      <div class="hcard">
        <span class="hlabel">Disk</span>
        <span class="hval">{health.disk_used} / {health.disk_size}</span>
        <div class="bar"><div class="fill" style="width:{health.disk_pct}" class:hot={parseInt(health.disk_pct)>85}></div></div>
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
                <button class="mini" disabled={acting!==null} onclick={() => restartService(s)} title="Restart">{acting===s.name ? '…' : 'restart'}</button>
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
            <div class="crow" style="grid-template-columns: 1.4fr 1.6fr 60px 80px 1fr auto;">
              <span class="cname" title={c.status}><span class="sdot" class:up={isUp(c.status)}></span>{c.name}</span>
              <span class="mono dim">{c.image}</span>
              <span class="mono dim" style="font-size: 11px;">{stats[c.name]?.cpu || '—'}</span>
              <span class="mono dim" style="font-size: 11px;">{stats[c.name]?.mem || '—'}</span>
              <span class="mono dim ports">{c.ports || '—'}</span>
              <span class="cactions">
                <button class="mini" disabled={acting!==null} onclick={() => execContainer(c)} title="Terminal">exec</button>
                <button class="mini" disabled={acting!==null} onclick={() => showLogs(c)} title="Logs">logs</button>
                <button class="mini" disabled={acting!==null} onclick={() => restart(c)} title="Restart">{acting===c.id ? '…' : 'restart'}</button>
                <button class="mini danger" disabled={acting!==null} onclick={() => stop(c)} title="Stop">stop</button>
              </span>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}

  {#if logFor}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
    <div class="logs-overlay" role="button" tabindex="0" onclick={closeLogs} onkeydown={(e)=>{if(e.key==='Escape')closeLogs();}}>
      <div class="logs-panel" role="document" onclick={(e) => e.stopPropagation()}>
        <div class="logs-head">
          <span><Icon name="logs" size={14} /> {logFor.name}</span>
          <div style="display: flex; gap: 8px;">
            <label style="font-size: 13px; display: inline-flex; align-items: center; gap: 6px; cursor: pointer;">
              <input type="checkbox" checked={logFollowing} onchange={toggleFollow} /> Follow
            </label>
            <button class="ghost-btn sm" onclick={closeLogs}><Icon name="x" size={13} /></button>
          </div>
        </div>
        <pre bind:this={logBodyEl} class="logs-body">{logLoading ? 'loading…' : logText}</pre>
      </div>
    </div>
  {/if}
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
  .unreachable { display: flex; gap: 14px; align-items: flex-start; border: 1px solid rgba(239,68,68,0.3); background: rgba(239,68,68,0.05); border-radius: 12px; padding: 16px; color: #f87171; }
  .unreachable code { font-family: var(--font-mono); }

  .health-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 26px; }
  .hcard { border: 1px solid var(--color-crush-border); background: var(--color-crush-surface); border-radius: 12px; padding: 14px; display: flex; flex-direction: column; gap: 4px; }
  .hlabel { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .hval { font-size: 14px; font-weight: 600; }
  .bar { height: 5px; border-radius: 3px; background: rgba(255,255,255,0.08); overflow: hidden; margin-top: 6px; }
  .fill { height: 100%; background: #4ade80; }
  .fill.hot { background: var(--color-crush-orange); }

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
  .logs-panel { width: 100%; max-width: 900px; max-height: 80vh; background: var(--color-crush-dark); border: 1px solid var(--color-crush-border); border-radius: 12px; display: flex; flex-direction: column; overflow: hidden; }
  .logs-head { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--color-crush-border); font-size: 13px; }
  .logs-head span { display: inline-flex; align-items: center; gap: 7px; font-weight: 600; }
  .logs-body { margin: 0; padding: 16px; overflow: auto; font-family: var(--font-mono); font-size: 12px; line-height: 1.55; white-space: pre-wrap; word-break: break-word; }
</style>
