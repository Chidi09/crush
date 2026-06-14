<script lang="ts" module>
  // Deterministic identicon for OCI images. A crushed image has no inherent
  // logo, so we derive a stable, unique visual fingerprint from its digest —
  // GitHub-style: a 5×5 symmetric grid of coloured cells. Pure inline SVG so it
  // works under the GUI's CSP (img-src 'self' data:) with zero dependencies.
  // Same seed → same icon every time, so an image is recognisable at a glance.
  function fnv1a(str: string): number {
    let h = 0x811c9dc5;
    for (let i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 0x01000193);
    }
    return h >>> 0;
  }
</script>

<script lang="ts">
  let { seed, size = 40 }: { seed: string; size?: number } = $props();

  const GRID = 5;
  const HALF = Math.ceil(GRID / 2); // 3 columns, mirrored → 5

  let model = $derived.by(() => {
    const s = seed || '?';
    const a = fnv1a(s);
    const b = fnv1a(s + 'crush');
    const hue = a % 360;
    const sat = 58 + (b % 22);           // 58–80%
    const light = 56 + ((a >>> 9) % 14); // 56–70%
    const color = `hsl(${hue} ${sat}% ${light}%)`;

    // 64 bits of entropy → fill the left half + centre column, then mirror.
    const bits = (BigInt(a) << 32n) | BigInt(b);
    const on: boolean[][] = [];
    let idx = 0n;
    for (let r = 0; r < GRID; r++) {
      const row = new Array<boolean>(GRID).fill(false);
      for (let c = 0; c < HALF; c++) {
        const v = ((bits >> idx) & 1n) === 1n;
        idx += 1n;
        row[c] = v;
        row[GRID - 1 - c] = v;
      }
      on.push(row);
    }
    return { on, color };
  });

  let cell = $derived(size / (GRID + 1));
  let pad = $derived(cell / 2);
</script>

<svg width={size} height={size} viewBox="0 0 {size} {size}" role="img" aria-label="image icon">
  <rect width={size} height={size} rx={size * 0.2} fill="rgba(255,255,255,0.05)" />
  {#each model.on as row, r}
    {#each row as v, c}
      {#if v}
        <rect
          x={pad + c * cell}
          y={pad + r * cell}
          width={cell}
          height={cell}
          rx={cell * 0.2}
          fill={model.color}
        />
      {/if}
    {/each}
  {/each}
</svg>
