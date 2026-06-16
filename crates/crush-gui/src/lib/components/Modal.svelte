<script lang="ts">
  import { onDestroy } from 'svelte';
  import { overlayRegistry } from '$lib/stores/OverlayRegistry';
  import { fade, slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  let {
    open = $bindable(false),
    title = "",
  }: { open?: boolean; title?: string } = $props();

  const id = Math.random().toString(36).substring(2);
  let node: HTMLElement | undefined = $state();

  $effect(() => {
    if (open && node) {
      overlayRegistry.register({ id, type: 'modal', node, close: () => (open = false) });
    } else {
      overlayRegistry.unregister(id);
    }
  });

  onDestroy(() => overlayRegistry.unregister(id));
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
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
      class="relative w-full max-w-lg bg-surface border border-[var(--border)] rounded-xl shadow-2xl overflow-hidden crush-card"
      transition:slide={{ duration: 250, easing: cubicOut, axis: 'y' }}
      role="dialog"
      aria-modal="true"
    >
      {#if title}
        <div class="px-5 py-4 border-b border-[var(--border)] font-medium text-lg">
          {title}
        </div>
      {/if}
      <div class="p-5">
        <slot />
      </div>
    </div>
  </div>
{/if}
