// Build-time fetch of the latest GitHub release. Runs once on the build server
// (via the `prebuild` npm hook), so visitors never hit the GitHub API — this
// eliminates the client-side 60-req/hour rate limit entirely. The result is
// written to a TS module that the DownloadBlock component imports statically.
//
// To keep links fresh after a new release, trigger a Vercel Deploy Hook from
// your release CI — each rebuild re-runs this script and re-bakes the assets.
//
// Network-safe: if the fetch fails (offline build, rate limit, API down), we
// emit an empty dataset and the component falls back to the releases page.

import { writeFile, mkdir } from 'node:fs/promises';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = 'Chidi09/crush';
const API = `https://api.github.com/repos/${REPO}/releases/latest`;
const OUT = fileURLToPath(new URL('../src/app/components/download-block/release.data.ts', import.meta.url));

async function main() {
  let version = '';
  let assets = [];

  try {
    const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'crush-web-build' };
    // A token (optional) raises the build-server limit to 5000/hr — set GITHUB_TOKEN in Vercel.
    if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

    const res = await fetch(API, { headers });
    if (res.ok) {
      const data = await res.json();
      version = data.tag_name ?? '';
      assets = (data.assets ?? []).map((a) => ({
        name: a.name,
        url: a.browser_download_url,
        size: a.size,
      }));
      console.log(`[fetch-release] baked ${assets.length} assets for ${version || '(no tag)'}`);
    } else {
      console.warn(`[fetch-release] GitHub API ${res.status} — emitting fallback (releases page)`);
    }
  } catch (err) {
    console.warn(`[fetch-release] fetch failed (${err?.message ?? err}) — emitting fallback`);
  }

  const banner =
    '// AUTO-GENERATED at build time by scripts/fetch-release.mjs. Do not edit by hand.\n';
  const body =
    `export interface ReleaseAsset { name: string; url: string; size: number; }\n` +
    `export interface ReleaseData { version: string; assets: ReleaseAsset[]; }\n\n` +
    `export const RELEASE: ReleaseData = ${JSON.stringify({ version, assets }, null, 2)};\n`;

  await mkdir(dirname(OUT), { recursive: true });
  await writeFile(OUT, banner + body, 'utf8');
  console.log(`[fetch-release] wrote ${OUT}`);
}

main().catch((e) => {
  console.error('[fetch-release] unexpected error', e);
  // Never fail the build over release metadata.
  process.exit(0);
});
