<script lang="ts">
  import Self from './JsonTree.svelte';

  let {
    data,
    expanded = true,
    name = "",
  }: { data?: any; expanded?: boolean; name?: string } = $props();

  let isExpanded = $state(expanded);

  const isObject = (val: any) => val !== null && typeof val === 'object';
  const isArray = (val: any) => Array.isArray(val);

  function toggle() {
    isExpanded = !isExpanded;
  }
</script>

<div class="font-mono text-xs text-text leading-tight">
  {#if isObject(data)}
    <div class="flex items-start">
      <button class="w-4 hover:text-white" onclick={toggle}>
        {isExpanded ? '▼' : '▶'}
      </button>
      <div class="flex-1">
        {#if name}
          <span class="text-crush-orange">{name}</span>:
        {/if}
        <span class="text-text-muted">{isArray(data) ? '[' : '{'}</span>
        {#if !isExpanded}
          <span class="text-text-muted">...{isArray(data) ? ']' : '}'}</span>
        {:else}
          <div class="pl-4 border-l border-[var(--border)] ml-1 my-1">
            {#each Object.entries(data) as [k, v]}
              <Self data={v} name={isArray(data) ? "" : k} />
            {/each}
          </div>
          <span class="text-text-muted">{isArray(data) ? ']' : '}'}</span>
        {/if}
      </div>
    </div>
  {:else}
    <div class="pl-4 py-0.5">
      {#if name}
        <span class="text-crush-orange">{name}</span>:
      {/if}
      {#if typeof data === 'string'}
        <span class="text-[var(--status-success)]">"{data}"</span>
      {:else if typeof data === 'number'}
        <span class="text-[var(--viz-5)]">{data}</span>
      {:else if typeof data === 'boolean'}
        <span class="text-[var(--viz-1)]">{data}</span>
      {:else if data === null}
        <span class="text-text-muted italic">null</span>
      {:else}
        <span>{data}</span>
      {/if}
    </div>
  {/if}
</div>
