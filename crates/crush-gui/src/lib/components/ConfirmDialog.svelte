<script lang="ts">
  // Global confirm/prompt modal, driven by the confirm store. Mounted once in
  // the root layout; every confirmAction()/promptInput() call renders here.
  import { confirmStore } from '$lib/stores/confirm.svelte.ts';
  import Icon from '$lib/components/Icon.svelte';
  import { fade, scale } from 'svelte/transition';

  let d = $derived(confirmStore.current);

  function onKey(e: KeyboardEvent) {
    if (!d) return;
    if (e.key === 'Escape') { e.preventDefault(); confirmStore.cancel(); }
    else if (e.key === 'Enter' && d.mode === 'prompt') { e.preventDefault(); confirmStore.accept(); }
  }

  function focusInput(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if d}
  <div class="cd-backdrop" role="presentation" onclick={() => confirmStore.cancel()} transition:fade={{ duration: 120 }}>
    <div
      class="cd-card"
      role="dialog"
      aria-modal="true"
      aria-label={d.title}
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      transition:scale={{ duration: 140, start: 0.96 }}
    >
      <div class="cd-head">
        <div class="cd-icon" class:danger={d.danger}>
          <Icon name={d.danger ? 'trash' : 'check'} size={18} />
        </div>
        <h3 class="cd-title">{d.title}</h3>
      </div>

      {#if d.message}
        <p class="cd-message">{d.message}</p>
      {/if}

      {#if d.mode === 'prompt'}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="cd-input"
          type="text"
          bind:value={confirmStore.input}
          placeholder={d.placeholder}
          use:focusInput
        />
      {/if}

      <div class="cd-actions">
        <button class="cd-btn cd-cancel" onclick={() => confirmStore.cancel()}>{d.cancelText}</button>
        <button class="cd-btn cd-confirm" class:danger={d.danger} onclick={() => confirmStore.accept()}>{d.confirmText}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .cd-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--overlay, rgba(0, 0, 0, 0.7));
    backdrop-filter: blur(2px);
    padding: 20px;
  }
  .cd-card {
    width: 100%;
    max-width: 420px;
    background: var(--surface, #141414);
    border: 1px solid var(--border-strong, #333);
    border-radius: 14px;
    box-shadow: var(--elevation-3, 0 16px 48px rgba(0, 0, 0, 0.5));
    padding: 22px;
  }
  .cd-head { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
  .cd-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-crush-text) 10%, transparent);
    color: var(--color-crush-text);
    flex-shrink: 0;
  }
  .cd-icon.danger {
    background: rgba(239, 68, 68, 0.12);
    color: var(--color-crush-red, #ef4444);
  }
  .cd-title { font-size: 16px; font-weight: 600; color: var(--text, #ededed); margin: 0; }
  .cd-message {
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text-muted, #8a8a93);
    margin: 0 0 18px;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .cd-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--surface-raised, #1e1e1e);
    border: 1px solid var(--border, #242424);
    color: var(--text, #ededed);
    border-radius: 8px;
    padding: 9px 12px;
    font-size: 14px;
    outline: none;
    margin-bottom: 18px;
  }
  .cd-input:focus { border-color: var(--color-crush-orange); }
  .cd-actions { display: flex; justify-content: flex-end; gap: 10px; }
  .cd-btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13.5px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.15s, border-color 0.15s, transform 0.08s;
  }
  .cd-btn:active { transform: scale(0.98); }
  .cd-cancel {
    background: none;
    border-color: var(--border-strong, #333);
    color: var(--text-muted, #8a8a93);
  }
  .cd-cancel:hover { color: var(--text, #ededed); background: var(--surface-hover, #2a2a2a); }
  .cd-confirm {
    background: var(--color-crush-primary, #ededed);
    color: var(--color-crush-on-primary, #000);
  }
  .cd-confirm:hover { background: var(--color-crush-primary-hover, #fff); }
  .cd-confirm.danger {
    background: var(--color-crush-red, #ef4444);
    color: #fff;
  }
  .cd-confirm.danger:hover { background: #dc2626; }
  @media (prefers-reduced-motion: reduce) {
    .cd-backdrop, .cd-card { transition: none; }
  }
</style>
