<script lang="ts">
  // Tiny inline area sparkline (CoinSpace-style). Stretches to fill width;
  // crisp stroke via non-scaling-stroke. Feed it a rolling number[].
  let {
    data = [],
    color = 'var(--color-crush-text)',
    height = 40
  }: { data?: number[]; color?: string; height?: number } = $props();

  const gid = 'spark-' + Math.random().toString(36).slice(2, 9);

  let geo = $derived(build(data, height));

  function build(d: number[], h: number): { line: string; area: string } {
    if (!d || d.length < 2) {
      const y = (h / 2).toFixed(1);
      return { line: `0,${y} 100,${y}`, area: `0,${y} 100,${y} 100,${h} 0,${h}` };
    }
    const min = Math.min(...d);
    const max = Math.max(...d);
    const range = max - min || 1;
    const step = 100 / (d.length - 1);
    const coords = d.map((v, i) => `${(i * step).toFixed(1)},${(h - ((v - min) / range) * (h - 6) - 3).toFixed(1)}`);
    const line = coords.join(' ');
    return { line, area: `${line} 100,${h} 0,${h}` };
  }
</script>

<svg class="spark" viewBox="0 0 100 {height}" preserveAspectRatio="none" height={height} aria-hidden="true">
  <defs>
    <linearGradient id={gid} x1="0" x2="0" y1="0" y2="1">
      <stop offset="0%" stop-color={color} stop-opacity="0.22" />
      <stop offset="100%" stop-color={color} stop-opacity="0" />
    </linearGradient>
  </defs>
  <polygon points={geo.area} fill="url(#{gid})" />
  <polyline points={geo.line} fill="none" stroke={color} stroke-width="1.5" vector-effect="non-scaling-stroke" stroke-linejoin="round" />
</svg>

<style>
  .spark { display: block; width: 100%; }
</style>
