<script lang="ts">
  import { onMount } from 'svelte';
  import { images, refreshImages } from '$lib/stores/images.svelte.ts';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';

  let pulling = $state(false);
  let pullRef = $state('');
  let pullError = $state<string | null>(null);
  let pullStatus = $state('');
  let search = $state('');
  let confirmId = $state<string | null>(null);

  let filtered = $derived(
    $images.filter(i => !search || i.tag.toLowerCase().includes(search.toLowerCase()))
  );

  onMount(refreshImages);

  async function doPull() {
    const ref = pullRef.trim();
    if (!ref || pulling) return;
    pulling = true;
    pullError = null;
    pullStatus = `Pulling ${ref}…`;
    try {
      await api.pullImage(ref);
      pullStatus = `Pulled ${ref}`;
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
      await refreshImages();
    } catch (e) {
      console.error('Remove failed', e);
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
    <h1>Images</h1>
    <input class="crush-input search" type="text" placeholder="Search…" bind:value={search} />
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
  </div>

  {#if $images.length === 0}
    <EmptyState title="No images cached" description="Pull an image above to get started" />
  {:else if filtered.length === 0}
    <p class="muted">No images match “{search}”.</p>
  {:else}
    <div class="image-list">
      {#each filtered as img (img.id)}
        <div class="crush-card image-row">
          <div class="img-info">
            <span class="img-tag">{img.tag}</span>
            <span class="img-meta">
              {formatSize(img.size_bytes)} · {img.layer_count} layer{img.layer_count === 1 ? '' : 's'} · <span class="mono-dim">{shortDigest(img.digest)}</span>
            </span>
          </div>
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
      {/each}
    </div>
  {/if}
</div>

<style>
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .search { width: 200px; }

  .pull-card { padding: 14px 16px; margin-bottom: 20px; }
  .pull-bar { display: flex; gap: 8px; }
  .pull-input { flex: 1; }
  .pull-btn { display: inline-flex; align-items: center; gap: 6px; background: var(--color-crush-orange); color: white; border: none; border-radius: 0.75rem; padding: 8px 20px; font-size: 13px; cursor: pointer; white-space: nowrap; transition: background 0.15s; }
  .pull-btn:hover:not(:disabled) { background: var(--color-crush-orange-light); }
  .pull-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .prog { margin-top: 12px; height: 3px; border-radius: 9999px; background: var(--color-crush-border); overflow: hidden; }
  .prog-bar { height: 100%; width: 40%; border-radius: 9999px; background: var(--color-crush-orange); animation: indet 1.1s ease-in-out infinite; }
  @keyframes indet { 0% { margin-left: -40%; } 100% { margin-left: 100%; } }

  .pull-status { margin: 10px 0 0; font-size: 12px; color: var(--color-crush-text-muted); }
  .pull-error { margin: 10px 0 0; font-size: 12px; color: var(--color-crush-red); font-family: var(--font-mono); }

  .muted { color: var(--color-crush-text-muted); font-size: 13px; }
  .image-list { display: flex; flex-direction: column; gap: 8px; }
  .image-row { display: flex; align-items: center; justify-content: space-between; padding: 16px 20px; }
  .img-info { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
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
