<script lang="ts">
  let {
    data = [],
    width = 300,
    height = 100,
    color = "var(--viz-1)",
  }: { data?: number[]; width?: number; height?: number; color?: string } = $props();

  let max = $derived(Math.max(...data, 1));
  let points = $derived(
    data
      .map((d, i) => {
        const x = (i / (data.length - 1 || 1)) * width;
        const y = height - (d / max) * height;
        return `${x},${y}`;
      })
      .join(" ")
  );
  let path = $derived(`M 0,${height} L ${points} L ${width},${height} Z`);
</script>

<div class="relative group" style="width: {width}px; height: {height}px;">
  <svg width={width} height={height} class="overflow-visible">
    <defs>
      <linearGradient id="gradient-{color}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color={color} stop-opacity="0.4" />
        <stop offset="100%" stop-color={color} stop-opacity="0.0" />
      </linearGradient>
    </defs>
    {#if data.length > 0}
      <path d={path} fill="url(#gradient-{color})" class="transition-all duration-300" />
      <polyline points={points} fill="none" stroke={color} stroke-width="2" class="transition-all duration-300" />
    {/if}
  </svg>
</div>
