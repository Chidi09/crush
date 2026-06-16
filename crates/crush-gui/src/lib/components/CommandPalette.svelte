<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { contextBus } from '$lib/stores/ContextBus';
  import { fade, slide } from 'svelte/transition';

  let { open = $bindable(false) }: { open?: boolean } = $props();
  let search = $state("");

  const actions = [
    { label: "New deployment", shortcut: "D" },
    { label: "New query", shortcut: "Q" },
    { label: "Settings", shortcut: "S" },
  ];

  let filtered = $derived(
    search ? actions.filter((a) => a.label.toLowerCase().includes(search.toLowerCase())) : actions
  );

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      open = !open;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleKeydown);
    }
  });
</script>

{#if open}
  <div class="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh] bg-black/50 backdrop-blur-sm" transition:fade={{ duration: 150 }}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="absolute inset-0" role="presentation" onclick={() => (open = false)}></div>
    <div
      class="relative w-full max-w-xl bg-surface border border-[var(--border-strong)] rounded-xl shadow-[var(--elevation-4)] overflow-hidden flex flex-col"
      transition:slide={{ duration: 200, axis: 'y' }}
    >
      <div class="p-3 border-b border-[var(--border)] flex items-center gap-3">
        <span class="text-text-muted">⌘</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={search}
          placeholder="Jump to server, app, or command..."
          class="flex-1 bg-transparent border-none outline-none text-lg text-white"
          autofocus
        />
      </div>
      <div class="max-h-[60vh] overflow-y-auto p-2">
        {#each filtered as item}
          <button class="w-full flex items-center justify-between px-3 py-2.5 rounded-md hover:bg-surface-hover text-left text-sm transition-colors">
            <span>{item.label}</span>
            {#if item.shortcut}
              <kbd class="px-2 py-0.5 bg-surface-raised border border-[var(--border)] rounded text-xs text-text-muted">{item.shortcut}</kbd>
            {/if}
          </button>
        {/each}
        {#if filtered.length === 0}
          <div class="p-4 text-center text-text-muted text-sm">No results found.</div>
        {/if}
      </div>
      {#if $contextBus.activeServer}
        <div class="px-3 py-2 bg-surface-raised border-t border-[var(--border)] text-xs text-text-muted flex gap-2">
          <span>Context:</span>
          <span class="text-crush-orange">{$contextBus.activeServer}</span>
        </div>
      {/if}
    </div>
  </div>
{/if}
