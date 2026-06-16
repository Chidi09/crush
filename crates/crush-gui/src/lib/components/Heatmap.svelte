<script lang="ts">
  let {
    data = [],
    colorBase = "14, 165, 233",
  }: { data?: number[][]; colorBase?: string } = $props();

  let max = $derived(Math.max(...data.flat(), 1));
</script>

<div class="flex flex-col gap-1">
  {#each data as row}
    <div class="flex gap-1">
      {#each row as cell}
        <div
          class="w-3 h-3 rounded-sm"
          style="background-color: rgba({colorBase}, {cell / max});"
          title="Value: {cell}"
        ></div>
      {/each}
    </div>
  {/each}
</div>
