<script lang="ts">
  let {
    status = 'neutral',
    pulse = false,
  }: { status?: 'success' | 'warn' | 'error' | 'info' | 'neutral'; pulse?: boolean } = $props();

  const colors: Record<string, string> = {
    success: 'var(--status-success)',
    warn: 'var(--status-warn)',
    error: 'var(--status-error)',
    info: 'var(--status-info)',
    neutral: 'var(--status-neutral)',
  };
</script>

<div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full border bg-surface text-xs font-medium" style="border-color: {colors[status]}; color: {colors[status]};">
  {#if pulse}
    <div class="relative flex h-2 w-2">
      <span class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-75" style="background-color: {colors[status]};"></span>
      <span class="relative inline-flex rounded-full h-2 w-2" style="background-color: {colors[status]};"></span>
    </div>
  {:else}
    <div class="h-1.5 w-1.5 rounded-full" style="background-color: {colors[status]};"></div>
  {/if}
  <slot />
</div>
