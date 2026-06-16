<script lang="ts">
  let {
    value = 0,
    size = 40,
    strokeWidth = 4,
    color = "var(--crush-orange)",
  }: { value?: number; size?: number; strokeWidth?: number; color?: string } = $props();

  let radius = $derived((size - strokeWidth) / 2);
  let circumference = $derived(radius * 2 * Math.PI);
  let offset = $derived(circumference - (value / 100) * circumference);
</script>

<div class="relative inline-flex items-center justify-center" style="width: {size}px; height: {size}px;">
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
      class="transition-all duration-300 ease-out"
    />
  </svg>
</div>
