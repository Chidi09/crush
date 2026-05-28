// Pure client-side SPA — Tauri has no Node server and renders everything
// in the embedded webview, so disable SSR and prerendering. adapter-static
// emits an index.html fallback that the SvelteKit client router takes over.
export const ssr = false;
export const prerender = false;
