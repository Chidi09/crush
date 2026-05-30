<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toaster from '$lib/components/Toaster.svelte';
  let { children } = $props();
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
