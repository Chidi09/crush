<script lang="ts">
  // Items may navigate via href (router) or via an onnavigate callback (in-page
  // state, e.g. an object-storage path). The last item renders as the current location.
  let {
    items = [],
    onnavigate,
  }: {
    items?: { label: string; href?: string; value?: string }[];
    onnavigate?: (value: string, index: number) => void;
  } = $props();
</script>

<nav class="flex text-sm text-text-muted font-medium" aria-label="Breadcrumb">
  <ol class="inline-flex items-center gap-2">
    {#each items as item, i}
      <li class="inline-flex items-center">
        {#if i === items.length - 1}
          <span class="text-white">{item.label}</span>
        {:else if onnavigate && item.value !== undefined}
          <button class="hover:text-white transition-colors" onclick={() => onnavigate?.(item.value!, i)}>{item.label}</button>
        {:else if item.href}
          <a href={item.href} class="hover:text-white transition-colors">{item.label}</a>
        {:else}
          <span class="text-white">{item.label}</span>
        {/if}
      </li>
      {#if i < items.length - 1}
        <li class="text-[var(--border-strong)]">/</li>
      {/if}
    {/each}
  </ol>
</nav>
