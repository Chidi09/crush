<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import TerminalPane from '$lib/components/TerminalPane.svelte';
  import DeviceView from '$lib/components/DeviceView.svelte';
  import RunOverview from '$lib/components/RunOverview.svelte';
  import { goto } from '$app/navigation';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import TechIcon from '$lib/components/TechIcon.svelte';
  import { stackEffect } from '$lib/stack.ts';
  import * as api from '$lib/tauri';
  import type { ProjectInfo, SystemInfo, BuildSummary, ResourceUsage, GitInfo } from '$lib/tauri';
  import { containers, startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import { services, refreshServices } from '$lib/stores/services.svelte.ts';
  import { images, refreshImages } from '$lib/stores/images.svelte.ts';
  import { run } from '$lib/stores/run.svelte.ts';

  const LAST_PROJECT = 'crush:lastProject';

  let projectPath = $state<string | null>(null);
  let project = $state<ProjectInfo | null>(null);
  let git = $state<GitInfo | null>(null);
  let detecting = $state(false);
  let sys = $state<SystemInfo | null>(null);
  let res = $state<ResourceUsage | null>(null);
  let builds = $state<BuildSummary[]>([]);
  let refreshing = $state(false);
  let devMode = $state(false);

  // The active run lives in the `run` store (module scope) so it survives route
  // changes — leaving the dashboard no longer orphans the running process.
  function openProjectPage(name: string) { goto(`/projects/${encodeURIComponent(name)}`); }

  let runningCount = $derived($containers.filter(c => c.status === 'running').length);
  // Stack-aware glow for the Run buttons (rainbow for turbo/fast, brand for fullstack)
  let fx = $derived(stackEffect(project?.stack_kind, project?.framework));

  // ── Public tunnel ────────────────────────────────────────────────────────
  // Webhook-class providers (Paystack/Stripe/Clerk) need a public URL in dev.
  let webhookProviders = $derived(api.tunnelProviders(project?.external_services));
  // Prefer the live bound port; fall back to the detected project port.
  let tunnelPort = $derived(run.port ?? project?.port ?? null);
  let tunnel = $state<api.TunnelInfo | null>(null);
  let tunnelBusy = $state(false);
  let tunnelError = $state<string | null>(null);
  let copied = $state(false);

  async function startTunnel() {
    if (!tunnelPort) return;
    tunnelBusy = true; tunnelError = null;
    try {
      tunnel = await api.startTunnel(tunnelPort);
    } catch (e) {
      tunnelError = String(e);
    } finally {
      tunnelBusy = false;
    }
  }
  async function stopTunnel() {
    if (!tunnel) return;
    tunnelBusy = true;
    try {
      await api.stopTunnel(tunnel.port);
      tunnel = null;
    } catch (e) {
      tunnelError = String(e);
    } finally {
      tunnelBusy = false;
    }
  }
  async function copyTunnel() {
    if (!tunnel) return;
    try {
      await navigator.clipboard.writeText(tunnel.url);
      copied = true;
      setTimeout(() => { copied = false; }, 1500);
    } catch { /* clipboard unavailable */ }
  }

  // Summary-card previews (top few items per resource).
  let runningContainers = $derived($containers.filter(c => c.status === 'running'));
  let topImages = $derived([...$images].sort((a, b) => b.size_bytes - a.size_bytes).slice(0, 3));
  // A docker tag like "library/redis:7-alpine" → "redis" so TechIcon can match.
  function imgTech(tag: string): string { return (tag.split(':')[0].split('/').pop() ?? tag); }
  let pollId: ReturnType<typeof setInterval> | null = null;

  // Recent builds grouped by project (one row per project: latest state + count)
  type ProjectBuilds = { project: string; language: string; framework: string; latest: BuildSummary; count: number };
  function groupByProject(list: BuildSummary[]): ProjectBuilds[] {
    const map = new Map<string, ProjectBuilds>();
    for (const b of list) {
      const g = map.get(b.project_name);
      if (!g) {
        map.set(b.project_name, { project: b.project_name, language: b.language, framework: b.framework, latest: b, count: 1 });
      } else {
        g.count++;
        if (b.timestamp_ms > g.latest.timestamp_ms) { g.latest = b; g.language = b.language; g.framework = b.framework; }
      }
    }
    return [...map.values()].sort((a, b) => b.latest.timestamp_ms - a.latest.timestamp_ms);
  }
  const PER_PAGE = 6;
  let buildPage = $state(0);
  let projects = $derived(groupByProject(builds));
  let pageCount = $derived(Math.max(1, Math.ceil(projects.length / PER_PAGE)));
  let pagedProjects = $derived(projects.slice(buildPage * PER_PAGE, buildPage * PER_PAGE + PER_PAGE));

  // Disk breakdown → segmented usage bar
  const SEG_COLORS = ['#e05540', '#22d3ee', '#4ade80', '#c084fc', '#eab308', '#6b6b80'];

  // Favicon: try to load from the running dev server once a port is known.
  let faviconUrl = $state<string | null>(null);
  $effect(() => {
    if (run.port) {
      faviconUrl = `http://localhost:${run.port}/favicon.ico`;
    } else {
      faviconUrl = null;
    }
  });

  onMount(async () => {
    // startPolling is already called by the layout — don't restart it here.
    const autorun = sessionStorage.getItem('crush:autorun');
    const saved = localStorage.getItem(LAST_PROJECT);
    if (autorun) {
      // Branch preview handed off from the project page: detect + run it here
      // so the dashboard owns the run and shows its live preview + terminal.
      sessionStorage.removeItem('crush:autorun');
      projectPath = autorun;
      await detectStack(autorun);
      await loadAll();
      runProject();
    } else {
      if (saved) { projectPath = saved; detectStack(saved); }
      await loadAll();
    }
    pollId = setInterval(async () => { await refreshLight(); }, 5000);
  });
  onDestroy(() => { if (pollId) clearInterval(pollId); });

  async function refreshLight() {
    await Promise.allSettled([
      refreshServices(),
      refreshImages(),
      api.listBuildHistory(20).then(b => builds = b.slice().reverse()).catch(() => {}),
      api.systemResources().then(r => res = r).catch(() => {}),
    ]);
  }
  async function loadAll() {
    refreshing = true;
    await Promise.allSettled([refreshLight(), api.systemInfo().then(s => sys = s).catch(() => {})]);
    refreshing = false;
  }

  async function detectStack(path: string) {
    detecting = true; project = null; git = null; deployTargets = [];
    try {
      project = await api.detectProject(path);
      git = await api.gitInfo(path);
      deployTargets = await api.detectDeployTargets(path).catch(() => []);
      branches = git?.is_repo ? await api.gitBranches(path, false).catch(() => []) : [];
    } catch (e) { console.error(e); } finally { detecting = false; }
  }

  // Git-aware run: switch the branch you run/deploy without leaving the dashboard.
  let branches = $state<api.BranchInfo[]>([]);
  let sortedBranches = $derived(
    branches
      .filter((b) => !b.is_remote)
      .sort((a, b) => {
        const rank = (name: string) => {
          if (name === 'main') return 1;
          if (name === 'master') return 2;
          if (name === git?.branch) return 3;
          return 4;
        };
        const rA = rank(a.name);
        const rB = rank(b.name);
        if (rA !== rB) return rA - rB;
        return a.name.localeCompare(b.name);
      })
  );
  let switching = $state(false);
  let switchErr = $state<string | null>(null);
  async function onSwitchBranch(branch: string) {
    if (!projectPath || !branch || branch === git?.branch) return;
    switching = true; switchErr = null;
    try {
      await api.switchBranch(projectPath, branch);
      await detectStack(projectPath); // re-detect so the next run uses this branch
    } catch (e) {
      switchErr = String(e);
    } finally {
      switching = false;
    }
  }

  // Detected deploy platforms (Vercel/Netlify/Hetzner/…) + one-click deploy.
  let deployTargets = $state<api.DeployTarget[]>([]);
  let deploying = $state<string | null>(null);
  async function deployTo(t: api.DeployTarget) {
    if (!projectPath) return;
    deploying = t.platform;
    try { await api.openTerminal(projectPath, t.deploy_command); }
    catch (e) { console.error(e); }
    finally { setTimeout(() => { deploying = null; }, 1500); }
  }
  async function openProject() {
    const p = await api.pickProjectDirectory();
    if (p) { projectPath = p; localStorage.setItem(LAST_PROJECT, p); detectStack(p); }
  }
  async function runProject() {
    if (!projectPath) { await openProject(); return; }
    if (run.activeRunId) return;
    await run.start(projectPath, devMode, { project, git });
  }
  function revealData() { if (sys) api.revealInExplorer(sys.data_dir).catch(() => {}); }
  function go(path: string) { goto(path); }
  function goKey(e: KeyboardEvent, path: string) { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); goto(path); } }

  function fmtSize(b: number): string {
    if (!b) return '0 B';
    if (b < 1_000_000) return `${(b / 1000).toFixed(0)} KB`;
    if (b < 1_000_000_000) return `${(b / 1_000_000).toFixed(0)} MB`;
    return `${(b / 1_000_000_000).toFixed(2)} GB`;
  }
  function timeAgo(ms: number): string {
    const s = Math.floor((Date.now() - ms) / 1000);
    if (s < 60) return `${s}s ago`;
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    return `${Math.floor(s / 86400)}d ago`;
  }
  function fmtMs(ms: number): string { return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`; }
  function baseName(p: string): string { return p.split(/[\\/]/).filter(Boolean).pop() ?? p; }
</script>

<div class="dashboard">
  <header class="hero">
    <div class="hero-left">
      <img class="logo" src="/logo.png" alt="Crush" width="84" height="56" />
      <span class="wordmark">crush</span>
    </div>
    <div class="hero-right">
      <span class="running-ind"><span class="rdot" class:live={runningCount > 0}></span>{runningCount} running</span>
      <button class="ghost-btn" onclick={loadAll} title="Refresh" class:spinning={refreshing}><Icon name="refresh" size={14} /></button>
    </div>
  </header>

  {#if run.activeRunId}
    <div class="run-stack animate-slide-up">
      <RunOverview
        status={run.status}
        port={run.port}
        url={run.url}
        endpoints={run.endpoints}
        framework={run.project?.framework ?? null}
        runtime={run.project?.runtime ?? null}
        projectName={run.project?.name ?? (run.projectPath ? baseName(run.projectPath) : 'project')}
        projectPath={run.projectPath ?? ''}
        deploymentId={run.activeRunId}
        branch={run.git?.branch ?? null}
        commit_short={run.git?.head?.short ?? null}
        onStop={() => run.stop()}
        onClose={() => run.close()}
      />
      {#if run.isMobile}
        <!-- Mobile run: mirror the emulator/device alongside the log stream. -->
        <div class="mobile-split">
          <DeviceView />
          <TerminalPane />
        </div>
      {:else}
        <TerminalPane />
      {/if}
    </div>
  {/if}

  <!-- Current project -->
  <div class="crush-card project-card">
    {#if projectPath}
      <div class="proj-top">
        <div class="proj-id">
          <div class="proj-label">Current project</div>
          <div class="proj-name-row">
            {#if faviconUrl}
              <img class="proj-favicon" src={faviconUrl} alt="" width="18" height="18"
                onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none'; }} />
            {/if}
            <div class="proj-name">{project?.name ?? baseName(projectPath)}</div>
          </div>
          <div class="proj-path">{projectPath}</div>
        </div>
        <div class="proj-actions">
          {#if !run.activeRunId}
            <button class="ghost-btn sm" onclick={openProject}>Change</button>
            <label style="font-size: 13px; display: inline-flex; align-items: center; gap: 6px; cursor: pointer; color: var(--color-gray-400); margin-right: 8px;">
              <input type="checkbox" bind:checked={devMode} /> Dev mode
            </label>
            <button
              class="btn-primary"
              class:fx-rainbow={fx.kind === 'turbo'}
              class:fx-rainbow-soft={fx.kind === 'fullstack'}
              class:fx-rainbow-faint={fx.kind === 'spa'}
              onclick={runProject}
            >Run <Icon name="play" size={13} fill /></button>
          {:else}
            <button class="ghost-btn sm danger" onclick={() => run.stop()}>Stop</button>
          {/if}
        </div>
      </div>

      {#if !run.activeRunId}
        {#if detecting}
          <div class="chips"><span class="chip skel">detecting stack…</span></div>
        {:else if project}
          <div class="chips">
            <span class="chip accent"><TechIcon name={project.runtime} size={13} />{project.runtime}{project.version ? ` ${project.version}` : ''}</span>
            {#if project.framework}<span class="chip"><TechIcon name={project.framework} size={13} />{project.framework}</span>{/if}
            <span class="chip">:{project.port}</span>
            {#if project.is_monorepo}<span class="chip">monorepo · {project.service_count} svc</span>{/if}
            {#if project.env_required.length}<span class="chip muted-chip">{project.env_required.length} env</span>{/if}
            <span class="chip muted-chip">{Math.round(project.confidence * 100)}% match</span>
            {#if fx.kind === 'turbo'}<span class="chip fx-chip-rainbow">⚡ {fx.label}</span>
            {:else if fx.kind === 'fullstack'}<span class="chip fx-chip-grad">✦ {fx.label}</span>
            {:else if fx.kind === 'spa'}<span class="chip fx-chip-grad faint">○ {fx.label}</span>{/if}
          </div>
          {#if project.external_services && project.external_services.length > 0}
            <div class="uses-row">
              <span class="uses-label">Uses</span>
              <div class="chips inline-chips">
                {#each project.external_services as ext}
                  <span class="chip ext-chip" title={`${ext.source_var} detected`}>
                    <TechIcon name={ext.name} size={13} />
                    {ext.name}
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          {#if tunnelPort || tunnel}
            <div class="tunnel-row">
              <span class="uses-label">Tunnel</span>
              <div class="tunnel-body">
                {#if tunnel}
                  <a class="tunnel-url" href={tunnel.url} onclick={(e) => { e.preventDefault(); api.openUrl(tunnel!.url); }} title="Open public URL">
                    <Icon name="globe" size={13} /> {tunnel.url}
                  </a>
                  <span class="tunnel-via">via {tunnel.provider}</span>
                  <button class="ghost-btn sm" onclick={() => copyTunnel()} title="Copy URL">
                    <Icon name={copied ? 'check' : 'copy'} size={13} /> {copied ? 'copied' : 'copy'}
                  </button>
                  <button class="ghost-btn sm danger" onclick={() => stopTunnel()} disabled={tunnelBusy}>
                    <Icon name="x" size={13} /> stop
                  </button>
                {:else}
                  <span class="tunnel-hint">
                    {#if webhookProviders.length > 0}
                      {webhookProviders.map((p) => p.name).join(', ')} need a public URL for webhooks
                    {:else}
                      Expose :{tunnelPort} to the internet (free cloudflare tunnel)
                    {/if}
                  </span>
                  <button class="ghost-btn sm accent" onclick={() => startTunnel()} disabled={tunnelBusy || !tunnelPort}>
                    <Icon name="globe" size={13} /> {tunnelBusy ? 'opening…' : 'expose publicly'}
                  </button>
                {/if}
              </div>
              {#if tunnelError}<div class="tunnel-err">{tunnelError}</div>{/if}
            </div>
          {/if}

          {#if deployTargets.length > 0}
            <div class="deploy-row">
              <span class="uses-label">Deploys to</span>
              <div class="deploy-list">
                {#each deployTargets as t (t.platform)}
                  <div class="deploy-target" title={`Detected from ${t.source}`}>
                    <span class="dt-rocket"><Icon name="rocket" size={12} /></span>
                    <TechIcon name={t.icon} size={14} />
                    <span class="dt-name">{t.platform}</span>
                    <button class="dt-deploy" disabled={deploying !== null}
                            title={`Run: ${t.deploy_command}`}
                            onclick={() => deployTo(t)}>
                      {deploying === t.platform ? 'deploying…' : 'Deploy'}
                    </button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {/if}

        {#if git && git.is_repo}
          <div class="git-source mt-4">
            <div class="git-source-header">
              {#if git.parsed_github}
                <button class="ghost-btn sm" onclick={() => api.openUrl(git?.remote_url || '')} title="Open GitHub">
                  <Icon name="github" size={13} fill /> {git.parsed_github.owner}/{git.parsed_github.repo}
                </button>
              {/if}
              {#if branches.length > 1}
                <label class="branch-switch" title="Switch the branch you run & deploy">
                  <Icon name="branch" size={13} />
                  <select
                    value={git.branch}
                    disabled={switching}
                    onchange={(e) => onSwitchBranch((e.currentTarget as HTMLSelectElement).value)}
                  >
                    {#each sortedBranches as b (b.name)}
                      <option value={b.name}>{b.name}</option>
                    {/each}
                  </select>
                  {#if switching}<span class="dim sm">switching…</span>{/if}
                </label>
              {:else}
                <div class="chip accent branch-chip"><Icon name="branch" size={13} /> {git.branch}</div>
              {/if}
              <div class="chips">
                {#if git.dirty_count > 0}<span class="chip dirty-chip" title="Uncommitted changes in the working tree">● {git.dirty_count} uncommitted</span>{/if}
                {#if git.ahead && git.ahead > 0}<span class="chip">↑ {git.ahead}</span>{/if}
                {#if git.behind && git.behind > 0}<span class="chip">↓ {git.behind}</span>{/if}
              </div>
            </div>
            {#if switchErr}<div class="switch-err">{switchErr}</div>{/if}
            {#if git.head}
              <div class="git-commit mt-2">
                <span class="strong">{git.head.message}</span>
                <span class="dim sm">— {git.head.author} · {git.head.committed_rel}</span>
              </div>
            {/if}
          </div>
        {/if}
      {/if}
    {:else}
      <EmptyState title="No project open" description="Open a Crush project to detect its stack and run it" action="Open project…" onAction={openProject} />
    {/if}
  </div>

  <!-- Summary cards: each shows a live count + a peek at the top items -->
  <div class="stats stagger">
    <div class="crush-card sumcard" role="button" tabindex="0" onclick={() => go('/containers')} onkeydown={(e) => goKey(e, '/containers')}>
      <div class="sum-head">
        <div class="stat-icon"><Icon name="containers" size={15} /></div>
        <span class="stat-label">Containers</span>
        <span class="sum-count">{runningCount}{#if $containers.length > runningCount}<span class="sum-tot">/{$containers.length}</span>{/if}</span>
      </div>
      <div class="sum-list">
        {#each runningContainers.slice(0, 3) as c}
          <div class="sum-row"><span class="sd live"></span><span class="sum-name">{c.name}</span><span class="sum-meta mono">{c.ports.length ? `:${c.ports[0].host_port}` : '—'}</span></div>
        {:else}
          <div class="sum-empty">No running containers</div>
        {/each}
        {#if runningContainers.length > 3}<div class="sum-more">+{runningContainers.length - 3} more</div>{/if}
      </div>
    </div>

    <div class="crush-card sumcard" role="button" tabindex="0" onclick={() => go('/images')} onkeydown={(e) => goKey(e, '/images')}>
      <div class="sum-head">
        <div class="stat-icon cyan"><Icon name="images" size={15} /></div>
        <span class="stat-label">Images</span>
        <span class="sum-count">{$images.length}</span>
      </div>
      <div class="sum-list">
        {#each topImages as img}
          <div class="sum-row"><TechIcon name={imgTech(img.tag)} size={14} /><span class="sum-name mono">{img.tag}</span><span class="sum-meta mono">{fmtSize(img.size_bytes)}</span></div>
        {:else}
          <div class="sum-empty">No images pulled</div>
        {/each}
        {#if $images.length > 3}<div class="sum-more">+{$images.length - 3} more</div>{/if}
      </div>
    </div>

    <div class="crush-card sumcard" role="button" tabindex="0" onclick={() => go('/services')} onkeydown={(e) => goKey(e, '/services')}>
      <div class="sum-head">
        <div class="stat-icon green"><Icon name="services" size={15} /></div>
        <span class="stat-label">Services</span>
        <span class="sum-count">{$services.length}</span>
      </div>
      <div class="sum-list">
        {#each $services.slice(0, 3) as svc}
          <div class="sum-row"><TechIcon name={svc.kind || svc.name} size={14} /><span class="sum-name">{svc.name}</span><span class="sum-meta mono">:{svc.port}</span></div>
        {:else}
          <div class="sum-empty">No native services</div>
        {/each}
        {#if $services.length > 3}<div class="sum-more">+{$services.length - 3} more</div>{/if}
      </div>
    </div>
  </div>

  <div class="crush-card disk-card">
    <div class="disk-head">
      <div class="disk-title"><div class="stat-icon purple"><Icon name="disk" size={15} /></div><span class="stat-label">Disk used</span></div>
      <span class="disk-total">{sys ? fmtSize(sys.disk_used_bytes) : '—'}</span>
    </div>
    {#if sys && sys.disk_used_bytes > 0}
      <div class="disk-bar">
        {#each sys.disk_breakdown as seg, i}
          <span class="disk-seg" style="width:{(seg.bytes / sys.disk_used_bytes) * 100}%; background:{SEG_COLORS[i % SEG_COLORS.length]}" title="{seg.label} · {fmtSize(seg.bytes)}"></span>
        {/each}
      </div>
      <div class="disk-legend">
        {#each sys.disk_breakdown as seg, i}
          <span class="leg"><span class="leg-dot" style="background:{SEG_COLORS[i % SEG_COLORS.length]}"></span>{seg.label}<span class="leg-val">{fmtSize(seg.bytes)}</span></span>
        {/each}
      </div>
    {:else}
      <div class="disk-bar"><span class="disk-seg" style="width:100%; background:var(--color-crush-surface)"></span></div>
      <p class="muted">No data stored yet.</p>
    {/if}
  </div>

  <!-- Main (builds table) + side column -->
  <div class="grid-main">
    <div class="crush-card panel">
      <div class="panel-head"><h2>Projects</h2><span class="count">{projects.length}</span></div>
      {#if projects.length}
        <table class="tbl">
          <thead><tr><th>Project</th><th>Stack</th><th class="r">Builds</th><th>Last build</th><th class="r">When</th></tr></thead>
          <tbody>
            {#each pagedProjects as p}
              <tr class="clickable" role="button" tabindex="0" onclick={() => openProjectPage(p.project)} onkeydown={(e) => { if (e.key === 'Enter') openProjectPage(p.project); }}>
                <td class="strong">{p.project}</td>
                <td class="dim"><span class="stack-cell"><TechIcon name={p.framework && p.framework !== 'none' ? p.framework : p.language} size={13} />{p.language}{p.framework && p.framework !== 'none' ? ` · ${p.framework}` : ''}</span></td>
                <td class="r mono dim">{p.count}</td>
                <td><span class="tag" class:cached={p.latest.was_cached} class:fail={!p.latest.success}>{!p.latest.success ? 'failed' : p.latest.was_cached ? 'cached' : 'fresh'}</span> <span class="dim sm mono">{fmtMs(p.latest.duration_ms)}</span></td>
                <td class="r dim sm">{timeAgo(p.latest.timestamp_ms)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if pageCount > 1}
          <div class="pager">
            <button class="ghost-btn xs" disabled={buildPage === 0} onclick={() => buildPage = Math.max(0, buildPage - 1)}>Prev</button>
            <span class="pager-info">{buildPage + 1} / {pageCount}</span>
            <button class="ghost-btn xs" disabled={buildPage >= pageCount - 1} onclick={() => buildPage = Math.min(pageCount - 1, buildPage + 1)}>Next</button>
          </div>
        {/if}
      {:else}
        <p class="muted">No builds yet — run a project to populate history.</p>
      {/if}
    </div>

    <div class="side">
      <div class="crush-card panel">
        <div class="panel-head"><h2>System</h2></div>
        {#if res}
          <div class="usage">
            <div class="usage-row">
              <div class="usage-top"><span class="usage-k">CPU</span><span class="usage-v mono">{res.cpu_percent.toFixed(0)}%</span></div>
              <div class="ubar"><span class="ufill cpu" style="width:{Math.min(100, res.cpu_percent)}%"></span></div>
            </div>
            <div class="usage-row">
              <div class="usage-top"><span class="usage-k">Memory</span><span class="usage-v mono">{fmtSize(res.mem_used_bytes)} / {fmtSize(res.mem_total_bytes)}</span></div>
              <div class="ubar"><span class="ufill mem" style="width:{res.mem_total_bytes ? Math.min(100, (res.mem_used_bytes / res.mem_total_bytes) * 100) : 0}%"></span></div>
            </div>
          </div>
        {/if}
        <dl class="kv">
          <dt>Version</dt><dd class="mono">{sys?.version ?? '—'}</dd>
          <dt>Platform</dt><dd class="mono">{sys ? `${sys.os}/${sys.arch}` : '—'}</dd>
          <dt>Disk</dt><dd class="mono">{sys ? fmtSize(sys.disk_used_bytes) : '—'}</dd>
          <dt>Data dir</dt><dd class="mono path" title={sys?.data_dir}>{sys?.data_dir ?? '—'}</dd>
        </dl>
        {#if sys}<button class="ghost-btn sm full" onclick={revealData}><Icon name="folder" size={13} /> Open data dir</button>{/if}
      </div>
    </div>
  </div>
</div>

<style>
  .dashboard { display: flex; flex-direction: column; gap: 14px; padding-bottom: 24px; }
  .run-stack { display: flex; flex-direction: column; gap: 14px; }
  /* Mobile run: device mirror beside the logs; stacks on narrow widths. */
  .mobile-split { display: grid; grid-template-columns: auto 1fr; gap: 14px; align-items: start; }
  @media (max-width: 900px) { .mobile-split { grid-template-columns: 1fr; } }

  .hero { display: flex; align-items: center; justify-content: space-between; }
  .hero-left { display: flex; align-items: center; gap: 10px; }
  .logo { display: block; height: 56px; width: auto; object-fit: contain; }
  .wordmark { font-family: var(--font-mono); font-size: 20px; font-weight: 700; letter-spacing: -0.02em; }
  .hero-right { display: flex; align-items: center; gap: 12px; }
  .running-ind { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--color-crush-text-muted); }
  .rdot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-crush-muted); }
  .rdot.live { background: var(--color-crush-green); box-shadow: 0 0 6px rgba(74,222,128,0.5); }
  .ghost-btn { display: inline-flex; align-items: center; justify-content: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.75rem; padding: 7px 10px; font-size: 13px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn.sm { padding: 5px 12px; } .ghost-btn.xs { padding: 3px 10px; font-size: 12px; } .ghost-btn.full { width: 100%; margin-top: 10px; }
  .ghost-btn.spinning :global(svg) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .project-card { padding: 20px; }
  .proj-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; margin-bottom: 14px; }
  .proj-id { min-width: 0; }
  .proj-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .proj-name-row { display: flex; align-items: center; gap: 8px; margin-top: 2px; }
  .proj-favicon { border-radius: 3px; object-fit: contain; flex-shrink: 0; }
  .proj-name { font-size: 20px; font-weight: 600; }
  .proj-path { font-family: var(--font-mono); font-size: 12px; color: var(--color-crush-muted); margin-top: 4px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ghost-btn.danger { color: var(--color-crush-red); border-color: rgba(239,68,68,0.3); }
  .ghost-btn.danger:hover { background: rgba(239,68,68,0.08); border-color: rgba(239,68,68,0.5); }
  .proj-actions { display: flex; gap: 8px; flex-shrink: 0; }
  .btn-primary { display: inline-flex; align-items: center; gap: 6px; background: var(--color-crush-primary); color: var(--color-crush-on-primary); border: none; border-radius: 0.75rem; padding: 7px 18px; font-size: 13px; cursor: pointer; transition: background 0.15s; }
  .btn-primary:hover { background: var(--color-crush-primary-hover); }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .chip { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; padding: 3px 10px; border-radius: 9999px; border: 1px solid var(--color-crush-border); color: var(--color-crush-text); background: rgba(255,255,255,0.02); }
  .chip :global(svg) { flex-shrink: 0; }
  .chip.accent { border-color: rgba(255,255,255,0.22); background: rgba(255,255,255,0.06); color: var(--color-crush-text); font-weight: 500; }
  .chip.muted-chip, .chip.skel { color: var(--color-crush-text-muted); }
  .chip.fx-chip-rainbow {
    border-color: transparent; font-weight: 600; color: #0b0b0d;
    background: linear-gradient(90deg, #ff2d55, #ff8a00, #ffe600, #36e27b, #00b3ff, #8a5cff);
    background-size: 200% 100%; animation: fx-chip-slide 3s linear infinite;
  }
  @keyframes fx-chip-slide { to { background-position: 200% 0; } }
  /* fullstack / spa — rainbow gradient *text*, faint border (understated) */
  .chip.fx-chip-grad {
    border-color: rgba(255,255,255,0.18); font-weight: 600;
    background: rgba(255,255,255,0.03);
    background-image: linear-gradient(90deg, #ff2d55, #ff8a00, #ffe600, #36e27b, #00b3ff, #8a5cff);
    background-size: 200% 100%;
    -webkit-background-clip: text; background-clip: text; color: transparent;
    animation: fx-chip-slide 6s linear infinite;
  }
  .chip.fx-chip-grad.faint { opacity: 0.7; animation-duration: 9s; }

  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; align-items: start; }
  .stat-icon { width: 30px; height: 30px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; border-radius: 8px; background: rgba(255,255,255,0.08); color: var(--color-crush-text); }
  .stat-icon.cyan { background: rgba(34,211,238,0.12); color: #22d3ee; }
  .stat-icon.green { background: rgba(74,222,128,0.12); color: #4ade80; }
  .stat-icon.purple { background: rgba(192,132,252,0.12); color: #c084fc; }
  .stat-label { font-size: 11px; color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }

  .sumcard { padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; cursor: pointer; text-align: left; }
  .sum-head { display: flex; align-items: center; gap: 10px; }
  .sum-head .stat-label { flex: 1; }
  .sum-count { font-size: 20px; font-weight: 700; line-height: 1; }
  .sum-tot { font-size: 13px; font-weight: 500; color: var(--color-crush-text-muted); }
  .sum-list { display: flex; flex-direction: column; gap: 2px; min-height: 66px; }
  .sum-row { display: flex; align-items: center; gap: 8px; font-size: 12px; padding: 3px 0; }
  .sum-row :global(svg) { flex-shrink: 0; opacity: 0.9; }
  .sum-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sum-meta { color: var(--color-crush-text-muted); flex-shrink: 0; }
  .sd { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; background: var(--color-crush-muted); }
  .sd.live { background: var(--color-crush-green); box-shadow: 0 0 6px rgba(74,222,128,0.5); }
  .sum-empty { font-size: 12px; color: var(--color-crush-text-muted); padding: 6px 0; }
  .sum-more { font-size: 11px; color: var(--color-crush-muted); padding-top: 2px; }

  .disk-card { padding: 16px 18px; display: flex; flex-direction: column; gap: 12px; }
  .disk-head { display: flex; align-items: center; justify-content: space-between; }
  .disk-title { display: flex; align-items: center; gap: 10px; }
  .disk-total { font-size: 18px; font-weight: 700; font-family: var(--font-mono); }
  .disk-bar { display: flex; width: 100%; height: 12px; border-radius: 6px; overflow: hidden; background: var(--color-crush-surface); }
  .disk-seg { height: 100%; transition: width 0.4s ease; }
  .disk-seg + .disk-seg { box-shadow: -1px 0 0 rgba(9,9,11,0.5); }
  .disk-legend { display: flex; flex-wrap: wrap; gap: 8px 16px; }
  .leg { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); }
  .leg-dot { width: 8px; height: 8px; border-radius: 2px; flex-shrink: 0; }
  .leg-val { color: var(--color-crush-text); font-family: var(--font-mono); font-size: 11px; }

  .pager { display: flex; align-items: center; justify-content: center; gap: 12px; margin-top: 12px; }
  .pager-info { font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }
  .ghost-btn.xs:disabled { opacity: 0.4; cursor: default; }

  .grid-main { display: grid; grid-template-columns: 2fr 1fr; gap: 14px; align-items: start; }
  .side { display: flex; flex-direction: column; gap: 14px; }
  .panel { padding: 16px 18px; }
  .panel-head { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
  .panel-head h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0; }
  .count { font-size: 11px; color: var(--color-crush-muted); background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 9999px; padding: 0 8px; line-height: 18px; }

  .tbl { width: 100%; border-collapse: collapse; font-size: 13px; }
  .tbl th { text-align: left; font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); font-weight: 500; padding: 0 10px 8px; border-bottom: 1px solid var(--color-crush-border); }
  .tbl th.r, .tbl td.r { text-align: right; }
  .tbl td { padding: 9px 10px; border-bottom: 1px solid rgba(42,42,53,0.4); }
  .tbl tbody tr:last-child td { border-bottom: none; }
  .tbl tbody tr:hover { background: rgba(255,255,255,0.03); }
  .tbl tbody tr.clickable { cursor: pointer; }
  .strong { font-weight: 500; }
  .dim { color: var(--color-crush-text-muted); }
  .sm { font-size: 12px; }
  .mono { font-family: var(--font-mono); }
  .stack-cell { display: inline-flex; align-items: center; gap: 6px; }
  .stack-cell :global(svg) { flex-shrink: 0; opacity: 0.85; }

  .tag { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; padding: 1px 8px; border-radius: 9999px; background: rgba(255,255,255,0.08); color: var(--color-crush-text); border: 1px solid rgba(255,255,255,0.18); }
  .tag.cached { background: rgba(74,222,128,0.1); color: var(--color-crush-green); border-color: rgba(74,222,128,0.2); }
  .tag.fail { background: rgba(239,68,68,0.1); color: var(--color-crush-red); border-color: rgba(239,68,68,0.2); }

  .usage { display: flex; flex-direction: column; gap: 12px; margin-bottom: 14px; padding-bottom: 14px; border-bottom: 1px solid var(--color-crush-border); }
  .usage-row { display: flex; flex-direction: column; gap: 6px; }
  .usage-top { display: flex; align-items: center; justify-content: space-between; }
  .usage-k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-crush-text-muted); }
  .usage-v { font-size: 12px; }
  .ubar { width: 100%; height: 6px; border-radius: 3px; background: var(--color-crush-surface); overflow: hidden; }
  .ufill { display: block; height: 100%; border-radius: 3px; transition: width 0.4s ease; }
  .ufill.cpu { background: #22d3ee; }
  .ufill.mem { background: #c084fc; }

  .kv { display: grid; grid-template-columns: 70px 1fr; gap: 8px 10px; margin: 0; font-size: 12px; }
  .kv dt { color: var(--color-crush-text-muted); text-transform: uppercase; letter-spacing: 0.04em; font-size: 10px; align-self: center; }
  .kv dd { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kv .path { color: var(--color-crush-muted); }

  .muted { color: var(--color-crush-text-muted); font-size: 13px; padding: 4px 0; }
  
  .mt-4 { margin-top: 16px; }
  .mt-2 { margin-top: 8px; }
  .git-source { padding-top: 16px; border-top: 1px solid var(--color-crush-border); }
  .git-source-header { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .git-commit { display: flex; align-items: baseline; gap: 8px; }
  .branch-chip { font-family: var(--font-mono); }
  .branch-switch { display: inline-flex; align-items: center; gap: 6px; padding: 3px 8px; border: 1px solid rgba(99,102,241,0.3); background: rgba(99,102,241,0.06); border-radius: 0.75rem; color: #a5b4fc; }
  .branch-switch select { background: transparent; border: none; color: var(--color-crush-text); font-family: var(--font-mono); font-size: 12.5px; cursor: pointer; outline: none; max-width: 220px; }
  .branch-switch select:disabled { opacity: 0.6; }
  .dirty-chip { color: #eab308 !important; border-color: rgba(234,179,8,0.3) !important; background: rgba(234,179,8,0.08) !important; }
  .switch-err { color: var(--color-crush-red); font-size: 12px; margin-top: 6px; }

  .uses-row { display: flex; align-items: center; gap: 8px; margin-top: 12px; padding-top: 12px; border-top: 1px dashed var(--color-crush-border); }
  .uses-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); font-weight: 600; flex-shrink: 0; }
  .inline-chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .ext-chip { background: rgba(99,102,241,0.06) !important; border-color: rgba(99,102,241,0.18) !important; color: #a5b4fc !important; font-weight: 500; }

  /* ── deploy targets ── */
  .deploy-row { display: flex; align-items: flex-start; gap: 8px; margin-top: 10px; padding-top: 10px; flex-wrap: wrap; }
  .deploy-list { display: flex; flex-wrap: wrap; gap: 8px; }
  .deploy-target { display: inline-flex; align-items: center; gap: 7px; padding: 4px 6px 4px 10px; border-radius: 9999px; background: rgba(224,85,64,0.06); border: 1px solid rgba(224,85,64,0.22); font-size: 12.5px; }
  .dt-rocket { display: inline-flex; color: var(--color-crush-orange); filter: drop-shadow(0 0 4px rgba(224,85,64,0.7)); }
  .dt-name { font-weight: 500; }
  .dt-deploy { font-size: 11.5px; color: var(--color-crush-orange); background: rgba(224,85,64,0.1); border: 1px solid rgba(224,85,64,0.3); border-radius: 9999px; padding: 2px 10px; cursor: pointer; transition: background 0.15s, border-color 0.15s; }
  .dt-deploy:hover:not(:disabled) { background: rgba(224,85,64,0.2); border-color: rgba(224,85,64,0.55); }
  .dt-deploy:disabled { opacity: 0.5; cursor: default; }

  /* ── tunnel ── */
  .tunnel-row { display: flex; align-items: flex-start; gap: 8px; margin-top: 10px; padding-top: 10px; flex-wrap: wrap; }
  .tunnel-body { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .tunnel-hint { font-size: 12.5px; color: var(--color-crush-text-muted); }
  .tunnel-url { display: inline-flex; align-items: center; gap: 6px; font-family: var(--font-mono); font-size: 12.5px; color: #4ade80; text-decoration: none; padding: 4px 9px; border-radius: 0.6rem; background: rgba(74,222,128,0.08); border: 1px solid rgba(74,222,128,0.22); }
  .tunnel-url:hover { background: rgba(74,222,128,0.14); }
  .tunnel-via { font-size: 11.5px; color: var(--color-crush-text-muted); }
  .tunnel-err { flex-basis: 100%; font-size: 12px; color: var(--color-crush-red); margin-top: 2px; }
  .ghost-btn.accent { color: #4ade80; border-color: rgba(74,222,128,0.3); }
  .ghost-btn.accent:hover { background: rgba(74,222,128,0.08); border-color: rgba(74,222,128,0.55); }
</style>
