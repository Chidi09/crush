import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    runes: true
  },
  kit: {
    // SPA mode: Tauri serves a static bundle and there is no Node server.
    adapter: adapter({
      fallback: 'index.html',
      precompress: false,
      strict: false
    }),
    // Tauri embeds the build output from ../build (see tauri.conf.json frontendDist).
    outDir: '.svelte-kit'
  }
};
