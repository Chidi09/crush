<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import * as api from '$lib/tauri';
  import type { SshHost } from '$lib/tauri';

  let hosts = $state<SshHost[]>([]);
  let loading = $state(true);
  let connecting = $state<string | null>(null);
  let err = $state<string | null>(null);

  function target(h: SshHost): string {
    const host = h.hostname ?? h.alias;
    const wu = h.user ? `${h.user}@${host}` : host;
    return h.port && h.port !== 22 ? `${wu}:${h.port}` : wu;
  }

  async function load() {
    loading = true;
    try { hosts = await api.sshHosts(); }
    catch (e) { err = String(e); hosts = []; }
    finally { loading = false; }
  }
  async function connect(alias: string) {
    connecting = alias; err = null;
    try { await api.sshConnect(alias); }
    catch (e) { err = String(e); }
    finally { connecting = null; }
  }
  onMount(load);
</script>

<div class="page">
  <header class="page-head">
    <div class="title"><Icon name="server" size={18} /><h1>Servers</h1>{#if hosts.length}<span class="count">{hosts.length}</span>{/if}</div>
    <button class="ghost-btn" onclick={load} title="Refresh"><Icon name="refresh" size={14} /> Refresh</button>
  </header>
  <p class="sub">Hosts from your <code>~/.ssh/config</code>. Connect opens a terminal running <code>ssh</code> (your keys, ports, and ProxyJump are honored).</p>

  {#if err}<div class="err">{err}</div>{/if}

  {#if loading}
    <p class="muted">Loading…</p>
  {:else if !hosts.length}
    <EmptyState
      title="No SSH hosts found"
      description="Add hosts to ~/.ssh/config (Host alias / HostName / User / Port) and they'll show up here ready to connect."
    />
  {:else}
    <div class="grid">
      {#each hosts as h (h.alias)}
        <div class="card">
          <div class="card-top">
            <span class="host-icon"><Icon name="server" size={16} /></span>
            <div class="host-text">
              <span class="alias">{h.alias}</span>
              <span class="target">{target(h)}</span>
            </div>
          </div>
          <button class="connect-btn" disabled={connecting !== null} onclick={() => connect(h.alias)}>
            {connecting === h.alias ? 'opening…' : 'Connect'}
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page { padding: 20px 24px; max-width: 1100px; margin: 0 auto; }
  .page-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
  .title { display: flex; align-items: center; gap: 9px; }
  .title h1 { font-size: 18px; font-weight: 600; margin: 0; }
  .count { font-size: 12px; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 9999px; padding: 1px 9px; color: var(--color-crush-text-muted); }
  .sub { font-size: 13px; color: var(--color-crush-text-muted); margin: 0 0 18px; line-height: 1.6; }
  .sub code { font-family: var(--font-mono); font-size: 12px; }
  .ghost-btn { display: inline-flex; align-items: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.7rem; padding: 5px 11px; font-size: 13px; cursor: pointer; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .err { color: var(--color-crush-red); font-size: 13px; margin-bottom: 14px; }
  .muted { color: var(--color-crush-text-muted); font-size: 13px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
  .card { border: 1px solid var(--color-crush-border); border-radius: 12px; background: var(--color-crush-surface); padding: 14px; display: flex; flex-direction: column; gap: 12px; }
  .card-top { display: flex; align-items: center; gap: 10px; }
  .host-icon { display: inline-flex; width: 34px; height: 34px; align-items: center; justify-content: center; border-radius: 9px; background: rgba(99,102,241,0.08); border: 1px solid rgba(99,102,241,0.2); color: #a5b4fc; flex-shrink: 0; }
  .host-text { display: flex; flex-direction: column; min-width: 0; }
  .alias { font-weight: 600; font-size: 14px; }
  .target { font-size: 12px; color: var(--color-crush-text-muted); font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .connect-btn { width: 100%; font-size: 13px; color: var(--color-crush-text); background: rgba(99,102,241,0.1); border: 1px solid rgba(99,102,241,0.3); border-radius: 8px; padding: 7px; cursor: pointer; transition: background 0.15s, border-color 0.15s; }
  .connect-btn:hover:not(:disabled) { background: rgba(99,102,241,0.18); border-color: rgba(99,102,241,0.5); }
  .connect-btn:disabled { opacity: 0.5; cursor: default; }
</style>
