<script lang="ts">
  import { fade } from 'svelte/transition';

  let {
    open = $bindable(false),
    x = 0,
    y = 0,
    items = [],
    onaction,
  }: {
    open?: boolean;
    x?: number;
    y?: number;
    items?: { label: string; action: string }[];
    onaction?: (action: string) => void;
  } = $props();

  function handleAction(action: string) {
    onaction?.(action);
    open = false;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) open = false; }}
    oncontextmenu={(e) => { e.preventDefault(); open = false; }}
  >
    <div
      class="absolute bg-surface border border-[var(--border-strong)] rounded-lg shadow-[var(--elevation-3)] py-1 min-w-[160px] text-sm crush-card"
      style="left: {x}px; top: {y}px;"
      transition:fade={{ duration: 100 }}
    >
      {#each items as item}
        <button
          class="w-full text-left px-4 py-1.5 hover:bg-surface-hover hover:text-white transition-colors"
          onclick={() => handleAction(item.action)}
        >
          {item.label}
        </button>
      {/each}
    </div>
  </div>
{/if}
