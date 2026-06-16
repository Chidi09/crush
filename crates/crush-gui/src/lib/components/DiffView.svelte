<script lang="ts">
  let { left = "", right = "" }: { left?: string; right?: string } = $props();

  // Minimal naive line diff for visual structural preview.
  let leftLines = $derived(left.split('\n'));
  let rightLines = $derived(right.split('\n'));

  let diffs = $derived.by(() => {
    const result: { type: 'equal' | 'removed' | 'added'; text: string }[] = [];
    const max = Math.max(leftLines.length, rightLines.length);
    for (let i = 0; i < max; i++) {
      if (leftLines[i] === rightLines[i]) {
        result.push({ type: 'equal', text: leftLines[i] ?? '' });
      } else {
        if (leftLines[i] !== undefined) result.push({ type: 'removed', text: leftLines[i] });
        if (rightLines[i] !== undefined) result.push({ type: 'added', text: rightLines[i] });
      }
    }
    return result;
  });
</script>

<div class="font-mono text-xs bg-surface border border-[var(--border)] rounded-md overflow-auto crush-card">
  <table class="w-full text-left border-collapse">
    <tbody>
      {#each diffs as line}
        <tr
          class:bg-red-900={line.type === 'removed'}
          class:bg-green-900={line.type === 'added'}
          class:bg-opacity-20={line.type === 'removed' || line.type === 'added'}
        >
          <td class="w-6 px-2 py-0.5 text-right opacity-50 select-none border-r border-[var(--border)]">
            {line.type === 'added' ? '+' : line.type === 'removed' ? '-' : ' '}
          </td>
          <td class="px-4 py-0.5 whitespace-pre break-all font-mono"
            class:text-red-400={line.type === 'removed'}
            class:text-green-400={line.type === 'added'}
            class:text-text={line.type === 'equal'}
          >
            {line.text}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
