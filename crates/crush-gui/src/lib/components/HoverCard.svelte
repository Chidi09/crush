<script lang="ts">
  import { fade } from 'svelte/transition';
  import type { Snippet } from 'svelte';

  let {
    text = '',
    children,
    content,
  }: { text?: string; children?: Snippet; content?: Snippet } = $props();

  let open = $state(false);
  let timer: ReturnType<typeof setTimeout>;

  function onEnter() {
    clearTimeout(timer);
    timer = setTimeout(() => (open = true), 300); // intentionality buffer
  }

  function onLeave() {
    clearTimeout(timer);
    timer = setTimeout(() => (open = false), 150);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative inline-block" role="presentation" onmouseenter={onEnter} onmouseleave={onLeave}>
  {@render children?.()}

  {#if open}
    <div
      class="absolute z-40 bottom-full mb-2 left-1/2 -translate-x-1/2 w-64 p-3 bg-surface border border-[var(--border-strong)] rounded-lg shadow-[var(--elevation-3)] crush-card text-sm"
      transition:fade={{ duration: 100 }}
    >
      {#if text}{text}{/if}
      {@render content?.()}
    </div>
  {/if}
</div>
