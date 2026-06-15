<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as api from '$lib/tauri';
  import type { DomainRecord } from '$lib/tauri';

  let domains = $state<DomainRecord[]>([]);
  let loading = $state(true);
  
  let showAdd = $state(false);
  let newHost = $state('');
  let newProject = $state('');
  let newPort = $state('');

  onMount(load);

  async function load() {
    loading = true;
    try {
      domains = await api.listDomains();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  async function addDomain() {
    if (!newHost || !newProject || !newPort) return;
    try {
      await api.addDomain(newHost, newProject, parseInt(newPort, 10));
      showAdd = false;
      newHost = '';
      newProject = '';
      newPort = '';
      await load();
    } catch (e) {
      alert(`Add failed: ${String(e)}`);
    }
  }

  async function removeDomain(host: string) {
    if (!confirm(`Remove domain ${host}?`)) return;
    try {
      await api.removeDomain(host);
      await load();
    } catch (e) {
      alert(`Remove failed: ${String(e)}`);
    }
  }
</script>

<div class="page">
  <header class="ph">
    <h1>Domains</h1>
    <div class="ph-actions">
      <button class="btn primary" onclick={() => showAdd = true}>
        <Icon name="globe" size={14} /> Add Domain
      </button>
      <button class="ghost-btn" onclick={load} title="Refresh"><Icon name="refresh" size={14} /></button>
    </div>
  </header>

  {#if showAdd}
    <div class="crush-card add-card">
      <div class="form-row">
        <label>
          <span>Hostname</span>
          <input type="text" bind:value={newHost} placeholder="e.g. myapp.crush.local" />
        </label>
        <label>
          <span>Project</span>
          <input type="text" bind:value={newProject} placeholder="e.g. frontend" />
        </label>
        <label>
          <span>Target Port</span>
          <input type="number" bind:value={newPort} placeholder="e.g. 3000" />
        </label>
      </div>
      <div class="form-actions">
        <button class="ghost-btn" onclick={() => showAdd = false}>Cancel</button>
        <button class="btn primary" onclick={addDomain}>Save Domain</button>
      </div>
    </div>
  {/if}

  {#if loading}
    <p class="muted">Loading domains...</p>
  {:else if domains.length === 0}
    <div class="empty-box">
      <Icon name="globe" size={26} />
      <p class="empty-title">No domains mapped</p>
      <p class="muted">Add a domain to route traffic through the L7 gateway to your deployed apps.</p>
    </div>
  {:else}
    <div class="ctable">
      <div class="crow chead">
        <span>Hostname</span>
        <span>Project</span>
        <span>Target Port</span>
        <span>Actions</span>
      </div>
      {#each domains as d}
        <div class="crow">
          <span class="cname mono">{d.host}</span>
          <span class="cname">{d.project}</span>
          <span class="mono dim">{d.port}</span>
          <div class="actions">
            <button class="ghost-btn sm text-red" onclick={() => removeDomain(d.host)} title="Remove Domain">
              <Icon name="trash" size={14} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  .ph { display: flex; align-items: center; justify-content: space-between; }
  .ph h1 { font-size: 20px; font-weight: 600; margin: 0; }
  .ph-actions { display: flex; align-items: center; gap: 12px; }
  
  .ghost-btn { display: inline-flex; align-items: center; justify-content: center; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 8px; padding: 7px 10px; cursor: pointer; gap: 6px;}
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .btn { display: inline-flex; align-items: center; justify-content: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 8px; padding: 7px 12px; font-size: 13px; cursor: pointer; }
  .btn.primary { background: var(--color-crush-primary); border-color: var(--color-crush-primary); color: var(--color-crush-on-primary); font-weight: 500; }
  .btn.primary:hover:not(:disabled) { background: var(--color-crush-primary-hover); }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }

  .add-card { display: flex; flex-direction: column; gap: 16px; padding: 20px; }
  .form-row { display: grid; grid-template-columns: 2fr 2fr 1fr; gap: 12px; }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; color: var(--color-crush-text); }
  input { background: rgba(0,0,0,0.2); border: 1px solid var(--color-crush-border); color: var(--color-crush-text); border-radius: 6px; padding: 8px 12px; font-size: 13px; outline: none; }
  input:focus { border-color: var(--color-crush-primary); }
  .form-actions { display: flex; justify-content: flex-end; gap: 8px; }

  .empty-box { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 48px 24px; color: var(--color-crush-text-muted); border: 1px dashed var(--color-crush-border); border-radius: 0.75rem; }
  .empty-box .empty-title { font-size: 15px; font-weight: 600; color: var(--color-crush-text); margin: 4px 0 0; }
  
  .ctable { display: flex; flex-direction: column; font-size: 13px; border: 1px solid var(--color-crush-border); border-radius: 8px; background: var(--color-crush-surface); overflow: hidden; }
  .crow { display: grid; grid-template-columns: 2fr 1.5fr 1fr auto; align-items: center; padding: 10px 16px; border-bottom: 1px solid var(--color-crush-border); gap: 12px; }
  .crow:last-child { border-bottom: none; }
  .chead { background: rgba(0,0,0,0.2); font-weight: 500; color: var(--color-crush-text-muted); font-size: 12px; text-transform: uppercase; letter-spacing: 0.05em; border-bottom: 1px solid var(--color-crush-border); }
  .cname { color: var(--color-crush-text); font-weight: 500; }
  .mono { font-family: var(--font-mono); }
  .dim { color: var(--color-crush-text-muted); }
  .actions { display: flex; gap: 8px; align-items: center; justify-content: flex-end; }
  .sm { padding: 4px 8px; font-size: 12px; }
  .text-red { color: var(--color-crush-red); }
  .text-red:hover { border-color: rgba(255,100,100,0.3); background: rgba(255,100,100,0.1); }
</style>
