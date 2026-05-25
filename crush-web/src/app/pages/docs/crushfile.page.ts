import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-crushfile',
  standalone: true,
  imports: [RouterLink, DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange"
              >Reference</span
            >
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">Crushfile</h1>
            <p class="text-base text-crush-textMuted">
              Complete TOML configuration schema for project compilation parameters.
            </p>
          </div>

          <!-- Section 1: Overview -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Overview</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              The
              <code
                class="bg-crush-surface px-1 py-0.5 rounded text-white border border-crush-border"
                >Crushfile</code
              >
              is a declarative TOML configuration located at the root of your workspace. Crush
              auto-detects and generates one when running, but developers can fine-tune compiling or
              runtime isolation settings directly.
            </p>
          </section>

          <!-- Section 2: Example -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none font-sans">Minimal Example</h2>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-2">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Crushfile</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >toml</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                [project] <span class="text-crush-orange font-semibold">name</span> =
                <span class="text-emerald-400">"my-api"</span>
                <span class="text-crush-orange font-semibold">version</span> =
                <span class="text-emerald-400">"0.1.0"</span>

                [build]
                <span class="text-crush-orange font-semibold">platform</span> =
                <span class="text-emerald-400">"linux/amd64"</span>
              </div>
            </div>
          </section>

          <!-- Section 3: Full Schema -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Full Schema</h2>
            <p class="text-sm text-crush-textMuted mb-6 leading-relaxed">
              Below is the comprehensive TOML field directory showing default values and supported
              data structures:
            </p>

            <div class="space-y-4">
              @for (field of fields; track field.key) {
                <div
                  class="group relative overflow-hidden rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5 hover:border-crush-orange/20 transition-all duration-300"
                >
                  <div
                    class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-crush-border/20 pb-2 mb-2 select-none"
                  >
                    <span
                      class="font-mono text-sm font-semibold text-crush-orangeLight group-hover:text-crush-orange transition-colors"
                    >
                      {{ field.key }}
                    </span>
                    <div class="flex items-center gap-2">
                      <span
                        class="px-2 py-0.5 rounded text-[10px] font-mono bg-crush-surface text-crush-textMuted border border-crush-border/60"
                      >
                        {{ field.type }}
                      </span>
                      <span
                        class="px-2 py-0.5 rounded text-[10px] font-mono bg-crush-orange/5 text-crush-orangeLight border border-crush-orange/20"
                      >
                        default: {{ field.default }}
                      </span>
                    </div>
                  </div>
                  <p class="text-sm text-crush-textMuted leading-relaxed">{{ field.desc }}</p>
                </div>
              }
            </div>
          </section>

          <!-- Section 4: Vs Dockerfile -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Crushfile vs Dockerfile</h2>
            <p class="text-sm text-crush-textMuted mb-6 leading-relaxed">
              A Crushfile replaces multi-stage Dockerfiles. It handles dependencies and build
              targets cleanly without writing complex scripts or hunting base images.
            </p>

            <div class="grid gap-6 md:grid-cols-2">
              <div
                class="rounded-xl border border-crush-border/30 bg-crush-surface/10 overflow-hidden opacity-60 hover:opacity-100 transition-opacity duration-300"
              >
                <div
                  class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
                >
                  <div class="flex items-center gap-2">
                    <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                    <span class="text-[10px] text-crush-textMuted font-mono ml-2">Dockerfile</span>
                  </div>
                  <span
                    class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                    >docker</span
                  >
                </div>
                <div
                  class="p-4 font-mono text-xs text-crush-textMuted leading-relaxed whitespace-pre overflow-x-auto"
                >
                  FROM node:20-alpine WORKDIR /app COPY package*.json ./ RUN npm ci COPY . . EXPOSE
                  3000 CMD ["node", "dist/index.js"]
                </div>
              </div>

              <div
                class="rounded-xl border border-crush-orange/30 bg-gradient-to-b from-crush-orange/5 to-transparent overflow-hidden hover:border-crush-orange/50 transition-colors duration-300"
              >
                <div
                  class="flex items-center justify-between px-4 py-2 border-b border-crush-orange/20 bg-crush-orange/5 select-none"
                >
                  <div class="flex items-center gap-2">
                    <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                    <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                    <span class="text-[10px] text-crush-orangeLight font-mono ml-2">Crushfile</span>
                  </div>
                  <span
                    class="text-[9px] text-crush-orangeLight uppercase tracking-wider font-semibold"
                    >toml</span
                  >
                </div>
                <div
                  class="p-4 font-mono text-xs text-crush-text leading-relaxed whitespace-pre overflow-x-auto"
                >
                  [project] name = "my-api" [build] platform = "linux/amd64"
                </div>
              </div>
            </div>
          </section>

          <!-- Footer Navigation Links -->
          <div
            class="flex items-center justify-between border-t border-crush-border/30 pt-8 mt-16 select-none"
          >
            <a
              routerLink="/docs/cli-reference"
              class="inline-flex items-center gap-2 text-sm text-crush-textMuted hover:text-white transition-colors"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-4 w-4"
              >
                <line x1="19" y1="12" x2="5" y2="12" />
                <polyline points="12 19 5 12 12 5" />
              </svg>
              CLI Reference
            </a>
            <a
              routerLink="/docs/docker-migration"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Docker Migration
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-4 w-4"
              >
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </a>
          </div>
        </article>
      </div>
    </div>
  `,
})
export default class CrushfilePage implements OnInit {
  fields = [
    { key: '[project] name', type: 'string', default: 'directory name', desc: 'Project name.' },
    { key: '[project] version', type: 'string', default: '"0.1.0"', desc: 'Project version.' },
    {
      key: '[build] platform',
      type: 'string | string[]',
      default: '"linux/amd64"',
      desc: 'Target platform(s). Use "linux/amd64,linux/arm64" for multi-arch.',
    },
    {
      key: '[build] base_image',
      type: 'string',
      default: 'auto-detected',
      desc: 'Base image override.',
    },
    {
      key: '[build] include',
      type: 'string[]',
      default: 'auto',
      desc: 'Files and directories to include in the image.',
    },
    {
      key: '[build] exclude',
      type: 'string[]',
      default: 'auto',
      desc: 'Files and directories to exclude.',
    },
    {
      key: '[build] cmd',
      type: 'string | string[]',
      default: 'auto-detected',
      desc: 'Default command to run.',
    },
    { key: '[build] env', type: 'table', default: '{}', desc: 'Environment variables.' },
    { key: '[build] expose', type: 'number[]', default: 'auto-detected', desc: 'Ports to expose.' },
    {
      key: '[platforms] dev',
      type: 'string',
      default: 'auto',
      desc: 'Platform for local development.',
    },
    {
      key: '[platforms] prod',
      type: 'string',
      default: '"linux/amd64"',
      desc: 'Platform for production deployment.',
    },
    {
      key: '[secrets] *',
      type: 'string',
      default: '—',
      desc: 'Encrypted secrets. Set with "crush secrets set".',
    },
    {
      key: '[runtime] restart',
      type: 'string',
      default: '"no"',
      desc: 'Restart policy: "no", "always", "on-failure".',
    },
    {
      key: '[runtime] memory',
      type: 'string',
      default: 'unlimited',
      desc: 'Memory limit (e.g. "512MB", "2GB").',
    },
    {
      key: '[runtime] cpu',
      type: 'number',
      default: 'unlimited',
      desc: 'CPU limit (e.g. 0.5 for half a core).',
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Crushfile — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Full Crushfile TOML schema reference. Auto-generate or manually configure your Crush projects.',
    });
  }
}
