<script lang="ts">
  import { onMount } from 'svelte';
  import { images, refreshImages } from '$lib/stores/images.svelte.ts';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Identicon from '$lib/components/Identicon.svelte';
  import TechIcon, { lookupTech } from '$lib/components/TechIcon.svelte';

  // Prefer a real stack/tech logo (from the image's stack hint, else its tag's
  // repo name); fall back to a deterministic identicon when nothing resolves.
  function iconName(img: { stack?: string | null; tag: string }): string | null {
    const candidates = [img.stack, (img.tag || '').split(':')[0].split('/').pop()];
    for (const c of candidates) { if (c && lookupTech(c)) return c; }
    return null;
  }
  import * as api from '$lib/tauri';
  import type { ImageDetail } from '$lib/tauri';
  import { toast } from '$lib/stores/toast.svelte.ts';

  let pulling = $state(false);
  let pullRef = $state('');
  let pullError = $state<string | null>(null);
  let pullStatus = $state('');
  let search = $state('');
  let confirmId = $state<string | null>(null);

  let copied = $state<string | null>(null);

  // Curated catalog of popular images (shared with `crush catalog`).
  let catalog = $state<api.CatalogEntry[]>([]);
  let showCatalog = $state(false);
  let catalogSearch = $state('');
  let filteredCatalog = $derived(
    catalog.filter((e) => {
      const q = catalogSearch.toLowerCase();
      return !q || e.name.toLowerCase().includes(q) || e.reference.toLowerCase().includes(q)
        || e.category.toLowerCase().includes(q) || e.description.toLowerCase().includes(q);
    })
  );
  async function loadCatalog() {
    if (catalog.length) return;
    try { catalog = await api.listCatalog(); } catch (e) { console.error('catalog load failed', e); }
  }
  async function pullFromCatalog(ref: string) {
    pullRef = ref;
    await doPull();
  }

  let filtered = $derived(
    $images.filter(i => !search || (i.tag || '').toLowerCase().includes(search.toLowerCase()) || i.digest.includes(search))
  );
  let totalSize = $derived($images.reduce((a, i) => a + i.size_bytes, 0));

  onMount(() => { refreshImages(); loadCatalog(); });

  function copy(text: string, key: string) {
    navigator.clipboard.writeText(text).then(() => {
      copied = key;
      toast(key.startsWith('dg') ? 'Digest copied' : 'Image ID copied', 'success');
      setTimeout(() => { if (copied === key) copied = null; }, 1200);
    }).catch(() => {});
  }
  let inspecting = $state<ImageDetail | null>(null);
  let inspectId = $state<string | null>(null);
  async function doInspect(id: string) {
    inspectId = id; inspecting = null;
    try { inspecting = await api.inspectImage(id); }
    catch (e) { toast('Inspect failed', 'error'); inspectId = null; }
  }
  function closeInspect() { inspecting = null; inspectId = null; }

  async function doPull() {
    const ref = pullRef.trim();
    if (!ref || pulling) return;
    pulling = true;
    pullError = null;
    pullStatus = `Pulling ${ref}…`;
    try {
      await api.pullImage(ref);
      pullStatus = `Pulled ${ref}`;
      toast(`Pulled ${ref}`, 'success');
      pullRef = '';
      await refreshImages();
      setTimeout(() => { if (!pulling) pullStatus = ''; }, 2500);
    } catch (e) {
      pullError = String(e);
      pullStatus = '';
    } finally {
      pulling = false;
    }
  }

  async function doRemove(id: string) {
    try {
      await api.removeImage(id);
      confirmId = null;
      toast('Image removed', 'success');
      await refreshImages();
    } catch (e) {
      console.error('Remove failed', e);
      toast('Remove failed', 'error');
    }
  }

  function formatSize(bytes: number): string {
    if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(0)} KB`;
    if (bytes < 1_000_000_000) return `${(bytes / 1_000_000).toFixed(0)} MB`;
    return `${(bytes / 1_000_000_000).toFixed(2)} GB`;
  }
  function shortDigest(d: string): string {
    const h = d.replace(/^sha256:/, '');
    return h.length > 12 ? h.slice(0, 12) : h;
  }
</script>

<div class="page">
  <header class="page-header">
    <div class="title-wrap">
      <h1>Images</h1>
      {#if $images.length}<span class="count-badge">{$images.length} image{$images.length === 1 ? '' : 's'} · {formatSize(totalSize)}</span>{/if}
    </div>
    <input class="crush-input search" type="text" placeholder="Search tag or digest…" bind:value={search} />
  </header>

  <div class="crush-card pull-card">
    <div class="pull-bar">
      <input
        class="crush-input pull-input"
        type="text"
        placeholder="Pull an image…  e.g. python:3.11-slim"
        bind:value={pullRef}
        onkeydown={(e) => e.key === 'Enter' && doPull()}
        disabled={pulling}
      />
      <button class="pull-btn" onclick={doPull} disabled={pulling || !pullRef.trim()}>
        {#if pulling}<span class="spin"></span> Pulling{:else}<Icon name="images" size={14} /> Pull{/if}
      </button>
    </div>
    {#if pulling}
      <div class="prog"><div class="prog-bar"></div></div>
    {/if}
    {#if pullStatus}<p class="pull-status">{pullStatus}</p>{/if}
    {#if pullError}<p class="pull-error">{pullError}</p>{/if}

    <button class="catalog-toggle" onclick={() => (showCatalog = !showCatalog)}>
      <Icon name="images" size={13} />
      {showCatalog ? 'Hide' : 'Browse'} popular images ({catalog.length})
    </button>
  </div>

  {#if showCatalog}
    <div class="crush-card catalog-card">
      <input class="crush-input" type="text" placeholder="Search catalog… e.g. search, flaresolverr" bind:value={catalogSearch} />
      <div class="catalog-grid">
        {#each filteredCatalog as e (e.reference)}
          <div class="cat-item">
            <div class="cat-icon">
              {#if lookupTech(e.name.split(' ')[0])}
                <TechIcon name={e.name.split(' ')[0]} size={22} />
              {:else}
                <Identicon seed={e.reference} size={28} />
              {/if}
            </div>
            <div class="cat-body">
              <div class="cat-name">{e.name}{#if e.native}<span class="cat-native">native</span>{/if}</div>
              <div class="cat-desc">{e.description}</div>
              <div class="cat-ref">{e.reference}</div>
            </div>
            <button class="cat-pull" onclick={() => pullFromCatalog(e.reference)} disabled={pulling} title="Pull {e.reference}">
              <Icon name="images" size={13} /> Pull
            </button>
          </div>
        {/each}
        {#if filteredCatalog.length === 0}<p class="muted">No catalog entries match “{catalogSearch}”.</p>{/if}
      </div>
    </div>
  {/if}

  {#if $images.length === 0}
    <EmptyState title="No images cached" description="Pull an image above to get started" />
  {:else if filtered.length === 0}
    <p class="muted">No images match “{search}”.</p>
  {:else}
    <div class="image-list stagger">
      {#each filtered as img (img.id)}
        {@const ic = iconName(img)}
        <div class="crush-card image-row">
          <div class="img-ident">
            {#if ic}
              <TechIcon name={ic} size={26} />
            {:else}
              <Identicon seed={img.digest || img.id || img.tag} size={40} />
            {/if}
          </div>
          <div class="img-info">
            <span class="img-tag">{img.tag || '<untagged>'}</span>
            <span class="img-meta">
              {formatSize(img.size_bytes)} · {img.layer_count} layer{img.layer_count === 1 ? '' : 's'}
              {#if img.os} · {img.os}/{img.arch}{/if}
              · <span class="mono-dim">{shortDigest(img.digest)}</span>
            </span>
          </div>
          <div class="img-actions">
            <button class="icon-btn" onclick={() => doInspect(img.id)} title="Inspect image config">
              <Icon name="images" size={12} /> Inspect
            </button>
            <button class="icon-btn" onclick={() => copy(img.id, `id-${img.id}`)} title="Copy image ID">
              <Icon name={copied === `id-${img.id}` ? 'check' : 'copy'} size={12} /> ID
            </button>
            <button class="icon-btn" onclick={() => copy(img.digest, `dg-${img.id}`)} title="Copy digest">
              <Icon name={copied === `dg-${img.id}` ? 'check' : 'copy'} size={12} /> Digest
            </button>
            {#if confirmId === img.id}
              <div class="confirm">
                <span class="confirm-q">Delete?</span>
                <button class="confirm-yes" onclick={() => doRemove(img.id)}>Yes</button>
                <button class="confirm-no" onclick={() => confirmId = null}>No</button>
              </div>
            {:else}
              <button class="delete-btn" onclick={() => confirmId = img.id} title="Delete image">
                <Icon name="stop" size={12} /> Delete
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if inspectId}
  <button class="overlay" onclick={closeInspect} aria-label="Close"></button>
  <div class="inspect animate-slide-up">
    <div class="ins-head">
      <h2>{inspecting?.tag || '<untagged>'}</h2>
      <button class="close-btn" onclick={closeInspect} aria-label="Close">×</button>
    </div>
    {#if inspecting}
      <div class="ins-body">
        <dl class="kv">
          <dt>Digest</dt><dd class="mono">{inspecting.digest}</dd>
          <dt>Size</dt><dd>{formatSize(inspecting.size_bytes)}</dd>
          <dt>Platform</dt><dd class="mono">{inspecting.os}/{inspecting.arch}</dd>
          {#if inspecting.config_digest}<dt>Config</dt><dd class="mono">{shortDigest(inspecting.config_digest)}</dd>{/if}
        </dl>
        {#if inspecting.entrypoint.length}
          <div class="ins-sec"><span class="ins-k">Entrypoint</span><code class="ins-code">{inspecting.entrypoint.join(' ')}</code></div>
        {/if}
        {#if inspecting.cmd.length}
          <div class="ins-sec"><span class="ins-k">Cmd</span><code class="ins-code">{inspecting.cmd.join(' ')}</code></div>
        {/if}
        {#if inspecting.env.length}
          <div class="ins-sec"><span class="ins-k">Env</span><div class="env-list">{#each inspecting.env as e}<code class="env-item">{e}</code>{/each}</div></div>
        {/if}
        <div class="ins-sec">
          <span class="ins-k">Layers <span class="lcount">{inspecting.layers.length}</span></span>
          <div class="layers">{#each inspecting.layers as l, i}<div class="layer"><span class="li">{i + 1}</span><span class="mono-dim">{shortDigest(l)}</span></div>{/each}</div>
        </div>
      </div>
    {:else}
      <div class="ins-body"><p class="muted">Loading…</p></div>
    {/if}
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); backdrop-filter: blur(2px); z-index: 40; border: none; padding: 0; cursor: default; }
  .inspect { position: fixed; top: 4vh; left: 50%; transform: translateX(-50%); width: min(680px, 92vw); max-height: 92vh; background: var(--color-crush-dark); border: 1px solid var(--color-crush-border); border-radius: 0.75rem; z-index: 50; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 24px 80px rgba(0,0,0,0.6); }
  .ins-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--color-crush-border); }
  .ins-head h2 { font-size: 15px; font-weight: 600; margin: 0; font-family: var(--font-mono); }
  .close-btn { background: none; border: none; color: var(--color-crush-text-muted); font-size: 22px; line-height: 1; cursor: pointer; }
  .ins-body { padding: 16px 18px; overflow-y: auto; display: flex; flex-direction: column; gap: 16px; }
  .kv { display: grid; grid-template-columns: 80px 1fr; gap: 6px 12px; margin: 0; font-size: 13px; }
  .kv dt { color: var(--color-crush-text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; align-self: center; }
  .kv dd { margin: 0; word-break: break-all; }
  .kv .mono { font-family: var(--font-mono); font-size: 12px; }
  .ins-sec { display: flex; flex-direction: column; gap: 6px; }
  .ins-k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-crush-text-muted); display: flex; align-items: center; gap: 6px; }
  .lcount { background: var(--color-crush-surface); border-radius: 9999px; padding: 0 7px; font-size: 10px; line-height: 16px; }
  .ins-code { font-family: var(--font-mono); font-size: 12px; background: rgba(9,9,11,0.6); border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 8px 10px; color: #67e8f9; word-break: break-all; }
  .env-list { display: flex; flex-direction: column; gap: 4px; }
  .env-item { font-family: var(--font-mono); font-size: 11.5px; color: var(--color-crush-text-muted); word-break: break-all; }
  .layers { display: flex; flex-direction: column; gap: 2px; max-height: 200px; overflow-y: auto; }
  .layer { display: flex; align-items: center; gap: 10px; font-size: 12px; padding: 3px 0; }
  .layer .li { color: var(--color-crush-muted); width: 20px; font-family: var(--font-mono); font-size: 11px; }
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .title-wrap { display: flex; align-items: baseline; gap: 10px; }
  .count-badge { font-size: 12px; color: var(--color-crush-text-muted); }
  .search { width: 220px; }
  .img-actions { display: flex; align-items: center; gap: 6px; }
  .icon-btn { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 5px 10px; cursor: pointer; }
  .icon-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }

  .pull-card { padding: 14px 16px; margin-bottom: 20px; }
  .pull-bar { display: flex; gap: 8px; }
  .pull-input { flex: 1; }
  .pull-btn { display: inline-flex; align-items: center; gap: 6px; background: var(--color-crush-primary); color: var(--color-crush-on-primary); border: none; border-radius: 0.75rem; padding: 8px 20px; font-size: 13px; cursor: pointer; white-space: nowrap; transition: background 0.15s; }
  .pull-btn:hover:not(:disabled) { background: var(--color-crush-primary-hover); }
  .pull-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .prog { margin-top: 12px; height: 3px; border-radius: 9999px; background: var(--color-crush-border); overflow: hidden; }
  .prog-bar { height: 100%; width: 40%; border-radius: 9999px; background: var(--color-crush-primary); animation: indet 1.1s ease-in-out infinite; }
  @keyframes indet { 0% { margin-left: -40%; } 100% { margin-left: 100%; } }

  .pull-status { margin: 10px 0 0; font-size: 12px; color: var(--color-crush-text-muted); }
  .pull-error { margin: 10px 0 0; font-size: 12px; color: var(--color-crush-red); font-family: var(--font-mono); }

  .muted { color: var(--color-crush-text-muted); font-size: 13px; }

  /* Curated catalog */
  .catalog-toggle { margin-top: 10px; display: inline-flex; align-items: center; gap: 6px;
    background: none; border: none; color: var(--color-crush-text-muted); font-size: 12px; cursor: pointer; padding: 2px 0; }
  .catalog-toggle:hover { color: var(--color-crush-text); }
  .catalog-card { padding: 14px 16px; margin-bottom: 20px; display: flex; flex-direction: column; gap: 12px; }
  .catalog-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px; }
  .cat-item { display: flex; align-items: flex-start; gap: 10px; padding: 10px 12px;
    border: 1px solid var(--color-crush-border); border-radius: 0.75rem; background: var(--color-crush-surface); }
  .cat-icon { flex-shrink: 0; width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; line-height: 0; }
  .cat-body { flex: 1; min-width: 0; }
  .cat-name { font-size: 13px; font-weight: 600; display: flex; align-items: center; gap: 6px; }
  .cat-native { font-size: 10px; font-weight: 600; color: #34d399; border: 1px solid #34d39955; border-radius: 4px; padding: 0 4px; }
  .cat-desc { font-size: 12px; color: var(--color-crush-text-muted); margin: 2px 0; }
  .cat-ref { font-size: 11px; font-family: var(--font-mono); color: var(--color-crush-text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cat-pull { flex-shrink: 0; display: inline-flex; align-items: center; gap: 5px; background: var(--color-crush-surface-2, var(--color-crush-surface));
    color: var(--color-crush-text); border: 1px solid var(--color-crush-border); border-radius: 0.6rem; padding: 5px 10px; font-size: 12px; cursor: pointer; }
  .cat-pull:hover:not(:disabled) { background: var(--color-crush-primary); color: var(--color-crush-on-primary); border-color: transparent; }
  .cat-pull:disabled { opacity: 0.5; cursor: not-allowed; }

  .image-list { display: flex; flex-direction: column; gap: 8px; }
  .image-row { display: flex; align-items: center; gap: 14px; padding: 16px 20px; }
  .img-ident { flex-shrink: 0; width: 40px; height: 40px; display: flex; align-items: center; justify-content: center; line-height: 0; }
  .img-info { display: flex; flex-direction: column; gap: 4px; min-width: 0; flex: 1; }
  .img-tag { font-size: 14px; font-weight: 500; font-family: var(--font-mono); }
  .img-meta { font-size: 12px; color: var(--color-crush-text-muted); }
  .mono-dim { font-family: var(--font-mono); color: var(--color-crush-muted); }

  .delete-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-crush-red); background: none; border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; padding: 6px 14px; cursor: pointer; transition: background 0.15s; }
  .delete-btn:hover { background: rgba(239,68,68,0.1); }
  .confirm { display: flex; align-items: center; gap: 8px; }
  .confirm-q { font-size: 12px; color: var(--color-crush-text-muted); }
  .confirm-yes { font-size: 12px; color: white; background: var(--color-crush-red); border: none; border-radius: 6px; padding: 6px 12px; cursor: pointer; }
  .confirm-no { font-size: 12px; color: var(--color-crush-text-muted); background: none; border: 1px solid var(--color-crush-border); border-radius: 6px; padding: 6px 12px; cursor: pointer; }
</style>
