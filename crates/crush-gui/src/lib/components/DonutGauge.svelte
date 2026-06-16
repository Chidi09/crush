<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let {
    value = 0,
    size = 64,
    strokeWidth = 6,
    color = "var(--viz-2)",
  }: { value?: number; size?: number; strokeWidth?: number; color?: string } = $props();

  const progress = new Tween(0, { duration: 600, easing: cubicOut });
  $effect(() => { progress.target = value; });

  let radius = $derived((size - strokeWidth) / 2);
  let circumference = $derived(radius * 2 * Math.PI);
  let offset = $derived(circumference - (progress.current / 100) * circumference);
</script>

<div class="relative flex items-center justify-center" style="width: {size}px; height: {size}px;">
  <svg width={size} height={size} class="transform -rotate-90">
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      stroke="var(--border)"
      stroke-width={strokeWidth}
      fill="none"
    />
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      stroke={color}
      stroke-width={strokeWidth}
      stroke-linecap="round"
      fill="none"
      stroke-dasharray={circumference}
      stroke-dashoffset={offset}
      class="transition-all"
    />
  </svg>
  <div class="absolute text-xs font-mono font-medium" style="color: {color}">
    {Math.round(progress.current)}%
  </div>
</div>
