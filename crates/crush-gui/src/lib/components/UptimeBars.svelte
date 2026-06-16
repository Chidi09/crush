<script lang="ts">
  let {
    segments = [],
  }: { segments?: { status: 'up' | 'down' | 'degraded'; tooltip?: string }[] } = $props();

  function getColor(status: string) {
    if (status === 'up') return 'var(--status-success)';
    if (status === 'degraded') return 'var(--status-warn)';
    return 'var(--status-error)';
  }
</script>

<div class="flex items-center gap-px w-full h-8 group bg-surface-raised rounded-sm overflow-hidden">
  {#each segments as segment}
    <div
      class="flex-1 h-full hover:opacity-80 transition-opacity cursor-pointer"
      style="background-color: {getColor(segment.status)}"
      title={segment.tooltip || segment.status}
    ></div>
  {/each}
</div>
