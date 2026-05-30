// Stack-aware visual treatment for the Run button (and a matching cue chip).
// Three frontend-oriented tiers, each with a graduated rainbow glow + a short
// "what we're optimising for" label:
//
//   • turbo     — a monorepo orchestrator (Turborepo / Nx / Lerna). Vivid
//                 rainbow glow + crisp ring.  "Turborepo detected"
//   • fullstack — an SSR meta-framework (Next.js, Nuxt, SvelteKit, Remix,
//                 Astro, AnalogJS, SolidStart, Qwik). Soft rainbow glow.
//                 "optimising for fullstack"
//   • spa       — a client-only app (Vite-driven React/Vue/Solid, Angular,
//                 plain Svelte). Faint rainbow glow.  "optimising for SPA"
//
// Backends and plain runtimes get nothing for now (by design).
//
// Names come from the detector's `framework_name`, e.g. "Next.js", "SvelteKit",
// "Turborepo", "Vite", "Angular". We normalise the same way TechIcon does:
// lowercase, keep dots, drop the rest.

export type StackEffect = {
  kind: 'turbo' | 'fullstack' | 'spa' | 'none';
  /** short human label for the cue chip / tooltip */
  label?: string;
};

function norm(s: string | null | undefined): string {
  if (!s) return '';
  return s.toLowerCase().replace(/[^a-z0-9.]/g, '');
}

// Monorepo orchestrators → vivid rainbow.
const TURBO_LABEL: Record<string, string> = {
  turborepo: 'Turborepo', turbo: 'Turborepo', nx: 'Nx', lerna: 'Lerna', monorepo: 'Monorepo',
};

// SSR / meta-frameworks → soft rainbow.
const FULLSTACK = new Set([
  'next.js', 'nextjs', 'next',
  'nuxt', 'nuxtjs',
  'sveltekit',
  'remix',
  'astro',
  'analogjs',
  'solidstart',
  'qwik',
]);

// Client-only frameworks (the detector reports "Vite" for a plain React/Vue/
// Solid SPA, "Angular" for an Angular CLI app, "Svelte" for non-kit Svelte).
const SPA = new Set([
  'vite',
  'angular',
  'react',
  'vue', 'vuejs',
  'svelte',
  'solid',
  'preact',
]);

// Label for a turbo stack — name the orchestrator when the framework string
// is one ("Turborepo"/"Nx"/"Lerna"), else a generic "Turborepo detected".
function turboLabel(framework?: string | null): string {
  const f = norm(framework);
  return `${TURBO_LABEL[f] ?? 'Turborepo'} detected`;
}

export function stackEffect(stackKind?: string | null, framework?: string | null): StackEffect {
  // Prefer the detector's authoritative classification (computed in Rust).
  switch ((stackKind ?? '').toLowerCase()) {
    case 'turbo': return { kind: 'turbo', label: turboLabel(framework) };
    case 'fullstack': return { kind: 'fullstack', label: 'optimising for fullstack' };
    case 'spa': return { kind: 'spa', label: 'optimising for SPA' };
    case 'backend': return { kind: 'none' };
  }

  // Fallback: derive from the framework name (older deployment records, or if
  // the detector didn't classify it).
  const f = norm(framework);
  if (!f) return { kind: 'none' };
  if (f in TURBO_LABEL) return { kind: 'turbo', label: turboLabel(framework) };
  if (FULLSTACK.has(f)) return { kind: 'fullstack', label: 'optimising for fullstack' };
  if (SPA.has(f)) return { kind: 'spa', label: 'optimising for SPA' };
  return { kind: 'none' };
}
