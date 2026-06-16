<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import DeployWizard from '$lib/components/DeployWizard.svelte';
  import * as api from '$lib/tauri';
  import type { DeploymentRecord, CloudDeployment } from '$lib/tauri';
  import { confirmAction } from '$lib/stores/confirm.svelte.ts';

  let all = $state<DeploymentRecord[]>([]);
  let cloud = $state<Record<string, CloudDeployment>>({});
  let loading = $state(true);
  // Per-project real logo (data URL from disk) and live-deployment status.
  let icons = $state<Record<string, string>>({});
  let liveStatus = $state<Record<string, { ok: boolean; checking: boolean }>>({});
  let liveTimer: ReturnType<typeof setInterval> | null = null;

  // Normalize project names so cloud + local records match (crush deploy uses
  // the Crushfile/dir name; runs may differ by case/separators).
  function projKey(p: string): string { return p.toLowerCase().replace(/[\s_-]+/g, ''); }
  function cloudFor(project: string): CloudDeployment | null { return cloud[projKey(project)] ?? null; }

  // Filters
  let query = $state('');
  let statusFilter = $state<'all' | 'running' | 'ready' | 'failed'>('all');
  let expanded = $state<Set<string>>(new Set());
  let defaultBranches = $state<Record<string, string>>({});

  type Group = { project: string; deps: DeploymentRecord[]; latest: DeploymentRecord };

  let groups = $derived.by<Group[]>(() => {
    const q = query.trim().toLowerCase();
    const map = new Map<string, DeploymentRecord[]>();
    for (const d of all) {
      if (statusFilter !== 'all' && d.status !== statusFilter) continue;
      if (q && !d.project.toLowerCase().includes(q)) continue;
      if (!map.has(d.project)) map.set(d.project, []);
      map.get(d.project)!.push(d);
    }
    const out: Group[] = [];
    for (const [project, deps] of map) {
      const sorted = [...deps].sort((a, b) => b.created_ms - a.created_ms);
      out.push({ project, deps: sorted, latest: sorted[0] });
    }
    return out.sort((a, b) => b.latest.created_ms - a.latest.created_ms);
  });

  onMount(() => {
    load();
    // Keep live-deployment status fresh without hammering the network.
    liveTimer = setInterval(refreshLiveness, 30000);
    return () => { if (liveTimer) clearInterval(liveTimer); };
  });

  async function load() {
    loading = true;
    try {
      all = await api.listAllDeployments();
      const list = await api.listCloudDeployments().catch(() => []);
      const map: Record<string, CloudDeployment> = {};
      for (const c of list) map[projKey(c.project)] = c;
      cloud = map;

      try {
        const stored = localStorage.getItem('crush:branches');
        if (stored) defaultBranches = JSON.parse(stored);
      } catch (e) {}

      for (const d of all) {
        if (!defaultBranches[d.project]) {
          defaultBranches[d.project] = 'main';
        }
      }

      loadIcons();
      refreshLiveness();
    }
    catch (e) { console.error(e); all = []; }
    finally { loading = false; }
  }

  // Resolve each project's real logo from disk (favicon/logo), keyed by project name.
  async function loadIcons() {
    const seen = new Set<string>();
    for (const d of all) {
      if (seen.has(d.project) || !d.project_path) continue;
      seen.add(d.project);
      if (icons[d.project] !== undefined) continue;
      api.findProjectIcon(d.project_path)
        .then((url) => { if (url) icons[d.project] = url; })
        .catch(() => {});
    }
  }

  // Probe each recorded cloud deployment to learn if it's actually live right now.
  async function refreshLiveness() {
    const entries = Object.values(cloud).filter((c) => c.url);
    for (const c of entries) {
      const key = projKey(c.project);
      liveStatus[key] = { ok: liveStatus[key]?.ok ?? false, checking: true };
      api.probeDeployment(c.url)
        .then((r) => { liveStatus[key] = { ok: r.ok, checking: false }; })
        .catch(() => { liveStatus[key] = { ok: false, checking: false }; });
    }
  }

  function liveFor(project: string) { return liveStatus[projKey(project)]; }

  function saveBranch(project: string) {
    localStorage.setItem('crush:branches', JSON.stringify(defaultBranches));
  }

  function openUrl(u: string) { if (u) api.openUrl(u).catch(console.error); }

  function toggle(project: string) {
    const next = new Set(expanded);
    next.has(project) ? next.delete(project) : next.add(project);
    expanded = next;
  }
  function expandAll() { expanded = new Set(groups.map(g => g.project)); }
  function collapseAll() { expanded = new Set(); }

  function ago(ms: number): string {
    const s = Math.floor((Date.now() - ms) / 1000);
    if (s < 60) return `${s}s ago`;
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    return `${Math.floor(s / 86400)}d ago`;
  }
  function dur(ms: number): string { return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(0)}s`; }
  function openProject(p: string) { goto(`/projects/${encodeURIComponent(p)}`); }

  let showDeploy = $state(false);
  let deployStack = $state<{ name: string; port: number; runtime: string | null; framework: string | null } | null>(null);
  let deployPath = $state<string | null>(null);

  function openDeploy(g: Group) {
    deployPath = g.latest.project_path;
    deployStack = {
      name: g.project,
      port: g.latest.port ?? 8080,
      runtime: g.latest.runtime ?? null,
      framework: g.latest.framework ?? null,
    };
    showDeploy = true;
  }

  function runProject(g: Group) {
    if (g.latest.project_path) {
      localStorage.setItem('crush:lastProject', g.latest.project_path);
      goto('/dashboard');
    }
  }

  async function stopDeploy(g: Group) {
    if (!await confirmAction({ title: 'Stop deployment', message: `Stop deployment for ${g.project}?`, confirmText: 'Stop', danger: true })) return;
    try {
      await api.runCapture(g.latest.project_path, 'crush', ['deploy', '--stop'], {});
      alert(`Deployment for ${g.project} stopped.`);
      await load();
    } catch (e) {
      alert(`Stop failed: ${String(e)}`);
    }
  }

  async function deleteDeploy(g: Group) {
    if (!await confirmAction({ title: 'Delete deployment', message: `Delete deployment for ${g.project}? This will remove the server and state.`, confirmText: 'Delete', danger: true })) return;
    try {
      await api.runCapture(g.latest.project_path, 'crush', ['deploy', '--destroy'], {});
      alert(`Deployment for ${g.project} deleted.`);
      await load();
    } catch (e) {
      alert(`Delete failed: ${String(e)}`);
    }
  }

  // These records are a historical archive of past `crush run`s, not live
  // processes — a run is stored as "running" the moment it starts and only
  // flips to "ready"/"failed" when it ends, so an abandoned run stays
  // "running" forever. Show past-tense labels so the list doesn't imply the
  // project is live right now.
  function statusLabel(status: string): string {
    switch (status) {
      case 'running': return 'Ran';
      case 'ready': return 'Ready';
      case 'failed': return 'Failed';
      case 'all': return 'All';
      default: return status;
    }
  }

  const STATUSES = ['all', 'running', 'ready', 'failed'] as const;
</script>

<div class="page">
  <header class="ph">
    <h1>Deployments</h1>
    <div class="ph-actions">
      <span class="total">{all.length} total</span>
      <button class="ghost-btn" onclick={load} title="Refresh"><Icon name="refresh" size={14} /></button>
    </div>
  </header>

  {#if loading}
    <p class="muted">Loading…</p>
  {:else if all.length === 0}
    <div class="empty-box">
      <Icon name="rocket" size={26} />
      <p class="empty-title">No deployments yet</p>
      <p class="muted">Every <code>crush run</code> is recorded here as a deployment — with its build &amp; runtime logs and a cached preview. Run a project from the Dashboard to create the first one.</p>
    </div>
  {:else}
    <!-- Filter bar -->
    <div class="filters">
      <div class="search">
        <Icon name="logs" size={14} />
        <input type="text" placeholder="Filter projects…" bind:value={query} />
        {#if query}<button class="clear" onclick={() => query = ''}>&times;</button>{/if}
      </div>
      <div class="seg">
        {#each STATUSES as s}
          <button class="seg-btn" class:active={statusFilter === s} onclick={() => statusFilter = s}>{statusLabel(s)}</button>
        {/each}
      </div>
      <div class="bulk">
        <button class="ghost-btn xs" onclick={expandAll}>Expand all</button>
        <button class="ghost-btn xs" onclick={collapseAll}>Collapse all</button>
      </div>
    </div>

    {#if groups.length === 0}
      <p class="muted">No deployments match the current filter.</p>
    {:else}
      {#each groups as g (g.project)}
        {@const isOpen = expanded.has(g.project)}
        {@const cd = cloudFor(g.project)}
        <div class="crush-card group">
          <div class="group-main">
            <div class="group-info" role="button" tabindex="0" onclick={() => openProject(g.project)} onkeydown={(e) => { if(e.key === 'Enter') openProject(g.project); }}>
              <div class="project-icon-wrapper">
                <!-- Base layer: the stack logo, only ever seen when no real
                     project icon is available. -->
                <div class="pi-fallback"><TechIcon name={g.latest.framework ?? g.latest.runtime} size={20} /></div>
                {#if icons[g.project]}
                  <!-- svelte-ignore a11y_missing_attribute -->
                  <img src={icons[g.project]} class="pi-image" />
                {:else if g.latest.port}
                  <!-- svelte-ignore a11y_missing_attribute -->
                  <img src={`http://localhost:${g.latest.port}/favicon.ico`} class="pi-image" onerror={(e) => { (e.currentTarget as HTMLElement).style.display = 'none'; }} />
                {/if}
              </div>
              <div class="project-meta">
                <div class="gh-name">{g.project}</div>
                {#if g.latest.framework || g.latest.runtime}
                  <div class="gh-stack"><TechIcon name={g.latest.framework ?? g.latest.runtime} size={11} /> {g.latest.framework ?? g.latest.runtime}</div>
                {/if}
              </div>
            </div>

            <div class="group-branch">
              <Icon name="branch" size={12} />
              <select class="branch-select" bind:value={defaultBranches[g.project]} onchange={() => saveBranch(g.project)}>
                <option value="main">main</option>
                {#if g.latest.branch && g.latest.branch !== 'main'}
                  <option value={g.latest.branch}>{g.latest.branch}</option>
                {/if}
              </select>
            </div>

            <div class="group-platform">
              {#if cd}
                {@const live = liveFor(g.project)}
                <span class="live-chip" class:is-down={live && !live.checking && !live.ok} title={`Deployed to ${cd.provider}`}>
                  <span class="rocket-glow"><Icon name="rocket" size={13} /></span>
                  <TechIcon name={cd.provider} size={13} />
                  <span class="live-provider">{cd.provider}</span>
                </span>
                <span class="health" title={
                  live?.checking ? 'Checking…'
                  : live?.ok ? 'Live — host is responding'
                  : live ? 'Offline — host did not respond'
                  : 'Status unknown'
                }>
                  <span class="health-dot" class:up={live?.ok} class:down={live && !live.checking && !live.ok} class:checking={live?.checking}></span>
                  <span class="health-text">{live?.checking ? 'checking' : live?.ok ? 'live' : live ? 'offline' : '—'}</span>
                </span>
                {#if cd.url}
                  <button class="live-url" title={`Open ${cd.url}`} onclick={(e) => { e.stopPropagation(); openUrl(cd.url); }}>
                    <Icon name="globe" size={12} /> {cd.domain ?? cd.url.replace(/^https?:\/\//, '')}
                  </button>
                {/if}
              {:else}
                <span class="muted" style="font-size: 11px;">Not deployed</span>
              {/if}
            </div>

            <div class="group-status">
              <span class="sdot {g.latest.status}"></span>
              <div class="status-col">
                <span class="dep-status">{statusLabel(g.latest.status)}</span>
                <span class="dep-when">{ago(g.latest.created_ms)}</span>
              </div>
            </div>

            <div class="group-actions">
              <button class="ghost-btn" onclick={() => openProject(g.project)} title="View logs">Logs</button>
              {#if cd}
                <button class="ghost-btn" onclick={() => stopDeploy(g)} title="Stop Deployment">Stop</button>
                <button class="ghost-btn text-red" onclick={() => deleteDeploy(g)} title="Delete Deployment">Delete</button>
              {/if}
              <button class="ghost-btn" onclick={() => openDeploy(g)} title="Deploy to cloud">Deploy</button>
              <button class="btn primary" onclick={() => runProject(g)}>Run</button>
            </div>
          </div>
          
          <button class="history-toggle" onclick={() => toggle(g.project)} aria-expanded={isOpen}>
             <svg class="chev" class:open={isOpen} viewBox="0 0 24 24" width="12" height="12"><path d="M9 6l6 6-6 6" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
             {g.deps.length} {g.deps.length === 1 ? 'deployment' : 'deployments'} history
          </button>

          {#if isOpen}
            <div class="rows">
              {#each g.deps.slice(0, 8) as d}
                <button class="dep-row" onclick={() => openProject(g.project)}>
                  <span class="sdot {d.status}"></span>
                  <span class="dep-status">{statusLabel(d.status)}</span>
                  {#if d.branch}<span class="dep-branch mono"><Icon name="branch" size={11} /> {d.branch}{#if d.commit_short} · {d.commit_short}{/if}</span>{:else}<span></span>{/if}
                  <span class="dep-port mono">{d.port ? `:${d.port}` : ''}</span>
                  <span class="dep-dur mono">{d.duration_ms ? dur(d.duration_ms) : ''}</span>
                  <span class="dep-when">{ago(d.created_ms)}</span>
                </button>
              {/each}
              {#if g.deps.length > 8}
                <button class="more" onclick={() => openProject(g.project)}>View all {g.deps.length} →</button>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  {/if}
</div>

{#if showDeploy && deployStack && deployPath}
  <DeployWizard path={deployPath} stack={deployStack} onClose={() => showDeploy = false} />
{/if}

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  .ph { display: flex; align-items: center; justify-content: space-between; }
  .ph h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .ph-actions { display: flex; align-items: center; gap: 12px; }
  .total { font-size: 13px; color: var(--color-crush-text-muted); }
  .ghost-btn { display: inline-flex; align-items: center; justify-content: center; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 8px; padding: 7px 10px; cursor: pointer; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn.xs { padding: 4px 10px; font-size: 12px; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }

  .filters { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .search { position: relative; display: flex; align-items: center; gap: 8px; flex: 1; min-width: 200px; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 7px 12px; color: var(--color-crush-text-muted); }
  .search input { flex: 1; background: none; border: none; outline: none; color: var(--color-crush-text); font-size: 13px; }
  .search .clear { background: none; border: none; color: var(--color-crush-text-muted); font-size: 16px; cursor: pointer; line-height: 1; }
  .seg { display: flex; gap: 2px; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 8px; padding: 2px; }
  .seg-btn { font-size: 12px; text-transform: capitalize; padding: 5px 12px; border-radius: 6px; background: none; border: none; color: var(--color-crush-text-muted); cursor: pointer; }
  .seg-btn.active { background: var(--color-crush-primary); color: var(--color-crush-on-primary); }
  .bulk { display: flex; gap: 6px; }

  .empty-box { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 48px 24px; color: var(--color-crush-text-muted); border: 1px dashed var(--color-crush-border); border-radius: 0.75rem; }
  .empty-box .empty-title { font-size: 15px; font-weight: 600; color: var(--color-crush-text); margin: 4px 0 0; }
  .empty-box .muted { max-width: 460px; line-height: 1.6; }
  .empty-box code { font-family: var(--font-mono); font-size: 12px; background: var(--color-crush-surface); padding: 1px 5px; border-radius: 4px; color: var(--color-crush-text); }

  .group { padding: 0; overflow: hidden; display: flex; flex-direction: column; }
  .group-main { display: grid; grid-template-columns: 2fr 1fr 1.5fr 1fr auto; align-items: center; gap: 16px; padding: 16px 20px; }
  
  .group-info { display: flex; align-items: center; gap: 14px; cursor: pointer; }
  .group-info:hover .gh-name { text-decoration: underline; }
  .project-icon-wrapper { position: relative; width: 42px; height: 42px; border-radius: 10px; overflow: hidden; background: rgba(255,255,255,0.06); flex-shrink: 0; border: 1px solid var(--color-crush-border); }
  .pi-fallback { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; }
  .pi-image { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; background: var(--color-crush-surface); z-index: 2; }
  .project-meta { display: flex; flex-direction: column; gap: 4px; }
  .gh-name { font-weight: 600; font-size: 15px; color: var(--color-crush-text); }
  .gh-stack { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); }

  .group-branch { display: flex; align-items: center; gap: 6px; color: var(--color-crush-text-muted); }
  .branch-select { background: rgba(255,255,255,0.03); border: 1px solid var(--color-crush-border); color: var(--color-crush-text); padding: 4px 8px; border-radius: 6px; font-size: 12px; font-family: var(--font-mono); outline: none; cursor: pointer; }
  .branch-select:hover { border-color: var(--color-crush-muted); }

  .group-platform { display: flex; flex-direction: column; align-items: flex-start; gap: 6px; }

  .group-status { display: flex; align-items: center; gap: 10px; }
  .status-col { display: flex; flex-direction: column; gap: 2px; }

  .group-actions { display: flex; align-items: center; gap: 8px; justify-content: flex-end; }
  .btn { display: inline-flex; align-items: center; justify-content: center; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 8px; padding: 7px 12px; font-size: 13px; cursor: pointer; }
  .btn.primary { background: var(--color-crush-primary); border-color: var(--color-crush-primary); color: var(--color-crush-on-primary); font-weight: 500; }
  .btn.primary:hover { background: var(--color-crush-primary-hover); }

  .history-toggle { display: flex; align-items: center; gap: 8px; background: rgba(0,0,0,0.15); border: none; border-top: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); font-size: 12px; padding: 8px 20px; cursor: pointer; width: 100%; text-align: left; }
  .history-toggle:hover { color: var(--color-crush-text); background: rgba(255,255,255,0.02); }
  .chev { color: var(--color-crush-muted); transition: transform 0.18s ease; flex-shrink: 0; }
  .chev.open { transform: rotate(90deg); }

  /* Live cloud deployment: glowing orange rocket + platform icon + name */
  .live-chip { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--color-crush-orange); background: rgba(224,85,64,0.08); border: 1px solid rgba(224,85,64,0.28); border-radius: 9999px; padding: 2px 10px; text-transform: capitalize; }
  .live-provider { font-weight: 600; }
  .rocket-glow { display: inline-flex; color: var(--color-crush-orange); filter: drop-shadow(0 0 5px rgba(224,85,64,0.85)); animation: rocket-pulse 1.8s ease-in-out infinite; }
  @keyframes rocket-pulse {
    0%, 100% { filter: drop-shadow(0 0 3px rgba(224,85,64,0.6)); transform: translateY(0); }
    50% { filter: drop-shadow(0 0 8px rgba(224,85,64,1)); transform: translateY(-1px); }
  }
  @media (prefers-reduced-motion: reduce) { .rocket-glow { animation: none; } }
  .live-url { display: inline-flex; align-items: center; gap: 5px; font-size: 11.5px; font-family: var(--font-mono); color: var(--color-crush-orange); background: none; border: 1px solid rgba(224,85,64,0.28); border-radius: 7px; padding: 2px 8px; cursor: pointer; }
  .live-url:hover { background: rgba(224,85,64,0.14); border-color: rgba(224,85,64,0.5); }
  /* When a recorded deployment fails its liveness probe, drop the "live" glow. */
  .live-chip.is-down { color: var(--color-crush-text-muted); background: rgba(127,127,140,0.08); border-color: rgba(127,127,140,0.28); }
  .live-chip.is-down .rocket-glow { color: var(--color-crush-text-muted); filter: none; animation: none; }
  .health { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-crush-text-muted); }
  .health-text { text-transform: uppercase; letter-spacing: 0.03em; }
  .health-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-crush-muted); flex-shrink: 0; }
  .health-dot.up { background: var(--color-crush-green); box-shadow: 0 0 6px rgba(74,222,128,0.6); }
  .health-dot.down { background: var(--color-crush-red); box-shadow: 0 0 6px rgba(239,68,68,0.5); }
  .health-dot.checking { background: var(--color-crush-orange); animation: health-pulse 1s ease-in-out infinite; }
  @keyframes health-pulse { 0%,100% { opacity: 0.4; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .health-dot.checking { animation: none; } }
  .dep-status { text-transform: capitalize; font-size: 13px; color: var(--color-crush-text); }
  .dep-when { font-size: 11.5px; color: var(--color-crush-muted); }

  .rows { display: flex; flex-direction: column; padding: 4px 12px 12px; background: rgba(0,0,0,0.1); }
  .dep-row { display: grid; grid-template-columns: 12px 64px 1fr auto auto 80px; align-items: center; gap: 12px; width: 100%; text-align: left; background: none; border: none; padding: 9px 8px; cursor: pointer; color: var(--color-crush-text); font-size: 13px; border-radius: 6px; }
  .dep-row:hover { background: rgba(255,255,255,0.03); }
  .sdot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .sdot.ready, .sdot.running { background: var(--color-crush-green); }
  .sdot.failed { background: var(--color-crush-red); }
  .dep-status { text-transform: capitalize; }
  .dep-branch { display: inline-flex; align-items: center; gap: 4px; color: var(--color-crush-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dep-port { color: var(--color-crush-text); }
  .dep-dur, .dep-row .dep-when { color: var(--color-crush-text-muted); font-size: 12px; }
  .dep-row .dep-when { text-align: right; }
  .mono { font-family: var(--font-mono); }
  .more { background: none; border: none; color: var(--color-crush-text); cursor: pointer; font-size: 12px; text-align: left; padding: 8px; }
  .more:hover { text-decoration: underline; }
</style>
