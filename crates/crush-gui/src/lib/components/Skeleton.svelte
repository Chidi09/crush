<script lang="ts">
  let {
    width = "100%",
    height = "1rem",
    circle = false,
  }: { width?: string; height?: string; circle?: boolean } = $props();
</script>

<div
  class="relative overflow-hidden bg-surface-raised"
  style="width: {width}; height: {height}; border-radius: {circle ? '50%' : 'var(--radius-md)'};"
>
  <div class="absolute inset-0 -translate-x-full animate-[shimmer_1.5s_infinite] bg-gradient-to-r from-transparent via-white/5 to-transparent"></div>
</div>

<style>
  @keyframes shimmer {
    100% {
      transform: translateX(100%);
    }
  }
</style>
