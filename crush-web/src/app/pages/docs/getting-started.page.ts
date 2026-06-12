import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmButtonDirective } from '@spartan-ng/ui-button-helm';
import { HlmIconComponent } from '@spartan-ng/ui-icon-helm';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-getting-started',
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
              Getting Started
            </h1>
            <p class="text-base text-crush-textMuted">
              Your first container up and running in under 5 minutes.
            </p>
          </div>

          <!-- Section 1: Install -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >1</span
              >
              Install Crush
            </h2>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30"
              >
                <div class="flex items-center gap-2 select-none">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span
                  class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold select-none"
                  >bash</span
                >
              </div>
              <div class="p-4 font-mono text-sm overflow-x-auto text-crush-text">
                <code>curl -fsSL https://crush-web-six.vercel.app/install.sh | sh</code>
              </div>
            </div>

            <div
              class="flex gap-3.5 p-4 rounded-xl border border-crush-orange/15 bg-crush-orange/5 mb-4 text-xs sm:text-sm"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-5 w-5 text-crush-orange shrink-0 mt-0.5 select-none"
              >
                <path d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <p class="font-bold text-crush-orangeLight mb-0.5">Alternative Methods Available</p>
                <p class="text-crush-textMuted leading-relaxed">
                  Prefer a different package manager? See the
                  <a
                    routerLink="/docs/installation"
                    class="text-crush-orange hover:text-crush-orangeLight transition-colors font-semibold"
                    >installation guide</a
                  >
                  for Homebrew, Cargo, Winget, Scoop, DNF, and APT repositories.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 2: Navigate -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >2</span
              >
              Enter Your Project
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Navigate into any of your active project folders. Crush is highly versatile and
              intelligently detects runtime stacks automatically.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30"
              >
                <div class="flex items-center gap-2 select-none">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span
                  class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold select-none"
                  >bash</span
                >
              </div>
              <div class="p-4 font-mono text-sm overflow-x-auto text-crush-text">
                <code>cd ~/projects/my-api</code>
              </div>
            </div>

            <div
              class="flex gap-3.5 p-4 rounded-xl border border-emerald-500/15 bg-emerald-500/5 text-xs sm:text-sm select-none"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-5 w-5 text-emerald-400 shrink-0 mt-0.5"
              >
                <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <p class="font-bold text-emerald-400 mb-0.5">Auto-detection Engine</p>
                <p class="text-crush-textMuted leading-relaxed">
                  Crush auto-detects Node.js, Python, Go, Rust, .NET, Ruby, PHP, and other standard
                  stacks to build perfect sandboxes.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 3: Run -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >3</span
              >
              Run Crush
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Launch the runtime command. Crush builds the dependency layer, compiles sandboxes, and
              spawns container environments immediately.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30"
              >
                <div class="flex items-center gap-2 select-none">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span
                  class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold select-none"
                  >crush CLI</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted">~/my-api $</span>
                <span class="text-crush-orange font-bold">crush</span>
                <span class="text-crush-textMuted"
                  >↳ detected: Node.js 20 · TypeScript · Express</span
                >
                <span class="text-crush-textMuted">↳ deps layer cached (lockfile unchanged)</span>
                <span class="text-emerald-400"
                  >✓ crushed to image my-api:latest (0.9s · 41 MB)</span
                >
                <span class="text-crush-textMuted">run it now? [Y/n]</span>
                <span class="text-emerald-400">✓ running on :3000 — started in 0.3s</span>
              </div>
            </div>
          </section>

          <!-- Section 4: Watch Mode -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >4</span
              >
              Hot-Reloading Watcher
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Enable active developer hot-reload. Crush syncs local workspace updates directly into
              isolated microVMs or process pools instantly without complete restarts.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30"
              >
                <div class="flex items-center gap-2 select-none">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span
                  class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold select-none"
                  >watch</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted">~/my-api $ </span
                ><span class="text-crush-orange font-bold">crush</span> watch
                <span class="text-crush-textMuted"
                  >↳ Active filesystem hot-reload watcher initialized...</span
                >
                <span class="text-crush-textMuted"
                  >↳ Change detected: src/routes/users.ts (modified)</span
                >
                <span class="text-emerald-400"
                  >✓ Native container sandbox updated incrementally in 24ms</span
                >
              </div>
            </div>
          </section>

          <!-- Section 5: Deploy -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 flex items-center gap-3 select-none">
              <span
                class="flex h-6 w-6 items-center justify-center rounded-full bg-crush-orange/10 text-crush-orange text-xs font-bold border border-crush-orange/20"
                >5</span
              >
              Deploy Anywhere
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Crush compiles target projects into standard OCI compliance schemas. Push to any
              registry and spin up standard OCI images across standard cloud configurations or Linux
              systems.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/85 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-2 select-none">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span
                  class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold select-none"
                  >registry & deploy</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"
                  ># Push image directly to secure OCI registry</span
                >
                <span class="text-crush-textMuted">~/my-api $ </span
                ><span class="text-crush-orange font-bold">crush</span> push
                ghcr.io/chidi09/myapp:latest
                <span class="text-crush-textMuted">↳ Uploading dependency layer cache [100%]</span>
                <span class="text-emerald-400"
                  >✓ Image pushed successfully (ghcr.io/chidi09/myapp:latest)</span
                >

                <span class="text-crush-textMuted"
                  ># Execute image deployment cleanly on remote systems</span
                >
                <span class="text-crush-textMuted">~/my-api $ </span
                ><span class="text-crush-orange font-bold">crush</span> run
                ghcr.io/chidi09/myapp:latest --port 3000 -d
                <span class="text-crush-textMuted"
                  >↳ Launching daemonless process in remote container scheduler...</span
                >
                <span class="text-emerald-400"
                  >✓ Container running in background (PID: 94812) listening on port :3000</span
                >
              </div>
            </div>
          </section>

          <!-- Footer Navigation Links -->
          <div
            class="flex items-center justify-between border-t border-crush-border/30 pt-8 mt-16 select-none"
          >
            <a
              routerLink="/docs"
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
              Back to Overview
            </a>
            <a
              routerLink="/docs/installation"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Installation Guide
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
export default class GettingStartedPage implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Getting Started — Crush');
    this.meta.updateTag({
      name: 'description',
      content: 'Your first container in 5 minutes with Crush. Install, build, run, deploy.',
    });
  }
}
