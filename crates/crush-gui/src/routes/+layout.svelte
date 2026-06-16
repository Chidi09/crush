<script lang="ts">
  import '../app.css';
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toaster from '$lib/components/Toaster.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import { installNativeGuard } from '$lib/native-guard';
  import { installErrorGuard } from '$lib/error-guard';
  import { startPolling, stopPolling } from '$lib/stores/containers.svelte.ts';
  import { refreshServices } from '$lib/stores/services.svelte.ts';
  import { refreshImages } from '$lib/stores/images.svelte.ts';
  let { children } = $props();

  // Start all background polling here so individual pages never restart it on navigation.
  onMount(() => {
    installNativeGuard();
    installErrorGuard();
    startPolling();
    refreshServices();
    refreshImages();
  });
  onDestroy(() => stopPolling());
</script>

<div class="app-shell">
  <div class="crush-glow" style="top: -200px; right: -200px;"></div>
  <Sidebar />
  <main class="main-content">
    {#key $page.url.pathname}
      <div class="page-anim">
        {#if children}
          {@render children()}
        {/if}
      </div>
    {/key}
  </main>
  <Toaster />
  <CommandPalette />
  <ConfirmDialog />
</div>

<style>
  .app-shell {
    display: flex;
    min-height: 100vh;
    background: var(--color-crush-black);
    position: relative;
  }

  .main-content {
    margin-left: 48px;
    flex: 1;
    padding: 24px 32px;
    position: relative;
    z-index: 1;
    overflow-x: hidden;
  }

  /* Subtle page-transition on route change */
  .page-anim { animation: pageIn 0.26s cubic-bezier(0.22, 1, 0.36, 1); }
  @keyframes pageIn {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .page-anim { animation: none; }
  }
</style>
