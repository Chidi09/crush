<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import * as api from '$lib/tauri';
  import type { DeploymentRecord } from '$lib/tauri';

  let all = $state<DeploymentRecord[]>([]);
  let loading = $state(true);

  // Filters
  let query = $state('');
  let statusFilter = $state<'all' | 'running' | 'ready' | 'failed'>('all');
  // Collapsed by default — track which projects the user has expanded.
  let expanded = $state<Set<string>>(new Set());

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

  onMount(load);
  async function load() {
    loading = true;
    try { all = await api.listAllDeployments(); }
    catch (e) { console.error(e); all = []; }
    finally { loading = false; }
  }

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
          <button class="seg-btn" class:active={statusFilter === s} onclick={() => statusFilter = s}>{s}</button>
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
        <div class="crush-card group">
          <div class="group-head" role="button" tabindex="0"
               onclick={() => toggle(g.project)}
               onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(g.project); } }}>
            <svg class="chev" class:open={isOpen} viewBox="0 0 24 24" width="14" height="14"><path d="M9 6l6 6-6 6" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>
            <span class="gh-name">{g.project}</span>
            {#if g.latest?.framework || g.latest?.runtime}
              <span class="gh-stack"><TechIcon name={g.latest.framework ?? g.latest.runtime} size={13} />{g.latest.framework ?? g.latest.runtime}</span>
            {/if}
            <!-- collapsed summary: latest status + when -->
            <span class="gh-summary">
              <span class="sdot {g.latest.status}"></span>
              <span class="dep-status">{g.latest.status}</span>
              <span class="dep-when">{ago(g.latest.created_ms)}</span>
            </span>
            <span class="gh-count">{g.deps.length}</span>
            <button class="open-btn" title="Open project" onclick={(e) => { e.stopPropagation(); openProject(g.project); }}><Icon name="play" size={11} /></button>
          </div>

          {#if isOpen}
            <div class="rows">
              {#each g.deps.slice(0, 8) as d}
                <button class="dep-row" onclick={() => openProject(g.project)}>
                  <span class="sdot {d.status}"></span>
                  <span class="dep-status">{d.status}</span>
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

  .group { padding: 0; overflow: hidden; }
  .group-head { display: flex; align-items: center; gap: 12px; width: 100%; text-align: left; background: none; border: none; padding: 12px 14px; cursor: pointer; color: var(--color-crush-text); }
  .group-head:hover { background: rgba(255,255,255,0.02); }
  .chev { color: var(--color-crush-muted); transition: transform 0.18s ease; flex-shrink: 0; }
  .chev.open { transform: rotate(90deg); }
  .gh-name { font-weight: 600; font-size: 14px; }
  .gh-stack { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); }
  .gh-summary { margin-left: auto; display: inline-flex; align-items: center; gap: 8px; font-size: 12px; color: var(--color-crush-text-muted); }
  .gh-count { font-size: 12px; color: var(--color-crush-muted); background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 9999px; min-width: 22px; text-align: center; padding: 0 7px; line-height: 18px; }
  .open-btn { display: inline-flex; align-items: center; justify-content: center; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 6px; padding: 4px 6px; cursor: pointer; }
  .open-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }

  .rows { display: flex; flex-direction: column; padding: 2px 8px 8px; border-top: 1px solid var(--color-crush-border); }
  .dep-row { display: grid; grid-template-columns: 12px 64px 1fr auto auto 80px; align-items: center; gap: 12px; width: 100%; text-align: left; background: none; border: none; padding: 9px 8px; cursor: pointer; color: var(--color-crush-text); font-size: 13px; border-radius: 6px; }
  .dep-row:hover { background: rgba(255,255,255,0.03); }
  .sdot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .sdot.ready, .sdot.running { background: var(--color-crush-green); }
  .sdot.failed { background: var(--color-crush-red); }
  .dep-status { text-transform: capitalize; }
  .dep-branch { display: inline-flex; align-items: center; gap: 4px; color: var(--color-crush-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dep-port { color: var(--color-crush-text); }
  .dep-dur, .dep-when { color: var(--color-crush-text-muted); font-size: 12px; }
  .dep-when { text-align: right; }
  .mono { font-family: var(--font-mono); }
  .more { background: none; border: none; color: var(--color-crush-text); cursor: pointer; font-size: 12px; text-align: left; padding: 8px; }
  .more:hover { text-decoration: underline; }
</style>
