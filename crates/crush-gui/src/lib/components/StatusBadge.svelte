<script lang="ts">
  let { status = 'unknown' }: { status?: string } = $props();
  $effect(() => {});
</script>

<span
  class="status-badge"
  class:running={status === 'running'}
  class:exited={status === 'exited' || status === 'stopped'}
  class:paused={status === 'paused'}
>
  <span class="dot" class:dot-running={status === 'running'} class:dot-exited={status !== 'running'}></span>
  {status}
</span>

<style>
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 10px;
    border-radius: 9999px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .running {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
    border: 1px solid rgba(74, 222, 128, 0.2);
  }

  .exited {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .paused {
    background: rgba(234, 179, 8, 0.1);
    color: #eab308;
    border: 1px solid rgba(234, 179, 8, 0.2);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .dot-running {
    background: #4ade80;
    box-shadow: 0 0 6px rgba(74, 222, 128, 0.5);
  }

  .dot-exited {
    background: currentColor;
    opacity: 0.5;
  }
</style>
