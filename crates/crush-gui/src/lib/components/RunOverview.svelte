<script lang="ts">
  import Icon from './Icon.svelte';
  import TechIcon from './TechIcon.svelte';
  import StatusBadge from './StatusBadge.svelte';
  import * as api from '$lib/tauri';

  let {
    status = 'running',
    port = null,
    url = null,
    framework = null,
    runtime = null,
    projectName,
    projectPath,
    deploymentId = null,
    branch = null,
    commit_short = null,
    onStop,
    onClose,
  }: {
    status?: 'running' | 'exited' | 'failed';
    port?: number | null;
    /** full preview URL from the detector (e.g. a swagger-ui path); preferred over port */
    url?: string | null;
    framework?: string | null;
    runtime?: string | null;
    projectName: string;
    projectPath: string;
    /** when set, the live preview is screenshotted into this deployment's cache */
    deploymentId?: string | null;
    branch?: string | null;
    commit_short?: string | null;
    onStop?: () => void;
    onClose?: () => void;
  } = $props();

  let frameEl: HTMLIFrameElement | undefined = $state();
  let captured = false;

  // Snapshot the rendered preview once, shortly after it loads, so the
  // deployment keeps a cached screenshot (like Vercel's OG image).
  function onFrameLoad() {
    if (captured || !deploymentId || status !== 'running' || !frameEl) return;
    captured = true;
    setTimeout(() => {
      if (!frameEl) return;
      const r = frameEl.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      api.capturePreview(
        projectName, deploymentId!,
        Math.round(r.left * dpr), Math.round(r.top * dpr),
        Math.round(r.width * dpr), Math.round(r.height * dpr),
      ).catch(() => {});
    }, 1500);
  }

  // Prefer the detector's full URL (backend swagger/docs); else build from port.
  let previewUrl = $derived(url ?? (port ? `http://localhost:${port}` : null));
  let previewLabel = $derived(previewUrl ? previewUrl.replace(/^https?:\/\//, '') : '');
  let previewKey = $state(0);
  // StatusBadge styles running (green) / exited|stopped (red); map both end states to 'exited'
  let badge = $derived(status === 'running' ? 'running' : 'exited');

  // Render the app at a logical desktop width and scale the whole thing down to
  // fit the card (Vercel-style thumbnail) instead of cropping a 1:1 slice.
  const BASE_W = 1280;
  const BOX_H = 340;
  let boxW = $state(0);
  let scale = $derived(boxW > 0 ? boxW / BASE_W : 1);

  function reload() { previewKey++; }
  function visit() { if (previewUrl) api.openUrl(previewUrl).catch(console.error); }
</script>

<div class="crush-card run-card">
  <div class="run-head">
    <h2>Live deployment</h2>
    <div class="run-actions">
      {#if previewUrl}<button class="btn" onclick={reload} title="Reload preview"><Icon name="refresh" size={13} /></button>{/if}
      {#if previewUrl}<button class="btn" onclick={visit}><Icon name="folder" size={12} /> Visit</button>{/if}
      {#if status === 'running'}
        <button class="btn danger" onclick={() => onStop?.()}><Icon name="stop" size={12} fill /> Stop</button>
      {:else}
        <button class="btn" onclick={() => onClose?.()}>Close</button>
      {/if}
    </div>
  </div>

  <div class="run-body">
    <!-- Preview -->
    <div class="preview" style="height:{BOX_H}px" bind:clientWidth={boxW}>
      {#if previewUrl && status === 'running'}
        {#key previewUrl + '|' + previewKey}
          <iframe
            bind:this={frameEl}
            src={previewUrl}
            title="App preview"
            onload={onFrameLoad}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
            style="width:{BASE_W}px; height:{BOX_H / scale}px; transform:scale({scale}); transform-origin:top left;"
          ></iframe>
        {/key}
      {:else if status === 'running'}
        <div class="preview-empty"><span class="spinner"></span><p>Waiting for the dev server to bind a port…</p></div>
      {:else}
        <div class="preview-empty"><Icon name="containers" size={26} /><p>Run {status} — preview unavailable.</p></div>
      {/if}
    </div>

    <!-- Metadata -->
    <div class="meta">
      <div class="mg">
        <span class="mk">Status</span>
        <span class="mv"><StatusBadge status={badge} /></span>
      </div>
      {#if previewUrl}
        <div class="mg">
          <span class="mk">URL</span>
          <button class="link" onclick={visit} title={previewUrl}>{previewLabel} <span class="arr">↗</span></button>
        </div>
      {/if}
      {#if framework || runtime}
        <div class="mg">
          <span class="mk">Stack</span>
          <span class="mv stack">
            {#if runtime}<span class="pill"><TechIcon name={runtime} size={13} />{runtime}</span>{/if}
            {#if framework}<span class="pill"><TechIcon name={framework} size={13} />{framework}</span>{/if}
          </span>
        </div>
      {/if}
      <div class="mg">
        <span class="mk">Project</span>
        <span class="mv">{projectName}</span>
      </div>
      <div class="mg">
        <span class="mk">Source</span>
        <span class="mv path mono" title={projectPath}>{projectPath}</span>
      </div>
      {#if branch}
        <div class="mg">
          <span class="mk">Branch / Commit</span>
          <span class="mv mono"><Icon name="branch" size={12} /> {branch} · {commit_short}</span>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .run-card { padding: 0; overflow: hidden; }
  .run-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--color-crush-border); }
  .run-head h2 { font-size: 14px; font-weight: 600; margin: 0; }
  .run-actions { display: flex; align-items: center; gap: 8px; }
  .btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 7px; padding: 6px 12px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .btn.danger { color: var(--color-crush-red); border-color: rgba(239,68,68,0.3); }
  .btn.danger:hover { background: rgba(239,68,68,0.1); }

  .run-body { display: grid; grid-template-columns: 1.4fr 1fr; gap: 0; }
  .preview { position: relative; overflow: hidden; background: #0a0a0c; border-right: 1px solid var(--color-crush-border); }
  .preview iframe { border: none; background: white; display: block; }
  .preview-empty { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: var(--color-crush-text-muted); font-size: 13px; text-align: center; padding: 20px; }
  .spinner { width: 22px; height: 22px; border: 2px solid var(--color-crush-border); border-top-color: var(--color-crush-text); border-radius: 50%; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .meta { display: flex; flex-direction: column; gap: 16px; padding: 18px 20px; overflow-y: auto; }
  .mg { display: flex; flex-direction: column; gap: 5px; }
  .mk { font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); }
  .mv { font-size: 13px; }
  .mv.path { color: var(--color-crush-muted); font-size: 11.5px; word-break: break-all; }
  .mono { font-family: var(--font-mono); }
  .link { display: inline-flex; align-items: center; gap: 5px; font-family: var(--font-mono); font-size: 13px; color: var(--color-crush-text); background: none; border: none; padding: 0; cursor: pointer; }
  .link:hover { text-decoration: underline; }
  .arr { opacity: 0.7; }
  .stack { display: flex; flex-wrap: wrap; gap: 6px; }
  .pill { display: inline-flex; align-items: center; gap: 5px; font-size: 12px; padding: 2px 9px; border-radius: 9999px; border: 1px solid var(--color-crush-border); background: rgba(255,255,255,0.02); }
  .pill :global(svg) { flex-shrink: 0; }
</style>
