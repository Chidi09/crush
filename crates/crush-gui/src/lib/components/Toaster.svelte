<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { toaster } from '$lib/stores/toast.svelte.ts';

  const ICON: Record<string, string> = { success: 'check', error: 'stop', info: 'sparkles' };
</script>

<div class="toaster" aria-live="polite">
  {#each toaster.items as t (t.id)}
    <div
      class="toast {t.kind}"
      in:fly={{ y: 14, duration: 220, easing: cubicOut }}
      out:fade={{ duration: 140 }}
    >
      <span class="t-icon"><Icon name={ICON[t.kind]} size={13} fill={t.kind !== 'info'} /></span>
      <span class="t-text">{t.text}</span>
      <button class="t-close" onclick={() => toaster.dismiss(t.id)} aria-label="Dismiss">×</button>
    </div>
  {/each}
</div>

<style>
  .toaster {
    position: fixed;
    bottom: 18px;
    right: 18px;
    z-index: 200;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 220px;
    max-width: 380px;
    padding: 10px 12px;
    border-radius: 10px;
    font-size: 13px;
    color: var(--color-crush-text);
    background: var(--color-crush-dark);
    border: 1px solid var(--color-crush-border);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
  }
  .t-icon { display: inline-flex; flex-shrink: 0; }
  .toast.success { border-color: rgba(16, 185, 129, 0.35); }
  .toast.success .t-icon { color: var(--color-crush-green); }
  .toast.error { border-color: rgba(239, 68, 68, 0.35); }
  .toast.error .t-icon { color: var(--color-crush-red); }
  .toast.info .t-icon { color: var(--color-crush-text); }
  .t-text { flex: 1; line-height: 1.4; }
  .t-close { background: none; border: none; color: var(--color-crush-text-muted); font-size: 16px; line-height: 1; cursor: pointer; flex-shrink: 0; }
  .t-close:hover { color: var(--color-crush-text); }

  @media (prefers-reduced-motion: reduce) {
    .toast { transition: none; }
  }
</style>
