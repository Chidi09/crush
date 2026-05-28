<script lang="ts">
  import { onMount } from 'svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import CopyField from '$lib/components/CopyField.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { services, refreshServices } from '$lib/stores/services.svelte.ts';
  import * as api from '$lib/tauri';

  let grouped = $derived.by(() => {
    const map = new Map<string, typeof $services>();
    for (const s of $services) {
      if (!map.has(s.project)) map.set(s.project, []);
      map.get(s.project)!.push(s);
    }
    return map;
  });

  onMount(refreshServices);

  async function stopService(name: string, project: string) {
    try {
      await api.stopNativeService(name, project);
      await refreshServices();
    } catch (e) {
      console.error('Failed to stop service', e);
    }
  }
</script>

<div class="page">
  <header class="page-header">
    <h1>Native Services</h1>
  </header>

  {#if $services.length === 0}
    <EmptyState title="No services running" description="Start a project with native service dependencies" />
  {:else}
    {#each [...grouped.entries()] as [project, svcs]}
      <div class="service-group">
        <h2 class="group-title">{project}</h2>
        {#each svcs as svc}
          <div class="crush-card svc-card">
            <div class="svc-header">
              <StatusBadge status="running" />
              <span class="svc-name">{svc.name}</span>
              <span class="svc-kind">{svc.kind}</span>
            </div>
            <div class="svc-details">
              Port <span class="mono">{svc.port}</span>
              · Data <span class="mono">{svc.data_dir}</span>
            </div>
            {#if svc.connection_string}
              <div class="svc-conn">
                <span class="conn-label">Connection string</span>
                <CopyField value={svc.connection_string} />
              </div>
            {/if}
            <button class="stop-btn" onclick={() => stopService(svc.name, svc.project)}><Icon name="stop" size={12} fill /> Stop</button>
          </div>
        {/each}
      </div>
    {/each}
  {/if}
</div>

<style>
  .page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
  .page-header h1 { font-size: 20px; font-weight: 600; margin: 0; }

  .service-group { margin-bottom: 24px; }
  .group-title { font-size: 13px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); margin: 0 0 12px; }

  .svc-card { padding: 20px; margin-bottom: 12px; }
  .svc-header { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
  .svc-name { font-size: 16px; font-weight: 600; flex: 1; }
  .svc-kind { font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); }

  .svc-details { font-size: 13px; color: var(--color-crush-text-muted); margin-bottom: 12px; }
  .mono { font-family: var(--font-mono); }

  .svc-conn { margin-bottom: 12px; }
  .conn-label { font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--color-crush-text-muted); display: block; margin-bottom: 4px; }

  .stop-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: #ef4444; background: none; border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; padding: 6px 16px; cursor: pointer; }
  .stop-btn:hover { background: rgba(239,68,68,0.1); }
</style>
