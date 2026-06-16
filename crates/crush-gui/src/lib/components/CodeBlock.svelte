<script lang="ts">
  let { code = "", language = "text" }: { code?: string; language?: string } = $props();

  // Basic naive highlighting just for structural preview.
  function highlight(str: string) {
    if (language === 'sql') {
      return str.replace(/\b(SELECT|FROM|WHERE|INSERT|UPDATE|DELETE|JOIN|ON|GROUP BY|ORDER BY|AND|OR|NOT|NULL)\b/gi, '<span class="text-[var(--status-info)] font-bold">$1</span>');
    }
    return str.replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
</script>

<pre class="m-0 p-4 bg-surface-raised border border-[var(--border)] rounded-lg font-mono text-xs overflow-auto text-text leading-relaxed crush-card"><code>{@html highlight(code)}</code></pre>
