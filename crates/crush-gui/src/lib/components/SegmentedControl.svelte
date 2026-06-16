<script lang="ts">
  let {
    options = [],
    selected = $bindable(),
    onchange,
  }: {
    options?: { value: string; label: string }[];
    selected?: string;
    onchange?: (value: string) => void;
  } = $props();

  // Default to the first option when uncontrolled.
  $effect(() => {
    if (selected === undefined && options.length) selected = options[0].value;
  });

  function select(val: string) {
    selected = val;
    onchange?.(val);
  }
</script>

<div class="inline-flex p-1 bg-surface-raised border border-[var(--border)] rounded-md gap-1">
  {#each options as opt}
    <button
      class="px-3 py-1 text-sm rounded transition-colors"
      class:bg-surface={selected === opt.value}
      class:text-white={selected === opt.value}
      class:text-text-muted={selected !== opt.value}
      class:hover:text-text={selected !== opt.value}
      onclick={() => select(opt.value)}
    >
      {opt.label}
    </button>
  {/each}
</div>
