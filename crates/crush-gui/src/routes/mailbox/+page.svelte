<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import * as api from '$lib/tauri';
  import type { CapturedMail } from '$lib/tauri';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  let mail = $state<CapturedMail[]>([]);
  let selectedId = $state<number | null>(null);
  let showRaw = $state(false);
  let unlisten: UnlistenFn | null = null;

  let selected = $derived(mail.find((m) => m.id === selectedId) ?? null);

  async function refresh() {
    mail = await api.listMail();
    // Keep a selection: default to newest if none/stale.
    if (mail.length && (selectedId === null || !mail.some((m) => m.id === selectedId))) {
      selectedId = mail[0].id;
    }
    if (!mail.length) selectedId = null;
  }

  async function clearAll() {
    await api.clearMail();
    await refresh();
  }

  function fmtTime(ms: number): string {
    try { return new Date(ms).toLocaleString(); } catch { return ''; }
  }

  onMount(async () => {
    await refresh();
    unlisten = await api.onMailReceived(() => refresh());
  });
  onDestroy(() => { unlisten?.(); });
</script>

<div class="page">
  <div class="page-head">
    <div class="title">
      <Icon name="mail" size={18} />
      <h1>Mailbox</h1>
      {#if mail.length}<span class="count">{mail.length}</span>{/if}
    </div>
    <div class="actions">
      <span class="sink-note"><span class="dot"></span> SMTP sink on <code>localhost:1025</code></span>
      <button class="ghost-btn" onclick={refresh} title="Refresh"><Icon name="refresh" size={14} /> Refresh</button>
      <button class="ghost-btn danger" onclick={clearAll} disabled={!mail.length}><Icon name="trash" size={14} /> Clear</button>
    </div>
  </div>

  {#if !mail.length}
    <EmptyState
      title="No captured email yet"
      description="Run an app with crush and any mail it sends (signup, reset, receipts) lands here instead of being delivered. Point your app at SMTP_HOST=localhost / SMTP_PORT=1025 — crush injects these automatically on run."
    />
  {:else}
    <div class="split">
      <div class="list">
        {#each mail as m (m.id)}
          <button
            class="row"
            class:active={m.id === selectedId}
            onclick={() => { selectedId = m.id; showRaw = false; }}
          >
            <div class="row-top">
              <span class="subj">{m.subject || '(no subject)'}</span>
              <span class="time">{fmtTime(m.received_ms)}</span>
            </div>
            <div class="row-meta">
              <span class="from">{m.from}</span>
              <span class="arrow">→</span>
              <span class="to">{m.to.join(', ')}</span>
            </div>
          </button>
        {/each}
      </div>

      <div class="detail">
        {#if selected}
          <div class="detail-head">
            <h2>{selected.subject || '(no subject)'}</h2>
            <div class="kv"><span class="k">From</span><span class="v">{selected.from}</span></div>
            <div class="kv"><span class="k">To</span><span class="v">{selected.to.join(', ')}</span></div>
            {#if selected.date}<div class="kv"><span class="k">Date</span><span class="v">{selected.date}</span></div>{/if}
            <div class="toggle">
              <button class="seg" class:on={!showRaw} onclick={() => showRaw = false}>Body</button>
              <button class="seg" class:on={showRaw} onclick={() => showRaw = true}>Raw</button>
            </div>
          </div>
          <pre class="body">{showRaw ? selected.raw : (selected.body || '(empty body)')}</pre>
        {:else}
          <div class="pick">Select a message</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .page { padding: 20px 24px; max-width: 1200px; margin: 0 auto; }
  .page-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; gap: 12px; flex-wrap: wrap; }
  .title { display: flex; align-items: center; gap: 9px; }
  .title h1 { font-size: 18px; font-weight: 600; margin: 0; }
  .count { font-size: 12px; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 9999px; padding: 1px 9px; color: var(--color-crush-text-muted); }
  .actions { display: flex; align-items: center; gap: 10px; }
  .sink-note { font-size: 12px; color: var(--color-crush-text-muted); display: inline-flex; align-items: center; gap: 6px; }
  .sink-note code { font-family: var(--font-mono); font-size: 11.5px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: #4ade80; box-shadow: 0 0 6px rgba(74,222,128,0.6); }
  .ghost-btn { display: inline-flex; align-items: center; gap: 6px; background: none; border: 1px solid var(--color-crush-border); color: var(--color-crush-text-muted); border-radius: 0.7rem; padding: 5px 11px; font-size: 13px; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .ghost-btn:hover { color: var(--color-crush-text); border-color: var(--color-crush-muted); }
  .ghost-btn:disabled { opacity: 0.4; cursor: default; }
  .ghost-btn.danger { color: var(--color-crush-red); border-color: rgba(239,68,68,0.3); }
  .ghost-btn.danger:hover:not(:disabled) { background: rgba(239,68,68,0.08); border-color: rgba(239,68,68,0.5); }

  .split { display: grid; grid-template-columns: 340px 1fr; gap: 14px; height: calc(100vh - 130px); }
  .list { overflow-y: auto; display: flex; flex-direction: column; gap: 6px; padding-right: 4px; }
  .row { text-align: left; background: var(--color-crush-surface); border: 1px solid var(--color-crush-border); border-radius: 10px; padding: 10px 12px; cursor: pointer; transition: border-color 0.15s, background 0.15s; }
  .row:hover { border-color: var(--color-crush-muted); }
  .row.active { border-color: #6366f1; background: rgba(99,102,241,0.06); }
  .row-top { display: flex; justify-content: space-between; gap: 8px; align-items: baseline; }
  .subj { font-weight: 600; font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .time { font-size: 11px; color: var(--color-crush-text-muted); flex-shrink: 0; }
  .row-meta { display: flex; gap: 6px; font-size: 12px; color: var(--color-crush-text-muted); margin-top: 3px; overflow: hidden; }
  .row-meta .to, .row-meta .from { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .arrow { flex-shrink: 0; }

  .detail { border: 1px solid var(--color-crush-border); border-radius: 12px; background: var(--color-crush-surface); display: flex; flex-direction: column; overflow: hidden; }
  .detail-head { padding: 16px 18px; border-bottom: 1px solid var(--color-crush-border); }
  .detail-head h2 { font-size: 16px; margin: 0 0 10px; }
  .kv { display: flex; gap: 10px; font-size: 13px; margin-bottom: 3px; }
  .kv .k { color: var(--color-crush-text-muted); width: 46px; flex-shrink: 0; }
  .kv .v { font-family: var(--font-mono); font-size: 12.5px; }
  .toggle { margin-top: 12px; display: inline-flex; border: 1px solid var(--color-crush-border); border-radius: 8px; overflow: hidden; }
  .seg { background: none; border: none; color: var(--color-crush-text-muted); padding: 4px 14px; font-size: 12.5px; cursor: pointer; }
  .seg.on { background: rgba(255,255,255,0.06); color: var(--color-crush-text); }
  .body { flex: 1; overflow: auto; margin: 0; padding: 16px 18px; font-family: var(--font-mono); font-size: 12.5px; line-height: 1.6; white-space: pre-wrap; word-break: break-word; color: var(--color-crush-text); }
  .pick { display: flex; align-items: center; justify-content: center; height: 100%; color: var(--color-crush-text-muted); }
</style>
