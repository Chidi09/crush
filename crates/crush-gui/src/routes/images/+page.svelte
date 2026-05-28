<script lang="ts">
  import { onMount } from 'svelte';
  import { images, refreshImages } from '$lib/stores/images.svelte.ts';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import * as api from '$lib/tauri';

  let pulling = $state(false);
  let pullRef = $state('');

  onMount(refreshImages);

  async function doPull() {
    if (!pullRef.trim()) return;
    pulling = true;
    try {
      await api.pullImage(pullRef.trim());
      pullRef = '';
      await refreshImages();
    } catch (e) {
      console.error('Pull failed', e);
    } finally {
      pulling = false;
    }
  }

  async function removeImage(id: string) {
    try {
      await api.removeImage(id);
      await refreshImages();
    } catch (e) {
      console.error('Remove failed', e);
    }
  }

  function formatSize(bytes: number): string {
    if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(0)} KB`;
    if (bytes < 1_000_000_000) return `${(bytes / 1_000_000).toFixed(0)} MB`;
    return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  }
</script>

<div class="page">
  <header class="page-header">
    <h1>Images</h1>
  </header>

  <div class="pull-bar">
    <input class="crush-input pull-input" type="text" placeholder="Pull image… (e.g. python:3.11-slim)" bind:value={pullRef} />
    <button class="pull-btn" onclick={doPull} disabled={pulling}>
      {pulling ? 'Pulling…' : 'Pull'}
    </button>
  </div>

  {#if $images.length === 0}
    <EmptyState title="No images cached" description="Pull an image to get started" />
  {:else}
    <div class="image-list">
      {#each $images as img}
        <div class="crush-card image-row">
          <div class="img-info">
            <span class="img-tag">{img.tag}</span>
            <span class="img-meta">{formatSize(img.size_bytes)} · {img.layer_count} layers</span>
          </div>
          <button class="delete-btn" onclick={() => removeImage(img.id)}>Delete</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page-header { margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }

  .pull-bar { display: flex; gap: 8px; margin-bottom: 20px; }
  .pull-input { flex: 1; }
  .pull-btn { background: var(--color-crush-orange); color: white; border: none; border-radius: 8px; padding: 8px 20px; font-size: 13px; cursor: pointer; white-space: nowrap; }
  .pull-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .image-list { display: flex; flex-direction: column; gap: 8px; }
  .image-row { display: flex; align-items: center; justify-content: space-between; padding: 16px 20px; }

  .img-info { display: flex; flex-direction: column; gap: 2px; }
  .img-tag { font-size: 14px; font-weight: 500; font-family: var(--font-mono); }
  .img-meta { font-size: 12px; color: var(--color-crush-text-muted); }

  .delete-btn { font-size: 12px; color: #ef4444; background: none; border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; padding: 6px 16px; cursor: pointer; }
</style>
