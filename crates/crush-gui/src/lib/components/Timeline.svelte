<script lang="ts">
  let {
    events = [],
  }: {
    events?: { id: string; title: string; timestamp: string; description?: string; status: 'success' | 'error' | 'pending' }[];
  } = $props();

  function getStatusColor(s: string) {
    if (s === 'success') return 'var(--status-success)';
    if (s === 'error') return 'var(--status-error)';
    return 'var(--status-warn)';
  }
</script>

<div class="relative border-l border-[var(--border)] ml-3 space-y-6">
  {#each events as event}
    <div class="relative pl-6">
      <div
        class="absolute -left-[5px] top-1.5 w-2.5 h-2.5 rounded-full border-2 border-surface"
        style="background-color: {getStatusColor(event.status)}"
      ></div>
      <div class="flex justify-between items-start mb-1">
        <div class="text-sm font-medium text-white">{event.title}</div>
        <div class="text-xs text-text-muted font-mono">{event.timestamp}</div>
      </div>
      {#if event.description}
        <div class="text-sm text-text-muted">{event.description}</div>
      {/if}
    </div>
  {/each}
</div>
