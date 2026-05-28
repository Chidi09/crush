<script lang="ts">
  import Icon from './Icon.svelte';

  let { value }: { value: string } = $props();
  let copied = $state(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {}
  }
</script>

<div class="copy-field">
  <code class="field-value">{value}</code>
  <button class="copy-btn" class:copied onclick={copy}>
    {#if copied}
      <Icon name="check" size={12} /> copied
    {:else}
      <Icon name="copy" size={12} /> Copy
    {/if}
  </button>
</div>

<style>
  .copy-field {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(9, 9, 11, 0.6);
    border: 1px solid var(--color-crush-border);
    border-radius: 0.5rem;
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .field-value {
    flex: 1;
    color: var(--color-crush-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .copy-btn {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid var(--color-crush-border);
    background: var(--color-crush-surface);
    color: var(--color-crush-text-muted);
    cursor: pointer;
    white-space: nowrap;
  }

  .copy-btn:hover,
  .copy-btn.copied {
    color: var(--color-crush-green);
    border-color: rgba(74, 222, 128, 0.3);
  }
</style>
