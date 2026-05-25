import { Component, OnInit } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Title, Meta } from '@angular/platform-browser';
import { HlmButtonDirective } from '@spartan-ng/ui-button-helm';
import { HlmIconComponent } from '@spartan-ng/ui-icon-helm';
import { HlmBadgeDirective } from '../ui/badge';
import {
  HlmCardDirective,
  HlmCardContentDirective,
  HlmCardHeaderDirective,
  HlmCardTitleDirective,
} from '../ui/card';
import { TerminalComponent } from '../components/terminal/terminal.component';
import { InstallBlockComponent } from '../components/install-block/install-block.component';
import { ComparisonTableComponent } from '../components/comparison-table/comparison-table.component';

@Component({
  selector: 'page-index',
  standalone: true,
  imports: [
    RouterLink,
    HlmButtonDirective,
    HlmIconComponent,
    HlmBadgeDirective,
    HlmCardDirective,
    HlmCardContentDirective,
    HlmCardHeaderDirective,
    HlmCardTitleDirective,
    TerminalComponent,
    InstallBlockComponent,
    ComparisonTableComponent,
  ],
  template: `
    <!-- Hero -->
    <section class="relative overflow-hidden border-b border-crush-border/30">
      <div
        class="absolute inset-0 bg-gradient-to-b from-crush-orange/5 via-transparent to-transparent pointer-events-none"
      ></div>
      <div class="mx-auto max-w-7xl px-4 pt-20 pb-24 sm:px-6 lg:px-8 sm:pt-32 lg:pt-40">
        <div class="mx-auto max-w-3xl text-center">
          <div
            class="mb-6 inline-flex items-center gap-2 rounded-full border border-crush-border/50 bg-crush-surface/50 px-4 py-1.5"
          >
            <span
              hlmBadge
              variant="outline"
              class="border-crush-orange/50 bg-crush-orange/10 text-crush-orange hover:bg-crush-orange/20"
              >v0.1.0</span
            >
            <span class="text-sm text-crush-textMuted">Native Windows container runtime</span>
          </div>
          <h1
            class="text-4xl font-extrabold tracking-tight text-white sm:text-5xl lg:text-6xl text-balance"
          >
            Containers that
            <span class="gradient-text">actually work</span>
            on Windows
          </h1>
          <p class="mt-6 text-lg leading-8 text-crush-textMuted max-w-2xl mx-auto">
            Sub-second starts. No WSL2. No VM overhead. Crush is a native Windows container runtime
            built on Job Objects. Build on Windows, deploy to any Linux server.
          </p>
          <div class="mt-10 flex items-center justify-center gap-4 flex-wrap">
            <a
              routerLink="/docs/getting-started"
              class="inline-flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-semibold text-white bg-crush-orange hover:bg-crush-orangeLight transition-colors duration-200 select-none outline-none"
            >
              Get Started
              <svg
                viewBox="0 0 24 24"
                class="h-4 w-4 fill-none stroke-current stroke-2.5 select-none"
              >
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </a>
            <a
              href="https://github.com/crushcontainer/crush"
              target="_blank"
              rel="noopener"
              class="inline-flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-semibold text-crush-text bg-transparent border border-crush-border hover:bg-crush-surface/50 hover:text-white hover:border-crush-border/80 transition-colors duration-200 select-none outline-none"
            >
              <svg
                role="img"
                viewBox="0 0 24 24"
                class="h-4 w-4 fill-current text-crush-textMuted transition-colors duration-200 select-none"
              >
                <path
                  d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"
                />
              </svg>
              GitHub
            </a>
          </div>
        </div>

        <div class="mt-16 mx-auto max-w-3xl">
          <app-terminal />
        </div>
      </div>
    </section>

    <!-- Features -->
    <section class="py-24 sm:py-32 relative overflow-hidden">
      <!-- Ambient light gradients background -->
      <div
        class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] bg-crush-orange/3 blur-[140px] pointer-events-none rounded-full"
      ></div>

      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 relative">
        <div class="mx-auto max-w-3xl text-center mb-20 select-none">
          <div
            class="mb-4 inline-flex items-center gap-1.5 rounded-full border border-crush-orange/20 bg-crush-orange/5 px-3 py-1 text-xs font-semibold text-crush-orange uppercase tracking-wider"
          >
            Engineered for Windows
          </div>
          <h2 class="text-4xl font-extrabold tracking-tight text-white sm:text-5xl">Why Crush?</h2>
          <p class="mt-4 text-lg text-crush-textMuted max-w-2xl mx-auto text-balance">
            Because containerizing on Windows should be simple, lightweight, and native—not
            requiring a resource-heavy Linux VM.
          </p>
        </div>

        <div class="grid gap-8 md:grid-cols-3">
          <!-- Card 1: Sub-second starts -->
          <div
            class="relative overflow-hidden rounded-2xl border border-crush-border/40 bg-gradient-to-b from-crush-surface/40 to-crush-surface/10 p-8 hover:border-crush-orange/30 hover:bg-crush-surface/20 hover:shadow-[0_0_30px_rgba(224,85,64,0.03)] transition-all duration-300 group flex flex-col justify-between"
          >
            <!-- Corner Glow -->
            <div
              class="absolute -right-16 -top-16 w-36 h-36 rounded-full bg-crush-orange/5 blur-3xl group-hover:bg-crush-orange/12 transition-all duration-500 pointer-events-none"
            ></div>

            <div>
              <div
                class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-crush-orange/10 to-crush-orangeLight/5 border border-crush-orange/20 text-crush-orangeLight shadow-[0_0_12px_rgba(224,85,64,0.05)] group-hover:scale-110 group-hover:border-crush-orange/40 group-hover:shadow-[0_0_20px_rgba(224,85,64,0.15)] transition-all duration-300 mb-6 select-none"
              >
                <!-- Explosion Starburst SVG -->
                <svg
                  viewBox="0 0 24 24"
                  class="h-6 w-6 fill-none stroke-current stroke-2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M12 2l2 4 4 1-3 3 2 4-5-2-5 2 2-4-3-3 4-1z" />
                  <path d="M6 6L3 3M18 6l3-3M6 18l-3 3M18 18l3 3" />
                </svg>
              </div>

              <h3
                class="text-xl font-bold text-white mb-3 group-hover:text-crush-orangeLight transition-colors duration-300"
              >
                Sub-second starts
              </h3>
              <p
                class="text-sm text-crush-textMuted leading-relaxed group-hover:text-crush-text transition-colors duration-300"
              >
                No daemon to wait for. No VM to boot. Crush starts containers in under a second.
                Just a single binary and your code is running.
              </p>
            </div>

            <!-- Dynamic UI Graphic -->
            <div
              class="mt-6 rounded-xl border border-crush-border/40 bg-crush-black/50 p-4 font-mono text-xs select-none relative overflow-hidden group-hover:border-crush-orange/20 transition-all duration-300"
            >
              <div
                class="flex items-center justify-between mb-3 border-b border-crush-border/30 pb-2"
              >
                <span class="text-crush-textMuted font-bold">CONTAINER METRICS</span>
                <span
                  class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-500/10 text-emerald-400"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                  Running
                </span>
              </div>
              <div class="space-y-2">
                <div class="flex justify-between items-center">
                  <span class="text-crush-textMuted">Startup Time</span>
                  <span class="text-crush-orange font-bold font-mono">0.28s</span>
                </div>
                <div class="w-full bg-crush-border/40 h-1.5 rounded-full overflow-hidden">
                  <div
                    class="bg-gradient-to-r from-crush-orange to-crush-orangeLight h-full rounded-full transition-all duration-1000 ease-out"
                    style="width: 100%"
                  ></div>
                </div>
                <div
                  class="flex justify-between items-center text-[10px] text-crush-textMuted pt-1"
                >
                  <span>CPU: 0.02%</span>
                  <span>MEM: 12.4 MB</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Card 2: Windows native -->
          <div
            class="relative overflow-hidden rounded-2xl border border-crush-border/40 bg-gradient-to-b from-crush-surface/40 to-crush-surface/10 p-8 hover:border-crush-orange/30 hover:bg-crush-surface/20 hover:shadow-[0_0_30px_rgba(224,85,64,0.03)] transition-all duration-300 group flex flex-col justify-between"
          >
            <!-- Corner Glow -->
            <div
              class="absolute -right-16 -top-16 w-36 h-36 rounded-full bg-crush-orange/5 blur-3xl group-hover:bg-crush-orange/12 transition-all duration-500 pointer-events-none"
            ></div>

            <div>
              <div
                class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-crush-orange/10 to-crush-orangeLight/5 border border-crush-orange/20 text-crush-orangeLight shadow-[0_0_12px_rgba(224,85,64,0.05)] group-hover:scale-110 group-hover:border-crush-orange/40 group-hover:shadow-[0_0_20px_rgba(224,85,64,0.15)] transition-all duration-300 mb-6 select-none"
              >
                <!-- Actual flat modern Windows Logo SVG -->
                <svg viewBox="0 0 24 24" class="h-6 w-6 fill-current">
                  <path d="M3 3h8.5v8.5H3zm10 0h8.5v8.5H13zM3 13h8.5v8.5H3zm10 0h8.5v8.5H13z" />
                </svg>
              </div>

              <h3
                class="text-xl font-bold text-white mb-3 group-hover:text-crush-orangeLight transition-colors duration-300"
              >
                Windows native
              </h3>
              <p
                class="text-sm text-crush-textMuted leading-relaxed group-hover:text-crush-text transition-colors duration-300"
              >
                Built on Windows Job Objects and API sets. No WSL2, no Hyper-V, no Docker Desktop.
                Just your Windows dev machine and your code.
              </p>
            </div>

            <!-- Dynamic UI Graphic -->
            <div
              class="mt-6 rounded-xl border border-crush-border/40 bg-crush-black/50 p-4 font-mono text-xs select-none relative overflow-hidden group-hover:border-crush-orange/20 transition-all duration-300"
            >
              <div
                class="flex items-center justify-between mb-3 border-b border-crush-border/30 pb-2"
              >
                <span class="text-crush-textMuted font-bold">NATIVE KERNEL</span>
                <span class="text-crush-orangeLight text-[10px]">Zero Overhead</span>
              </div>
              <div class="space-y-2.5 text-[10px] leading-relaxed">
                <div
                  class="flex items-center justify-between p-1 rounded bg-crush-orange/5 border border-crush-orange/10"
                >
                  <span class="text-crush-orange font-bold">Crush:</span>
                  <span class="text-crush-text"
                    >App &rarr;
                    <span class="bg-crush-orange/20 text-crush-orangeLight px-1 rounded text-[9px]"
                      >Job Object</span
                    >
                    &rarr; OS Kernel</span
                  >
                </div>
                <div
                  class="flex items-center justify-between p-1 rounded bg-crush-surface/40 border border-crush-border/30 opacity-40 line-through"
                >
                  <span class="text-crush-textMuted">Docker:</span>
                  <span class="text-crush-textMuted text-[9px]"
                    >App &rarr; WSL2 VM &rarr; Linux Kernel</span
                  >
                </div>
              </div>
            </div>
          </div>

          <!-- Card 3: AI diagnosis -->
          <div
            class="relative overflow-hidden rounded-2xl border border-crush-border/40 bg-gradient-to-b from-crush-surface/40 to-crush-surface/10 p-8 hover:border-crush-orange/30 hover:bg-crush-surface/20 hover:shadow-[0_0_30px_rgba(224,85,64,0.03)] transition-all duration-300 group flex flex-col justify-between"
          >
            <!-- Corner Glow -->
            <div
              class="absolute -right-16 -top-16 w-36 h-36 rounded-full bg-crush-orange/5 blur-3xl group-hover:bg-crush-orange/12 transition-all duration-500 pointer-events-none"
            ></div>

            <div>
              <div
                class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-crush-orange/10 to-crush-orangeLight/5 border border-crush-orange/20 text-crush-orangeLight shadow-[0_0_12px_rgba(224,85,64,0.05)] group-hover:scale-110 group-hover:border-crush-orange/40 group-hover:shadow-[0_0_20px_rgba(224,85,64,0.15)] transition-all duration-300 mb-6 select-none"
              >
                <!-- Globally-known AI Sparkle/Shimmer SVG -->
                <svg viewBox="0 0 24 24" class="h-6 w-6 fill-current">
                  <path
                    d="M12 2c0 5.5 4.5 10 10 10-5.5 0-10 4.5-10 10 0-5.5-4.5-10-10-10 5.5 0 10-4.5 10-10z"
                  />
                  <path
                    d="M19 3c0 2.2 1.8 4 4 4-2.2 0-4 1.8-4 4 0-2.2-1.8-4-4-4 2.2 0 4-1.8 4-4z"
                  />
                </svg>
              </div>

              <h3
                class="text-xl font-bold text-white mb-3 group-hover:text-crush-orangeLight transition-colors duration-300"
              >
                AI diagnosis
              </h3>
              <p
                class="text-sm text-crush-textMuted leading-relaxed group-hover:text-crush-text transition-colors duration-300"
              >
                When something breaks, Crush tells you what and why. Built-in crash analysis means
                fewer "works on my machine" moments.
              </p>
            </div>

            <!-- Dynamic UI Graphic -->
            <div
              class="mt-6 rounded-xl border border-crush-border/40 bg-crush-black/50 p-4 font-mono text-xs select-none relative overflow-hidden group-hover:border-crush-orange/20 transition-all duration-300"
            >
              <div
                class="flex items-center justify-between mb-3 border-b border-crush-border/30 pb-2"
              >
                <span class="text-red-400 font-bold flex items-center gap-1.5">
                  <span class="w-1.5 h-1.5 rounded-full bg-red-500 animate-ping"></span>
                  FATAL
                </span>
                <span class="text-crush-orangeLight text-[10px] animate-pulse"
                  >AI Agent Active</span
                >
              </div>
              <div class="space-y-1.5 text-[9px] leading-relaxed">
                <div class="text-red-400/95 font-mono">Error: Connection refused on port 5432</div>
                <div
                  class="text-crush-orange border-t border-crush-orange/10 pt-1.5 mt-1 font-bold"
                >
                  Crush Diagnostic: Missing database environment variable.
                </div>
                <div class="text-crush-textMuted">
                  Fix:
                  <code
                    class="bg-crush-surface px-1 py-0.5 rounded text-white border border-crush-border"
                    >crush secrets set DB_HOST</code
                  >
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <hr class="max-w-7xl mx-auto border-crush-border/30" />

    <!-- Comparison -->
    <section class="py-20 sm:py-28">
      <div class="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto max-w-2xl text-center mb-12">
          <h2 class="text-3xl font-bold text-white sm:text-4xl">Crush vs Docker Desktop</h2>
          <p class="mt-4 text-lg text-crush-textMuted">
            Built for Windows, not running on Windows despite itself
          </p>
        </div>
        <app-comparison-table />
      </div>
    </section>

    <hr class="max-w-7xl mx-auto border-crush-border/30" />

    <!-- Works Everywhere -->
    <section class="py-20 sm:py-28">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto max-w-2xl text-center mb-16">
          <h2 class="text-3xl font-bold text-white sm:text-4xl">Works everywhere</h2>
          <p class="mt-4 text-lg text-crush-textMuted">
            Develop on Windows. Deploy to any Linux server.
          </p>
        </div>

        <div class="rounded-xl border border-crush-border/50 bg-crush-surface/30 p-8 mb-12">
          <div class="flex flex-col sm:flex-row items-center justify-between gap-4 text-sm">
            <div class="flex items-center gap-3 text-crush-text font-semibold group">
              <svg
                role="img"
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-current text-crush-orangeLight transition-transform duration-300 group-hover:scale-110 shrink-0"
              >
                <path
                  d="M0 0h11.377v11.373H0zm12.623 0H24v11.373H12.623zM0 12.627h11.377V24H0zm12.623 0H24V24H12.623z"
                />
              </svg>
              <span>Windows dev machine</span>
            </div>
            <hlm-icon
              name="lucideArrowRight"
              size="sm"
              class="text-crush-orange rotate-90 sm:rotate-0"
            />
            <div class="flex items-center gap-3 text-crush-text font-semibold group">
              <svg
                role="img"
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-current text-crush-orangeLight transition-transform duration-300 group-hover:scale-110 shrink-0"
              >
                <path
                  d="M13.983 11.078h2.119a.186.186 0 00.186-.185V9.006a.186.186 0 00-.186-.186h-2.119a.185.185 0 00-.185.185v1.888c0 .102.083.185.185.185m-2.954-5.43h2.118a.186.186 0 00.186-.186V3.574a.186.186 0 00-.186-.185h-2.118a.185.185 0 00-.185.185v1.888c0 .102.082.185.185.185m0 2.716h2.118a.187.187 0 00.186-.186V6.29a.186.186 0 00-.186-.185h-2.118a.185.185 0 00-.185.185v1.887c0 .102.082.185.185.186m-2.93 0h2.12a.186.186 0 00.184-.186V6.29a.185.185 0 00-.185-.185H8.1a.185.185 0 00-.185.185v1.887c0 .102.083.185.185.186m-2.964 0h2.119a.186.186 0 00.185-.186V6.29a.185.185 0 00-.185-.185H5.136a.185.185 0 00-.185.185v1.887c0 .102.084.185.185.186m5.893 2.715h2.118a.185.185 0 00.186-.185V9.006a.186.186 0 00-.186-.186h-2.118a.185.185 0 00-.185.185v1.888c0 .102.082.185.185.185m-2.928 0h2.12a.185.185 0 00.185-.185V9.006a.186.186 0 00-.186-.186h-2.12a.185.185 0 00-.184.185v1.888c0 .102.083.185.184.185m-2.964 0h2.119a.185.185 0 00.185-.185V9.006a.186.186 0 00-.185-.186H5.136a.185.185 0 00-.185.185v1.888c0 .102.084.185.185.185m-2.964 0h2.119a.185.185 0 00.185-.185V9.006a.186.186 0 00-.185-.186H2.17a.185.185 0 00-.185.185v1.888c0 .102.083.185.185.185m-1.21-2.715h2.119a.186.186 0 00.186-.186V6.29a.186.186 0 00-.186-.185H.96a.185.185 0 00-.185.185v1.887c0 .102.083.185.185.186m18.777 4.095c.563-.092 1.1-.285 1.545-.583a4.01 4.01 0 00.77-.665 7.97 7.97 0 001.328-1.996c.277-.604.428-1.26.44-1.921l.012-.48a3.11 3.11 0 00-.012-.395l-.014-.239a5.95 5.95 0 00-.518-1.802 8.78 8.78 0 00-1.232-1.998 3.51 3.51 0 00-.882-.765c-.482-.315-1.047-.48-1.62-.477-.103-.001-.206 0-.31.006-.566.035-1.12.186-1.62.443h-.012v-.004l-.062.036c-.035.021-.073.042-.11.066a5.53 5.53 0 00-1.636 1.628l-.053.087-.04.098c-.131.332-.234.675-.308 1.026v.006l-.004.02v.006l-.002.02c-.068.423-.086.852-.054 1.28v.002a7.35 7.35 0 00.22 1.48v.005a7.39 7.39 0 00.518 1.396l.035.07.037.07c.28.487.64.916 1.062 1.272a8.68 8.68 0 002.03 1.22c.602.268 1.25.405 1.905.403m.036-.883a4.91 4.91 0 01-1.063-.122 7.74 7.74 0 01-1.801-1.077 7.15 7.15 0 01-1.785-2.015 6.47 6.47 0 01-.65-2.457 6.13 6.13 0 01.05-2.221c.06-.31.156-.612.288-.9l.063-.131.075-.12c.313-.46.7-.847 1.15-1.144a8.16 8.16 0 012.759-.974l.115-.015a2.6 2.6 0 011.082.122 3.12 3.12 0 011.163.79c.345.424.57 1 .667 1.77.05.4.053.864-.015 1.645-.07.8-.3 1.572-.647 2.277a7.08 7.08 0 01-1.189 1.788 3.52 3.52 0 01-1.378.796c-.347.108-.707.163-1.07.163"
                />
              </svg>
              <span>OCI image</span>
            </div>
            <hlm-icon
              name="lucideArrowRight"
              size="sm"
              class="text-crush-orange rotate-90 sm:rotate-0"
            />
            <div class="flex items-center gap-3 text-crush-text font-semibold group">
              <svg
                role="img"
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-current text-crush-orangeLight transition-transform duration-300 group-hover:scale-110 shrink-0"
              >
                <path
                  d="M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.134.063.198.114.333l.003.003c.391.778 1.113 1.132 1.884 1.071.771-.06 1.592-.536 2.257-1.306.631-.765 1.683-1.084 2.378-1.503.348-.199.629-.469.649-.853.023-.4-.2-.811-.714-1.376v-.097l-.003-.003c-.17-.2-.25-.535-.338-.926-.085-.401-.182-.786-.492-1.046h-.003c-.059-.054-.123-.067-.188-.135a.357.357 0 00-.19-.064c.431-1.278.264-2.55-.173-3.694-.533-1.41-1.465-2.638-2.175-3.483-.796-1.005-1.576-1.957-1.56-3.368.026-2.152.236-6.133-3.544-6.139zm.529 3.405h.013c.213 0 .396.062.584.198.19.135.33.332.438.533.105.259.158.459.166.724 0-.02.006-.04.006-.06v.105a.086.086 0 01-.004-.021l-.004-.024a1.807 1.807 0 01-.15.706.953.953 0 01-.213.335.71.71 0 00-.088-.042c-.104-.045-.198-.064-.284-.133a1.312 1.312 0 00-.22-.066c.05-.06.146-.133.183-.198.053-.128.082-.264.088-.402v-.02a1.21 1.21 0 00-.061-.4c-.045-.134-.101-.2-.183-.333-.084-.066-.167-.132-.267-.132h-.016c-.093 0-.176.03-.262.132a.8.8 0 00-.205.334 1.18 1.18 0 00-.09.4v.019c.002.089.008.179.02.267-.193-.067-.438-.135-.607-.202a1.635 1.635 0 01-.018-.2v-.02a1.772 1.772 0 01.15-.768c.082-.22.232-.406.43-.533a.985.985 0 01.594-.2zm-2.962.059h.036c.142 0 .27.048.399.135.146.129.264.288.344.465.09.199.14.4.153.667v.004c.007.134.006.2-.002.266v.08c-.03.007-.056.018-.083.024-.152.055-.274.135-.393.2.012-.09.013-.18.003-.267v-.015c-.012-.133-.04-.2-.082-.333a.613.613 0 00-.166-.267.248.248 0 00-.183-.064h-.021c-.071.006-.13.04-.186.132a.552.552 0 00-.12.27.944.944 0 00-.023.33v.015c.012.135.037.2.08.334.046.134.098.2.166.268.01.009.02.018.034.024-.07.057-.117.07-.176.136a.304.304 0 01-.131.068 2.62 2.62 0 01-.275-.402 1.772 1.772 0 01-.155-.667 1.759 1.759 0 01.08-.668 1.43 1.43 0 01.283-.535c.128-.133.26-.2.418-.2zm1.37 1.706c.332 0 .733.065 1.216.399.293.2.523.269 1.052.468h.003c.255.136.405.266.478.399v-.131a.571.571 0 01.016.47c-.123.31-.516.643-1.063.842v.002c-.268.135-.501.333-.775.465-.276.135-.588.292-1.012.267a1.139 1.139 0 01-.448-.067 3.566 3.566 0 01-.322-.198c-.195-.135-.363-.332-.612-.465v-.005h-.005c-.4-.246-.616-.512-.686-.71-.07-.268-.005-.47.193-.6.224-.135.38-.271.483-.336.104-.074.143-.102.176-.131h.002v-.003c.169-.202.436-.47.839-.601.139-.036.294-.065.466-.065zm2.8 2.142c.358 1.417 1.196 3.475 1.735 4.473.286.534.855 1.659 1.102 3.024.156-.005.33.018.513.064.646-1.671-.546-3.467-1.089-3.966-.22-.2-.232-.335-.123-.335.59.534 1.365 1.572 1.646 2.757.13.535.16 1.104.021 1.67.067.028.135.06.205.067 1.032.534 1.413.938 1.23 1.537v-.043c-.06-.003-.12 0-.18 0h-.016c.151-.467-.182-.825-1.065-1.224-.915-.4-1.646-.336-1.77.465-.008.043-.013.066-.018.135-.068.023-.139.053-.209.064-.43.268-.662.669-.793 1.187-.13.533-.17 1.156-.205 1.869v.003c-.02.334-.17.838-.319 1.35-1.5 1.072-3.58 1.538-5.348.334a2.645 2.645 0 00-.402-.533 1.45 1.45 0 00-.275-.333c.182 0 .338-.03.465-.067a.615.615 0 00.314-.334c.108-.267 0-.697-.345-1.163-.345-.467-.931-.995-1.788-1.521-.63-.4-.986-.87-1.15-1.396-.165-.534-.143-1.085-.015-1.645.245-1.07.873-2.11 1.274-2.763.107-.065.037.135-.408.974-.396.751-1.14 2.497-.122 3.854a8.123 8.123 0 01.647-2.876c.564-1.278 1.743-3.504 1.836-5.268.048.036.217.135.289.202.218.133.38.333.59.465.21.201.477.335.876.335.039.003.075.006.11.006.412 0 .73-.134.997-.268.29-.134.52-.334.74-.4h.005c.467-.135.835-.402 1.044-.7zm2.185 8.958c.037.6.343 1.245.882 1.377.588.134 1.434-.333 1.791-.765l.211-.01c.315-.007.577.01.847.268l.003.003c.208.199.305.53.391.876.085.4.154.78.409 1.066.486.527.645.906.636 1.14l.003-.007v.018l-.003-.012c-.015.262-.185.396-.498.595-.63.401-1.746.712-2.457 1.57-.618.737-1.37 1.14-2.036 1.191-.664.053-1.237-.2-1.574-.898l-.005-.003c-.21-.4-.12-1.025.056-1.69.176-.668.428-1.344.463-1.897.037-.714.076-1.335.195-1.814.12-.465.308-.797.641-.984l.045-.022zm-10.814.049h.01c.053 0 .105.005.157.014.376.055.706.333 1.023.752l.91 1.664.003.003c.243.533.754 1.064 1.189 1.637.434.598.77 1.131.729 1.57v.006c-.057.744-.48 1.148-1.125 1.294-.645.135-1.52.002-2.395-.464-.968-.536-2.118-.469-2.857-.602-.369-.066-.61-.2-.723-.4-.11-.2-.113-.602.123-1.23v-.004l.002-.003c.117-.334.03-.752-.027-1.118-.055-.401-.083-.71.043-.94.16-.334.396-.4.69-.533.294-.135.64-.202.915-.47h.002v-.002c.256-.268.445-.601.668-.838.19-.201.38-.336.663-.336zm7.159-9.074c-.435.201-.945.535-1.488.535-.542 0-.97-.267-1.28-.466-.154-.134-.28-.268-.373-.335-.164-.134-.144-.333-.074-.333.109.016.129.134.199.2.096.066.215.2.36.333.292.2.68.467 1.167.467.485 0 1.053-.267 1.398-.466.195-.135.445-.334.648-.467.156-.136.149-.267.279-.267.128.016.034.134-.147.332a8.097 8.097 0 01-.69.468zm-1.082-1.583V5.64c-.006-.02.013-.042.029-.05.074-.043.18-.027.26.004.063 0 .16.067.15.135-.006.049-.085.066-.135.066-.055 0-.092-.043-.141-.068-.052-.018-.146-.008-.163-.065zm-.551 0c-.02.058-.113.049-.166.066-.047.025-.086.068-.14.068-.05 0-.13-.02-.136-.068-.01-.066.088-.133.15-.133.08-.031.184-.047.259-.005.019.009.036.03.03.05v.02h.003z"
                />
              </svg>
              <span>Any Linux server</span>
            </div>
          </div>
        </div>

        <!-- Dockerfile / VPS callout -->
        <div class="mb-8 rounded-xl border border-crush-orange/20 bg-crush-orange/5 px-6 py-4 flex items-start gap-4">
          <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight">
            <svg viewBox="0 0 24 24" class="h-4 w-4 fill-none stroke-current stroke-2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M9 21V9"/>
            </svg>
          </div>
          <div>
            <p class="text-sm font-semibold text-white">Already have a Dockerfile? Crush runs it as-is.</p>
            <p class="mt-1 text-sm text-crush-textMuted leading-relaxed">
              Crush builds out-of-the-box hostable environments from your project — no config needed.
              When you're ready to deploy, export a standard
              <code class="px-1 py-0.5 rounded bg-crush-surface border border-crush-border text-crush-text text-xs">Dockerfile</code>
              or
              <code class="px-1 py-0.5 rounded bg-crush-surface border border-crush-border text-crush-text text-xs">docker-compose.yml</code>
              to ship to any VPS, CI pipeline, or cloud provider.
            </p>
          </div>
        </div>

        <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          @for (cloud of clouds; track cloud.name) {
            <div
              class="rounded-lg border border-crush-border/30 bg-crush-surface/20 px-5 py-4 flex items-center gap-4 hover:border-crush-orange/30 transition-colors group"
            >
              <svg
                role="img"
                viewBox="0 0 24 24"
                class="h-6 w-6 fill-current text-crush-orangeLight group-hover:text-crush-orange transition-colors shrink-0"
              >
                <path [attr.d]="cloud.iconPath" />
              </svg>
              <span class="text-sm font-semibold text-crush-text">{{ cloud.name }}</span>
            </div>
          }
        </div>

        <div class="mt-10 text-center">
          <a
            routerLink="/docs/docker-migration"
            hlmBtn
            variant="outline"
            class="border-crush-border text-crush-text hover:bg-crush-surface gap-2"
          >
            Learn about deployment
            <hlm-icon name="lucideExternalLink" size="sm" />
          </a>
        </div>
      </div>
    </section>

    <hr class="max-w-7xl mx-auto border-crush-border/30" />

    <!-- Install -->
    <section class="py-20 sm:py-28">
      <div class="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto max-w-2xl text-center mb-10">
          <h2 class="text-3xl font-bold text-white sm:text-4xl">Install in one command</h2>
          <p class="mt-4 text-lg text-crush-textMuted">
            Pick your platform and you're running in seconds
          </p>
        </div>
        <app-install-block />
        <div class="mt-8 text-center">
          <a
            routerLink="/docs/installation"
            class="text-sm text-crush-orange hover:text-crush-orangeLight transition-colors"
          >
            View all 10 install methods &rarr;
          </a>
        </div>
      </div>
    </section>

    <!-- CTA -->
    <section class="border-t border-crush-border/30 py-28 relative overflow-hidden select-none">
      <!-- Glow background -->
      <div
        class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[350px] h-[350px] bg-crush-orange/5 blur-[120px] pointer-events-none rounded-full"
      ></div>

      <div class="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 relative">
        <div
          class="relative overflow-hidden rounded-3xl border border-crush-border/40 bg-gradient-to-b from-crush-surface/40 to-crush-surface/10 p-12 sm:p-16 text-center shadow-2xl hover:border-crush-orange/30 transition-all duration-500 group"
        >
          <!-- Glass effect top-right glow -->
          <div
            class="absolute -right-16 -top-16 w-48 h-48 rounded-full bg-crush-orange/3 blur-2xl group-hover:bg-crush-orange/6 transition-all duration-500 pointer-events-none"
          ></div>

          <h2 class="text-3xl font-extrabold tracking-tight text-white sm:text-5xl mb-4">
            Start <span class="gradient-text">crushing</span>
          </h2>
          <p
            class="mt-4 text-base sm:text-lg text-crush-textMuted max-w-xl mx-auto leading-relaxed"
          >
            Your first daemon-free native Windows container is 5 minutes away. Experience zero-VM
            overhead speeds today.
          </p>

          <div class="mt-10 flex items-center justify-center gap-4 flex-wrap">
            <a
              routerLink="/docs/getting-started"
              class="inline-flex items-center gap-2 px-8 py-3.5 rounded-lg text-sm font-bold text-white bg-crush-orange hover:bg-crush-orangeLight transition-all hover:scale-105 active:scale-95 duration-200 cursor-pointer shadow-lg shadow-crush-orange/10 select-none outline-none"
            >
              Get Started
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-none stroke-current stroke-2.5">
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </a>
          </div>

          <!-- Divider -->
          <div class="mt-12 border-t border-crush-border/30"></div>

          <!-- Contribute + Star -->
          <div class="mt-8 flex flex-wrap items-center justify-center gap-6 text-sm">
            <a
              href="https://github.com/Chidi09/crush"
              target="_blank"
              rel="noopener"
              class="flex items-center gap-2 text-crush-textMuted hover:text-crush-orange transition-colors duration-200"
            >
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-current"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>
              Star us on GitHub
              <span class="px-1.5 py-0.5 rounded-full bg-crush-surface border border-crush-border/60 text-[10px] font-mono text-crush-text">⭐</span>
            </a>
            <span class="w-px h-4 bg-crush-border/50 hidden sm:block"></span>
            <a
              href="https://github.com/Chidi09/crush/issues"
              target="_blank"
              rel="noopener"
              class="flex items-center gap-1.5 text-crush-textMuted hover:text-crush-orange transition-colors duration-200"
            >
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-none stroke-current stroke-2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
              Contribute — browse open issues
            </a>
            <span class="w-px h-4 bg-crush-border/50 hidden sm:block"></span>
            <span class="flex items-center gap-1.5 text-crush-textMuted">
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-current text-[#5865F2]"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057c.002.022.015.043.03.053a19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>
              Discord — coming soon
            </span>
          </div>

          <!-- Built by -->
          <div class="mt-8 flex flex-col items-center gap-4">
            <p class="text-[11px] uppercase tracking-widest text-crush-textMuted font-semibold">Built by</p>
            <div class="flex items-center gap-3">
              <!-- Founder -->
              <a
                href="https://github.com/Chidi09"
                target="_blank"
                rel="noopener"
                class="flex items-center gap-2.5 rounded-full border border-crush-border/50 bg-crush-surface/40 px-4 py-2 hover:border-crush-orange/40 hover:bg-crush-surface/60 transition-all duration-200 group"
              >
                <div class="h-6 w-6 rounded-full bg-gradient-to-br from-crush-orange to-crush-orangeLight flex items-center justify-center text-white text-[11px] font-bold shrink-0">C</div>
                <span class="text-sm font-medium text-crush-text group-hover:text-white transition-colors">Chidi</span>
                <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2 text-crush-textMuted group-hover:text-crush-orange transition-colors" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
              </a>
              <!-- Join -->
              <a
                href="https://github.com/Chidi09/crush/issues"
                target="_blank"
                rel="noopener"
                class="flex items-center gap-2 rounded-full border border-dashed border-crush-border/50 bg-transparent px-4 py-2 text-sm text-crush-textMuted hover:border-crush-orange/50 hover:text-crush-orange hover:bg-crush-orange/5 transition-all duration-200"
              >
                <span class="text-base leading-none font-light">+</span>
                You? Join us
              </a>
            </div>
          </div>

        </div>
      </div>
    </section>
  `,
})
export default class IndexPage implements OnInit {
  clouds = [
    {
      name: 'AWS EC2 / ECS / EKS',
      iconPath:
        'M6.763 10.036c0 .296.032.535.088.71.064.176.144.368.256.576.04.063.056.127.056.183 0 .08-.048.16-.152.24l-.503.335a.383.383 0 0 1-.208.072c-.08 0-.16-.04-.239-.112a2.47 2.47 0 0 1-.287-.375 6.18 6.18 0 0 1-.248-.471c-.622.734-1.405 1.101-2.347 1.101-.67 0-1.205-.191-1.596-.574-.391-.384-.59-.894-.59-1.533 0-.678.239-1.23.726-1.644.487-.415 1.133-.623 1.955-.623.272 0 .551.024.846.064.296.04.6.104.918.176v-.583c0-.607-.127-1.03-.375-1.277-.255-.248-.686-.367-1.3-.367-.28 0-.568.031-.863.103-.295.072-.583.16-.862.272a2.287 2.287 0 0 1-.28.104.488.488 0 0 1-.127.023c-.112 0-.168-.08-.168-.247v-.391c0-.128.016-.224.056-.28a.597.597 0 0 1 .224-.167c.279-.144.614-.264 1.005-.36a4.84 4.84 0 0 1 1.246-.151c.95 0 1.644.216 2.091.647.439.43.662 1.085.662 1.963v2.586zm-3.24 1.214c.263 0 .534-.048.822-.144.287-.096.543-.271.758-.51.128-.152.224-.32.272-.512.047-.191.08-.423.08-.694v-.335a6.66 6.66 0 0 0-.735-.136 6.02 6.02 0 0 0-.75-.048c-.535 0-.926.104-1.19.32-.263.215-.39.518-.39.917 0 .375.095.655.295.846.191.2.47.296.838.296zm6.41.862c-.144 0-.24-.024-.304-.08-.064-.048-.12-.16-.168-.311L7.586 5.55a1.398 1.398 0 0 1-.072-.32c0-.128.064-.2.191-.2h.783c.151 0 .255.025.31.08.065.048.113.16.16.312l1.342 5.284 1.245-5.284c.04-.16.088-.264.151-.312a.549.549 0 0 1 .32-.08h.638c.152 0 .256.025.32.08.063.048.12.16.151.312l1.261 5.348 1.381-5.348c.048-.16.104-.264.16-.312a.52.52 0 0 1 .311-.08h.743c.127 0 .2.06',
    },
    {
      name: 'Azure Container Instances / AKS',
      iconPath:
        'M22.379 23.343a1.62 1.62 0 0 0 1.536-2.14v.002L17.35 1.76A1.62 1.62 0 0 0 15.816.657H8.184A1.62 1.62 0 0 0 6.65 1.76L.086 21.204a1.62 1.62 0 0 0 1.536 2.139h4.741a1.62 1.62 0 0 0 1.535-1.103l.977-2.892 4.947 3.32a1.62 1.62 0 0 0 1.954 0l4.947-3.32.977 2.892a1.62 1.62 0 0 0 1.535 1.103Z',
    },
    {
      name: 'GCP Cloud Run / GKE',
      iconPath:
        'M12.19 2.38a9.344 9.344 0 0 0-9.234 6.893c.053-.02-.055.013 0 0-3.875 2.551-3.922 8.11-.247 10.941l.006-.007-.007.03a6.717 6.717 0 0 0 4.077 1.356h5.173l.03.03h5.192c6.687.053 9.376-8.605 3.835-12.35a9.365 9.365 0 0 0-2.821-4.552l-.043.043.006-.05A9.344 9.344 0 0 0 12.19 2.38z',
    },
    {
      name: 'DigitalOcean Droplets',
      iconPath:
        'M12.04 0C5.408-.02.005 5.37.005 11.992h4.638c0-4.923 4.882-8.731 10.064-6.855a6.95 6.95 0 014.147 4.148c1.889 5.177-1.924 10.055-6.84 10.064v-4.61H7.391v4.623h4.61V24c7.108 0 11.99-4.884 11.99-12.008c0-6.621-5.385-11.986-11.95-11.992zm-5.753 19.38v2.308h2.3v-2.308zm0-4.62v2.31h2.3v-2.31zm4.614 4.62v2.308h2.306v-2.308zm0-4.62v2.31h2.306v-2.31z',
    },
    {
      name: 'Hetzner VPS',
      iconPath:
        'M0 0v24h24V0H0zm4.602 4.025h2.244c.509 0 .716.215.716.717v5.64h8.883v-5.64c0-.509.215-.717.717-.717h2.229c.5 0 .71.23.724.717v14.516c-.014.487-.224.717-.724.717h-2.229c-.502 0-.717-.23-.717-.717v-5.467H7.562v5.467c0 .487-.207.717-.716.717H4.602c-.5 0-.71-.23-.724-.717V4.742c.014-.487.224-.717.724-.717z',
    },
    {
      name: 'Any bare metal Linux',
      iconPath:
        'M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.134.063.198.114.333l.003.003c.391.778 1.113 1.132 1.884 1.071.771-.06 1.592-.536 2.257-1.306.631-.765 1.683-1.084 2.378-1.503.348-.199.629-.469.649-.853.023-.4-.2-.811-.714-1.376v-.097l-.003-.003c-.17-.2-.25-.535-.338-.926-.085-.401-.182-.786-.492-1.046h-.003c-.059-.054-.123-.067-.188-.135a.357.357 0 00-.19-.064c.431-1.278.264-2.55-.173-3.694-.533-1.41-1.465-2.638-2.175-3.483-.796-1.005-1.576-1.957-1.56-3.368.026-2.152.236-6.133-3.544-6.139zm.529 3.405h.013c.213 0 .396.062.584.198.19.135.33.332.438.533.105.259.158.459.166.724 0-.02.006-.04.006-.06v.105a.086.086 0 01-.004-.021l-.004-.024a1.807 1.807 0 01-.15.706.953.953 0 01-.213.335.71.71 0 00-.088-.042c-.104-.045-.198-.064-.284-.133a1.312 1.312 0 00-.22-.066c.05-.06.146-.133.183-.198.053-.128.082-.264.088-.402v-.02a1.21 1.21 0 00-.061-.4c-.045-.134-.101-.2-.183-.333-.084-.066-.167-.132-.267-.132h-.016c-.093 0-.176.03-.262.132a.8.8 0 00-.205.334 1.18 1.18 0 00-.09.4v.019c.002.089.008.179.02.267-.193-.067-.438-.135-.607-.202a1.635 1.635 0 01-.018-.2v-.02a1.772 1.772 0 01.15-.768c.082-.22.232-.406.43-.533a.985.985 0 01.594-.2zm-2.962.059h.036c.142 0 .27.048.399.135.146.129.264.288.344.465.09.199.14.4.153.667v.004c.007.134.006.2-.002.266v.08c-.03.007-.056.018-.083.024-.152.055-.274.135-.393.2.012-.09.013-.18.003-.267v-.015c-.012-.133-.04-.2-.082-.333a.613.613 0 00-.166-.267.248.248 0 00-.183-.064h-.021c-.071.006-.13.04-.186.132a.552.552 0 00-.12.27.944.944 0 00-.023.33v.015c.012.135.037.2.08.334.046.134.098.2.166.268.01.009.02.018.034.024-.07.057-.117.07-.176.136a.304.304 0 01-.131.068 2.62 2.62 0 01-.275-.402 1.772 1.772 0 01-.155-.667 1.759 1.759 0 01.08-.668 1.43 1.43 0 01.283-.535c.128-.133.26-.2.418-.2zm1.37 1.706c.332 0 .733.065 1.216.399.293.2.523.269 1.052.468h.003c.255.136.405.266.478.399v-.131a.571.571 0 01.016.47c-.123.31-.516.643-1.063.842v.002c-.268.135-.501.333-.775.465-.276.135-.588.292-1.012.267a1.139 1.139 0 01-.448-.067 3.566 3.566 0 01-.322-.198c-.195-.135-.363-.332-.612-.465v-.005h-.005c-.4-.246-.616-.512-.686-.71-.07-.268-.005-.47.193-.6.224-.135.38-.271.483-.336.104-.074.143-.102.176-.131h.002v-.003c.169-.202.436-.47.839-.601.139-.036.294-.065.466-.065zm2.8 2.142c.358 1.417 1.196 3.475 1.735 4.473.286.534.855 1.659 1.102 3.024.156-.005.33.018.513.064.646-1.671-.546-3.467-1.089-3.966-.22-.2-.232-.335-.123-.335.59.534 1.365 1.572 1.646 2.757.13.535.16 1.104.021 1.67.067.028.135.06.205.067 1.032.534 1.413.938 1.23 1.537v-.043c-.06-.003-.12 0-.18 0h-.016c.151-.467-.182-.825-1.065-1.224-.915-.4-1.646-.336-1.77.465-.008.043-.013.066-.018.135-.068.023-.139.053-.209.064-.43.268-.662.669-.793 1.187-.13.533-.17 1.156-.205 1.869v.003c-.02.334-.17.838-.319 1.35-1.5 1.072-3.58 1.538-5.348.334a2.645 2.645 0 00-.402-.533 1.45 1.45 0 00-.275-.333c.182 0 .338-.03.465-.067a.615.615 0 00.314-.334c.108-.267 0-.697-.345-1.163-.345-.467-.931-.995-1.788-1.521-.63-.4-.986-.87-1.15-1.396-.165-.534-.143-1.085-.015-1.645.245-1.07.873-2.11 1.274-2.763.107-.065.037.135-.408.974-.396.751-1.14 2.497-.122 3.854a8.123 8.123 0 01.647-2.876c.564-1.278 1.743-3.504 1.836-5.268.048.036.217.135.289.202.218.133.38.333.59.465.21.201.477.335.876.335.039.003.075.006.11.006.412 0 .73-.134.997-.268.29-.134.52-.334.74-.4h.005c.467-.135.835-.402 1.044-.7zm2.185 8.958c.037.6.343 1.245.882 1.377.588.134 1.434-.333 1.791-.765l.211-.01c.315-.007.577.01.847.268l.003.003c.208.199.305.53.391.876.085.4.154.78.409 1.066.486.527.645.906.636 1.14l.003-.007v.018l-.003-.012c-.015.262-.185.396-.498.595-.63.401-1.746.712-2.457 1.57-.618.737-1.37 1.14-2.036 1.191-.664.053-1.237-.2-1.574-.898l-.005-.003c-.21-.4-.12-1.025.056-1.69.176-.668.428-1.344.463-1.897.037-.714.076-1.335.195-1.814.12-.465.308-.797.641-.984l.045-.022zm-10.814.049h.01c.053 0 .105.005.157.014.376.055.706.333 1.023.752l.91 1.664.003.003c.243.533.754 1.064 1.189 1.637.434.598.77 1.131.729 1.57v.006c-.057.744-.48 1.148-1.125 1.294-.645.135-1.52.002-2.395-.464-.968-.536-2.118-.469-2.857-.602-.369-.066-.61-.2-.723-.4-.11-.2-.113-.602.123-1.23v-.004l.002-.003c.117-.334.03-.752-.027-1.118-.055-.401-.083-.71.043-.94.16-.334.396-.4.69-.533.294-.135.64-.202.915-.47h.002v-.002c.256-.268.445-.601.668-.838.19-.201.38-.336.663-.336zm7.159-9.074c-.435.201-.945.535-1.488.535-.542 0-.97-.267-1.28-.466-.154-.134-.28-.268-.373-.335-.164-.134-.144-.333-.074-.333.109.016.129.134.199.2.096.066.215.2.36.333.292.2.68.467 1.167.467.485 0 1.053-.267 1.398-.466.195-.135.445-.334.648-.467.156-.136.149-.267.279-.267.128.016.034.134-.147.332a8.097 8.097 0 01-.69.468zm-1.082-1.583V5.64c-.006-.02.013-.042.029-.05.074-.043.18-.027.26.004.063 0 .16.067.15.135-.006.049-.085.066-.135.066-.055 0-.092-.043-.141-.068-.052-.018-.146-.008-.163-.065zm-.551 0c-.02.058-.113.049-.166.066-.047.025-.086.068-.14.068-.05 0-.13-.02-.136-.068-.01-.066.088-.133.15-.133.08-.031.184-.047.259-.005.019.009.036.03.03.05v.02h.003z',
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Crush — Containers that actually work on Windows');
    this.meta.updateTag({
      name: 'description',
      content:
        'Crush is a native Windows container runtime. Sub-second starts, no WSL2, no VM overhead. Build once, deploy anywhere.',
    });
  }
}
