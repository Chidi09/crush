<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import DeployWizard from '$lib/components/DeployWizard.svelte';
  import * as api from '$lib/tauri';
  import { toast } from '$lib/stores/toast.svelte.ts';
  import type { DeploymentRecord, DeploymentDetail, GitInfo, BranchInfo } from '$lib/tauri';

  let project = $derived($page.params.project);

  let list = $state<DeploymentRecord[]>([]);
  let loading = $state(true);
  let selected = $state<DeploymentDetail | null>(null);
  let logTab = $state<'build' | 'runtime'>('build');

  let git = $state<GitInfo | null>(null);
  let branches = $state<BranchInfo[]>([]);
  let fetchingBranches = $state(false);
  let resolvedPath = $state<string | null>(null);

  $effect(() => {
    const p = project;
    loading = true;
    selected = null;
    api.listDeployments(p).then(async (d) => {
      list = d;
      const path = d[0]?.project_path;
      if (path) {
        resolvedPath = path;
        git = await api.gitInfo(path).catch(() => null);
        loadBranches(false);
      }
    }).catch(() => list = []).finally(() => loading = false);
  });

  async function loadBranches(fetch: boolean) {
    if (!resolvedPath) return;
    fetchingBranches = true;
    try {
      branches = await api.gitBranches(resolvedPath, fetch);
    } catch (e) {
      console.error(e);
    } finally {
      fetchingBranches = false;
    }
  }

  async function previewBranch(branchName: string) {
    if (!resolvedPath) return;
    try {
      const worktreePath = await api.previewBranch(resolvedPath, branchName);
      // Hand off to the dashboard so IT owns the run (sets activeRunId →
      // shows the live preview + terminal). Starting the run here would
      // orphan it from the UI.
      localStorage.setItem('crush:lastProject', worktreePath);
      sessionStorage.setItem('crush:autorun', worktreePath);
      goto('/dashboard');
    } catch (e) { console.error(e); }
  }

  async function open(id: string) {
    try {
      selected = await api.getDeployment(project, id);
      logTab = (selected.runtime_log && !selected.build_log) ? 'runtime' : 'build';
    } catch (e) { console.error(e); }
  }
  async function visit(d: DeploymentDetail) {
    if (d.port) await api.openUrl(`http://localhost:${d.port}`).catch(() => {});
  }
  function runThis() {
    const path = list[0]?.project_path;
    if (path) { localStorage.setItem('crush:lastProject', path); goto('/dashboard'); }
  }

  // Deploy wizard (eject-to-provider → run official CLI)
  let showDeploy = $state(false);
  let deployStack = $state<{ name: string; port: number; runtime: string | null; framework: string | null } | null>(null);
  async function openDeploy() {
    if (!resolvedPath) return;
    let port = list[0]?.port ?? 8080;
    let runtime: string | null = list[0]?.runtime ?? null;
    let framework: string | null = list[0]?.framework ?? null;
    try {
      const p = await api.detectProject(resolvedPath);
      if (p.port) port = p.port;
      runtime = p.runtime; framework = p.framework;
    } catch { /* fall back to deployment-record values */ }
    deployStack = { name: project, port, runtime, framework };
    showDeploy = true;
  }

  // Eject: generate standard Docker artifacts (the "leave Crush anytime" path).
  let ejecting = $state(false);
  let ejectMsg = $state<string | null>(null);
  let ejectErr = $state(false);
  async function doEject(force = false) {
    if (!resolvedPath || ejecting) return;
    ejecting = true; ejectMsg = null; ejectErr = false;
    try {
      await api.ejectProject(resolvedPath, force);
      ejectMsg = 'Wrote Dockerfile + docker-compose.yml to the project root.';
      toast('Ejected — Dockerfile + docker-compose.yml', 'success');
    } catch (e) {
      const msg = String((e as any)?.message ?? e);
      if (!force && /already exists/i.test(msg)) {
        ejecting = false;
        if (confirm('Dockerfile / docker-compose.yml already exist. Overwrite them?')) {
          return doEject(true);
        }
        return;
      }
      ejectErr = true;
      ejectMsg = `Eject failed: ${msg}`;
    } finally {
      ejecting = false;
    }
  }
  async function remove(id: string, e: Event) {
    e.stopPropagation();
    await api.deleteDeployment(project, id).catch(() => {});
    list = list.filter(d => d.id !== id);
    if (selected?.id === id) selected = null;
  }

  function ago(ms: number): string {
    const s = Math.floor((Date.now() - ms) / 1000);
    if (s < 60) return `${s}s ago`;
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    return `${Math.floor(s / 86400)}d ago`;
  }
  function dur(ms: number): string { return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(0)}s`; }
</script>

<div class="page">
  <header class="ph">
    <div class="bc">
      <a href="/dashboard" class="crumb">Projects</a>
      <span class="sep">/</span>
      <span class="cur">{project}</span>
    </div>
    <div class="ph-actions">
      {#if resolvedPath}<button class="btn" onclick={() => doEject(false)} disabled={ejecting} title="Generate Dockerfile + docker-compose.yml"><Icon name="box" size={12} /> {ejecting ? 'Ejecting…' : 'Eject'}</button>{/if}
      {#if resolvedPath}<button class="btn" onclick={openDeploy} title="Deploy to a cloud provider"><Icon name="rocket" size={12} /> Deploy</button>{/if}
      {#if list.length && list[0].project_path}<button class="btn primary" onclick={runThis}>Run <Icon name="play" size={12} fill /></button>{/if}
    </div>
  </header>

  {#if ejectMsg}
    <div class="eject-note" class:err={ejectErr}>
      <Icon name={ejectErr ? 'stop' : 'check'} size={13} /> {ejectMsg}
    </div>
  {/if}

  {#if selected}
    <button class="link back" onclick={() => selected = null}>← All deployments</button>
    <div class="crush-card detail">
      <div class="d-top">
        <div class="preview">
          {#if selected.screenshot}
            <img src={selected.screenshot} alt="Cached preview" />
          {:else}
            <div class="no-shot"><Icon name="images" size={26} /><p>No cached preview</p></div>
          {/if}
        </div>
        <div class="d-meta">
          <div class="mg"><span class="mk">Status</span><span class="mv"><span class="sdot {selected.status}"></span>{selected.status}</span></div>
          {#if selected.port}<div class="mg"><span class="mk">URL</span><button class="link" onclick={() => visit(selected!)}>localhost:{selected.port} ↗</button></div>{/if}
          <div class="mg"><span class="mk">Duration</span><span class="mv mono">{selected.duration_ms ? dur(selected.duration_ms) : '—'}</span></div>
          <div class="mg"><span class="mk">Created</span><span class="mv">{ago(selected.created_ms)}</span></div>
          {#if selected.framework || selected.runtime}
            <div class="mg"><span class="mk">Stack</span><span class="mv stack">{#if selected.runtime}<span class="pill"><TechIcon name={selected.runtime} size={12} />{selected.runtime}</span>{/if}{#if selected.framework}<span class="pill"><TechIcon name={selected.framework} size={12} />{selected.framework}</span>{/if}</span></div>
          {/if}
          <div class="mg"><span class="mk">Source</span><span class="mv mono path">{selected.project_path}</span></div>
          {#if selected.branch}
            <div class="mg"><span class="mk">Branch / Commit</span><span class="mv mono"><Icon name="branch" size={12} /> {selected.branch} · {selected.commit_short}</span></div>
          {/if}
        </div>
      </div>
      <div class="logs">
        <div class="log-tabs">
          <button class="lt" class:active={logTab === 'build'} onclick={() => logTab = 'build'}>Build logs</button>
          <button class="lt" class:active={logTab === 'runtime'} onclick={() => logTab = 'runtime'}>Runtime logs</button>
        </div>
        <pre class="log-body">{(logTab === 'build' ? selected.build_log : selected.runtime_log) || `No ${logTab} logs.`}</pre>
      </div>
    </div>
  {:else}
    <div class="crush-card">
      <div class="lh"><h2>Deployments</h2><span class="count">{list.length}</span></div>
      {#if loading}
        <p class="muted">Loading…</p>
      {:else if list.length === 0}
        <p class="muted">No deployments recorded yet. Run this project from the dashboard to create one.</p>
      {:else}
        <div class="list">
          {#each list as d}
            <div class="dep-row clickable" role="button" tabindex="0" onclick={() => open(d.id)} onkeydown={(e) => { if (e.key === 'Enter') open(d.id); }}>
              <span class="sdot {d.status}"></span>
              <span class="dep-status">{d.status}</span>
              <span class="dep-stack">{#if d.framework || d.runtime}<TechIcon name={d.framework ?? d.runtime} size={13} />{/if}{d.framework ?? d.runtime ?? '—'}</span>
              <span class="dep-port mono">{d.port ? `:${d.port}` : ''}</span>
              <span class="dep-dur mono">{d.duration_ms ? dur(d.duration_ms) : ''}</span>
              <span class="dep-when">{ago(d.created_ms)}</span>
              {#if d.has_screenshot}<Icon name="images" size={12} />{:else}<span class="ico-sp"></span>{/if}
              <button class="del" onclick={(e) => remove(d.id, e)} title="Delete" aria-label="Delete">×</button>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Git Source -->
    {#if git && git.is_repo}
      <div class="crush-card mt-4">
        <div class="git-source-header">
          {#if git.parsed_github}
            <button class="ghost-btn sm" onclick={() => api.openUrl(git?.remote_url || '')} title="Open GitHub">
              <Icon name="github" size={13} fill /> {git.parsed_github.owner}/{git.parsed_github.repo}
            </button>
          {/if}
          <div class="chip accent branch-chip"><Icon name="branch" size={13} /> {git.branch}</div>
          <div class="chips">
            {#if git.dirty_count > 0}<span class="chip">dirty: {git.dirty_count}</span>{/if}
            {#if git.ahead && git.ahead > 0}<span class="chip">↑ {git.ahead}</span>{/if}
            {#if git.behind && git.behind > 0}<span class="chip">↓ {git.behind}</span>{/if}
          </div>
        </div>
        {#if git.head}
          <div class="git-commit mt-2">
            <span class="strong">{git.head.message}</span>
            <span class="dim sm">— {git.head.author} · {git.head.committed_rel}</span>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Branches Panel -->
    {#if resolvedPath && git?.is_repo}
      <div class="crush-card mt-4">
        <div class="lh">
          <h2>Branches</h2>
          <span class="count">{branches.length}</span>
          <button class="btn sm ml-auto" onclick={() => loadBranches(true)} disabled={fetchingBranches}>{fetchingBranches ? 'Fetching...' : 'Refresh'}</button>
        </div>
        <div class="list">
          {#each branches as b}
            <div class="dep-row branch-row">
              <Icon name="branch" size={14} />
              <span class="strong">{b.name}</span>
              {#if b.is_remote && !b.is_current}<span class="pill">remote</span>{/if}
              <span class="dim truncate">{b.message}</span>
              <span class="dim sm">{b.author}</span>
              <span class="dim sm mono text-right">{b.committed_rel}</span>
              <button class="btn primary sm ml-auto" onclick={() => previewBranch(b.name)}>Preview</button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

{#if showDeploy && deployStack && resolvedPath}
  <DeployWizard path={resolvedPath} stack={deployStack} onClose={() => showDeploy = false} />
{/if}

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  .ph { display: flex; align-items: center; justify-content: space-between; }
  .bc { display: flex; align-items: center; gap: 8px; font-size: 15px; }
  .crumb { color: var(--color-crush-text-muted); text-decoration: none; }
  .crumb:hover { color: var(--color-crush-text); }
  .sep { color: var(--color-crush-muted); }
  .cur { font-weight: 600; }
  .btn { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 6px 14px; cursor: pointer; }
  .btn:hover:not(:disabled) { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .btn:disabled { opacity: 0.5; cursor: default; }
  .btn.primary { color: var(--color-crush-on-primary); background: var(--color-crush-primary); border-color: var(--color-crush-primary); }
  .btn.primary:hover { background: var(--color-crush-primary-hover); border-color: var(--color-crush-primary-hover); }
  .eject-note { display: flex; align-items: center; gap: 8px; font-size: 13px; padding: 10px 14px; border-radius: 8px; background: rgba(16,185,129,0.1); border: 1px solid rgba(16,185,129,0.25); color: var(--color-crush-green); }
  .eject-note.err { background: rgba(239,68,68,0.1); border-color: rgba(239,68,68,0.25); color: var(--color-crush-red); }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; padding: 16px; }
  .link { background: none; border: none; color: var(--color-crush-text); cursor: pointer; font-size: 13px; padding: 0; }
  .link:hover { text-decoration: underline; }
  .back { width: max-content; }

  .crush-card { padding: 16px 18px; }
  .lh { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
  .lh h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0; }
  .count { font-size: 11px; color: var(--color-crush-muted); border: 1px solid var(--color-crush-border); border-radius: 9999px; padding: 0 8px; line-height: 18px; }

  .list { display: flex; flex-direction: column; }
  .dep-row { display: grid; grid-template-columns: 12px 70px 1fr auto auto 80px 14px 18px; align-items: center; gap: 12px; width: 100%; text-align: left; background: none; border: none; border-bottom: 1px solid rgba(42,42,53,0.4); padding: 11px 8px; cursor: pointer; color: var(--color-crush-text); font-size: 13px; }
  .dep-row:last-child { border-bottom: none; }
  .dep-row:hover { background: rgba(255,255,255,0.02); }
  .sdot { width: 8px; height: 8px; border-radius: 50%; }
  .sdot.ready, .sdot.running { background: var(--color-crush-green); }
  .sdot.failed { background: var(--color-crush-red); }
  .dep-status { text-transform: capitalize; }
  .dep-stack { display: inline-flex; align-items: center; gap: 6px; color: var(--color-crush-text-muted); }
  .dep-port { color: var(--color-crush-text); }
  .dep-dur, .dep-when { color: var(--color-crush-text-muted); font-size: 12px; }
  .dep-when { text-align: right; }
  .ico-sp { width: 12px; }
  .del { background: none; border: none; color: var(--color-crush-muted); font-size: 16px; cursor: pointer; line-height: 1; }
  .del:hover { color: var(--color-crush-red); }
  .mono { font-family: var(--font-mono); }

  .detail { display: flex; flex-direction: column; gap: 16px; }
  .d-top { display: grid; grid-template-columns: 1.4fr 1fr; gap: 16px; }
  .preview { height: 260px; background: #0a0a0c; border: 1px solid var(--color-crush-border); border-radius: 8px; overflow: hidden; }
  .preview img { width: 100%; height: 100%; object-fit: cover; object-position: top; display: block; }
  .no-shot { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--color-crush-text-muted); font-size: 13px; }
  .d-meta { display: flex; flex-direction: column; gap: 14px; }
  .mg { display: flex; flex-direction: column; gap: 4px; }
  .mk { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .mv { font-size: 13px; display: inline-flex; align-items: center; gap: 6px; }
  .mv.path { color: var(--color-crush-muted); font-size: 11.5px; word-break: break-all; }
  .stack { display: flex; flex-wrap: wrap; gap: 6px; }
  .pill { display: inline-flex; align-items: center; gap: 5px; font-size: 12px; padding: 2px 9px; border-radius: 9999px; border: 1px solid var(--color-crush-border); }

  .logs { border: 1px solid var(--color-crush-border); border-radius: 8px; overflow: hidden; }
  .log-tabs { display: flex; gap: 2px; padding: 8px; border-bottom: 1px solid var(--color-crush-border); background: var(--color-crush-surface); }
  .lt { font-size: 12px; color: var(--color-crush-text-muted); background: none; border: none; border-radius: 6px; padding: 4px 12px; cursor: pointer; }
  .lt.active { background: var(--color-crush-dark); color: var(--color-crush-text); }
  .log-body { margin: 0; padding: 12px 14px; font-family: var(--font-mono); font-size: 11.5px; line-height: 1.6; color: var(--color-crush-text); white-space: pre-wrap; word-break: break-word; max-height: 360px; overflow-y: auto; background: rgba(9,9,11,0.6); }

  .mt-4 { margin-top: 16px; }
  .mt-2 { margin-top: 8px; }
  .ml-auto { margin-left: auto; }
  .text-right { text-align: right; }
  .truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .git-source-header { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .git-commit { display: flex; align-items: baseline; gap: 8px; }
  .branch-chip { font-family: var(--font-mono); }
  .ghost-btn { display: inline-flex; align-items: center; justify-content: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.75rem; padding: 7px 10px; font-size: 13px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn.sm { padding: 5px 12px; }

  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .chip { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; padding: 3px 10px; border-radius: 9999px; border: 1px solid var(--color-crush-border); color: var(--color-crush-text); background: rgba(255,255,255,0.02); }
  .chip.accent { border-color: rgba(255,255,255,0.22); background: rgba(255,255,255,0.06); color: var(--color-crush-text); font-weight: 500; }

  .branch-row { grid-template-columns: 14px 150px auto 1fr 100px 80px auto; gap: 16px; cursor: default; }
  .branch-row:hover { background: none; }
</style>
