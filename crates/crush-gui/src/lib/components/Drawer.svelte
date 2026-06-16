<script lang="ts">
  import { onDestroy } from 'svelte';
  import { overlayRegistry } from '$lib/stores/OverlayRegistry';
  import { fade, slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  let {
    open = $bindable(false),
    title = "",
    side = 'right',
  }: { open?: boolean; title?: string; side?: 'left' | 'right' } = $props();

  const id = Math.random().toString(36).substring(2);
  let node: HTMLElement | undefined = $state();

  $effect(() => {
    if (open && node) {
      overlayRegistry.register({ id, type: 'drawer', node, close: () => (open = false) });
    } else {
      overlayRegistry.unregister(id);
    }
  });

  onDestroy(() => overlayRegistry.unregister(id));
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex bg-black/60 backdrop-blur-sm"
    class:justify-end={side === 'right'}
    class:justify-start={side === 'left'}
    transition:fade={{ duration: 150 }}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="absolute inset-0"
      role="presentation"
      onclick={() => (open = false)}
      onkeydown={(e) => e.key === 'Escape' && (open = false)}
    ></div>

    <div
      bind:this={node}
      class="relative w-full max-w-md h-full bg-surface border-x border-[var(--border)] shadow-2xl flex flex-col"
      transition:slide={{ duration: 300, easing: cubicOut, axis: 'x' }}
      role="dialog"
      aria-modal="true"
    >
      {#if title}
        <div class="px-5 py-4 border-b border-[var(--border)] font-medium text-lg flex items-center justify-between">
          <span>{title}</span>
          <button class="text-text-muted hover:text-text" onclick={() => (open = false)}>✕</button>
        </div>
      {/if}
      <div class="p-5 flex-1 overflow-y-auto">
        <slot />
      </div>
    </div>
  </div>
{/if}
