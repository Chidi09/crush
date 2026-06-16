<script lang="ts">
  let {
    logs = [],
    autoScroll = $bindable(true),
    showHeader = true,
  }: {
    logs?: { id: string; text: string; severity?: 'info' | 'warn' | 'error' }[];
    autoScroll?: boolean;
    showHeader?: boolean;
  } = $props();

  let container: HTMLElement | undefined = $state();

  function getSeverityColor(sev?: string) {
    if (sev === 'error') return 'text-[var(--status-error)]';
    if (sev === 'warn') return 'text-[var(--status-warn)]';
    return 'text-text-muted';
  }

  // Re-run on new log entries; keep the view pinned to the tail when following.
  $effect(() => {
    logs.length;
    if (autoScroll && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div class="relative bg-black border border-[var(--border)] rounded-lg font-mono text-xs overflow-hidden flex flex-col h-full crush-card">
  {#if showHeader}
    <div class="p-2 border-b border-[var(--border)] flex justify-between bg-surface-raised items-center text-text-muted">
      <span class="flex items-center gap-2">
        <div class="w-2 h-2 rounded-full bg-[var(--status-success)] animate-pulse"></div>
        Live Logs
      </span>
      <label class="flex items-center gap-1 cursor-pointer hover:text-white">
        <input type="checkbox" bind:checked={autoScroll} class="accent-[var(--crush-orange)]"/> Follow
      </label>
    </div>
  {/if}
  <div bind:this={container} class="flex-1 overflow-auto p-2 space-y-1">
    {#each logs as log (log.id)}
      <div class="break-all whitespace-pre-wrap {getSeverityColor(log.severity)}">
        <span class="opacity-50 mr-2">{log.id.substring(0, 6)}</span>
        <span class="text-text">{log.text}</span>
      </div>
    {/each}
  </div>
</div>
