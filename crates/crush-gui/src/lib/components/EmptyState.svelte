<script lang="ts">
  let { title = 'Nothing here', description = '', action, onAction }: {
    title?: string;
    description?: string;
    action?: string;
    onAction?: () => void;
  } = $props();
</script>

<div class="empty-state">
  <svg class="icon" width="32" height="32" viewBox="0 0 32 32" aria-hidden="true">
    <path d="M16 3 L28 16 L16 29 L4 16 Z" fill="currentColor" />
  </svg>
  <h3>{title}</h3>
  {#if description}
    <p>{description}</p>
  {/if}
  {#if action}
    <button class="action-btn" onclick={onAction}>
      {action}
    </button>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    text-align: center;
  }

  .icon {
    font-size: 32px;
    color: var(--color-crush-muted);
    margin-bottom: 12px;
  }

  h3 {
    font-size: 16px;
    font-weight: 600;
    color: var(--color-crush-text);
    margin: 0 0 4px;
  }

  p {
    font-size: 13px;
    color: var(--color-crush-text-muted);
    margin: 0 0 16px;
    max-width: 300px;
  }

  .action-btn {
    background: var(--color-crush-primary);
    color: var(--color-crush-on-primary);
    border: none;
    border-radius: 8px;
    padding: 8px 20px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }

  .action-btn:hover {
    background: var(--color-crush-primary-hover);
  }
</style>
