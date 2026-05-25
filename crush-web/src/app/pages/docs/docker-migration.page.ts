import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmButtonDirective } from '@spartan-ng/ui-button-helm';
import { HlmIconComponent } from '@spartan-ng/ui-icon-helm';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-docker-migration',
  standalone: true,
  imports: [RouterLink, HlmButtonDirective, HlmIconComponent, DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange">Guide</span>
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              Docker Migration
            </h1>
            <p class="text-base text-crush-textMuted">
              Migrate existing Docker workflows to daemonless Crush container environments in under
              10 minutes.
            </p>
          </div>

          <!-- Step 1 -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >1</span
              >
              Install Crush locally
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Before starting the migration, ensure the Crush command line utility is globally
              linked in your shell.
            </p>

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
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >bash</span
                >
              </div>
              <div class="p-4 font-mono text-sm overflow-x-auto text-crush-text">
                <code>curl -fsSL https://crushrun.dev/install | sh</code>
              </div>
            </div>
          </section>

          <!-- Step 2 -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >2</span
              >
              Convert your Dockerfile
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Run
              <code
                class="bg-crush-surface px-1 py-0.5 rounded text-white border border-crush-border"
                ><span class="text-crush-orange font-bold">crush</span> migrate</code
              >
              inside your project folder. The migration wizard automatically scans your Dockerfile
              instructions, determines the optimal framework layers, and generates a corresponding
              <code
                class="bg-crush-surface px-1 py-0.5 rounded text-white border border-crush-border"
                >Crushfile</code
              >
              schema.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2.5 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-1.5">
                  <span class="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">crush CLI</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >migrate</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted">~/my-api $</span>
                <span class="text-crush-orange font-bold">crush</span> migrate
                <span class="text-crush-textMuted">↳ reading Dockerfile...</span>
                <span class="text-crush-textMuted"
                  >↳ detected: Node.js 20 · TypeScript · Express</span
                >
                <span class="text-crush-textMuted"
                  >↳ generated Crushfile with all dependencies</span
                >
                <span class="text-emerald-400"
                  >✓ migration complete. Run '<span class="text-crush-orange font-bold">crush</span>
                  build' to test.</span
                >
              </div>
            </div>
          </section>

          <!-- Step 3 -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >3</span
              >
              Build and verify sandbox
            </h2>

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
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >build</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"># Compile isolated OCI target image</span>
                <span class="text-crush-orange font-bold">crush</span> build --tag myapp:latest

                <span class="text-crush-textMuted"># Run locally in native container process</span>
                <span class="text-crush-orange font-bold">crush</span> run myapp:latest --port 3000
              </div>
            </div>
          </section>

          <!-- Compose support -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Docker Compose Support</h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Crush handles existing multi-container
              <code
                class="bg-crush-surface px-1 py-0.5 rounded text-white border border-crush-border"
                >docker-compose.yml</code
              >
              deployment environments out of the box without requiring manual re-configuration.
            </p>

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
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >compose</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"
                  ># Spin up compose stack natively inside daemonless jobs</span
                >
                <span class="text-crush-orange font-bold">crush</span> compose up -d
              </div>
            </div>
          </section>

          <!-- DOCKER_HOST compatibility -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">DOCKER_HOST API Layer</h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Crush spins up a localized socket adapter mirroring standard Docker REST sockets.
              External integrations or automation tools communicating over
              <code
                class="bg-crush-surface px-1 py-0.5 rounded text-white border border-crush-border"
                >DOCKER_HOST</code
              >
              interfaces function transparently.
            </p>

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
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >socket adapter</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"
                  ># Route third-party tools through local socket</span
                >
                export DOCKER_HOST=unix:///var/run/crush.sock

                <span class="text-crush-textMuted"
                  ># Standard docker commands route cleanly to Crush process engines</span
                >
                docker ps docker-compose up
              </div>
            </div>
          </section>

          <!-- Footer Navigation Links -->
          <div
            class="flex items-center justify-between border-t border-crush-border/30 pt-8 mt-16 select-none"
          >
            <a
              routerLink="/docs/crushfile"
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
              Crushfile Schema
            </a>
            <a
              routerLink="/docs/windows"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Windows Guide
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
export default class DockerMigrationPage implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Docker Migration — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Migrate from Docker to Crush in 10 minutes. Dockerfile migration, docker-compose support, DOCKER_HOST compatibility.',
    });
  }
}
