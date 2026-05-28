<script lang="ts">
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  let { children } = $props();
</script>

<div class="app-shell">
  <div class="crush-glow" style="top: -200px; right: -200px;"></div>
  <Sidebar />
  <main class="main-content">
    {#if children}
      {@render children()}
    {/if}
  </main>
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
</style>
