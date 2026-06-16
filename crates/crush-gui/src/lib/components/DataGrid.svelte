<script lang="ts">
  let {
    columns = [],
    rows = [],
  }: {
    columns?: { key: string; label: string; width?: string; frozen?: boolean }[];
    rows?: any[];
  } = $props();
</script>

<div class="overflow-auto border border-[var(--border)] rounded-md bg-surface crush-card w-full max-h-[600px] relative">
  <table class="w-full text-left text-sm whitespace-nowrap border-collapse">
    <thead class="sticky top-0 bg-surface-raised z-10 shadow-[var(--inner-glow)] text-text-muted">
      <tr>
        {#each columns as col}
          <th
            class="px-3 py-2 border-b border-[var(--border)] font-medium"
            class:sticky={col.frozen}
            class:left-0={col.frozen}
            class:bg-surface-raised={col.frozen}
            class:z-20={col.frozen}
            style="width: {col.width || 'auto'};"
          >
            {col.label}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody class="divide-y divide-[var(--border)]">
      {#each rows as row}
        <tr class="hover:bg-surface-hover group transition-colors duration-150 cursor-default">
          {#each columns as col}
            <td
              class="px-3 py-2"
              class:sticky={col.frozen}
              class:left-0={col.frozen}
              class:bg-surface={col.frozen}
              class:group-hover:bg-surface-hover={col.frozen}
            >
              <slot name="cell" {row} {col}>
                <span class="text-text">{row[col.key]}</span>
              </slot>
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
