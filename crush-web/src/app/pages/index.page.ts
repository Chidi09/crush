import { Component, OnInit, Inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Title, Meta } from '@angular/platform-browser';
import { CommonModule, DOCUMENT } from '@angular/common';
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
import { DownloadBlockComponent } from '../components/download-block/download-block.component';
import { ComparisonTableComponent } from '../components/comparison-table/comparison-table.component';

@Component({
  selector: 'page-index',
  standalone: true,
  imports: [
    RouterLink,
    CommonModule,
    HlmButtonDirective,
    HlmIconComponent,
    HlmBadgeDirective,
    HlmCardDirective,
    HlmCardContentDirective,
    HlmCardHeaderDirective,
    HlmCardTitleDirective,
    TerminalComponent,
    DownloadBlockComponent,
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
              >v0.8.1</span
            >
            <span class="text-sm text-crush-textMuted">Lightweight Docker Desktop Alternative</span>
          </div>
          <h1
            class="text-4xl font-extrabold tracking-tight text-white sm:text-5xl lg:text-6xl text-balance"
          >
            Run docker-compose on Windows <span class="gradient-text">without WSL2</span> or Docker
            Desktop
          </h1>
          <p class="mt-6 text-lg leading-8 text-crush-textMuted max-w-2xl mx-auto">
            No VMs, no daemons, no memory hogging. Crush auto-detects your stack and starts your
            development services (Postgres, Redis, pgvector) natively on Windows using Job Objects.
            Fast, lightweight, and ejects to standard Dockerfiles.
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
              href="#download"
              class="inline-flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-semibold text-white bg-crush-surface border border-crush-border hover:bg-crush-surface/80 hover:border-crush-orange/50 hover:text-white transition-colors duration-200 select-none outline-none shadow-lg shadow-crush-orange/5"
            >
              Download — GUI &amp; CLI
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-none stroke-current stroke-2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
            </a>
            <a
              href="https://github.com/Chidi09/crush"
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

    <!-- New in 1.0 -->
    <section class="py-20 sm:py-24 border-b border-crush-border/30 relative overflow-hidden">
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 relative">
        <div class="mx-auto max-w-3xl text-center mb-14 select-none">
          <div
            class="mb-4 inline-flex items-center gap-1.5 rounded-full border border-crush-orange/20 bg-crush-orange/5 px-3 py-1 text-xs font-semibold text-crush-orange uppercase tracking-wider"
          >
            New in 1.0
          </div>
          <h2 class="text-4xl font-extrabold tracking-tight text-white sm:text-5xl">
            The full lifecycle, natively
          </h2>
          <p class="mt-4 text-lg text-crush-textMuted max-w-2xl mx-auto text-balance">
            Develop, integrate, expose, snapshot, and ship with zero downtime — without a single
            container in your dev loop.
          </p>
        </div>

        <div class="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
          @for (f of whatsNew; track f.title) {
            <div
              class="rounded-2xl border border-crush-border/60 bg-card p-6 hover:border-crush-orange/30 transition-colors duration-300"
            >
              <div class="text-xs font-mono text-crush-orange mb-2">{{ f.cmd }}</div>
              <h3 class="text-lg font-bold text-white mb-2">{{ f.title }}</h3>
              <p class="text-sm text-crush-textMuted leading-relaxed">{{ f.body }}</p>
            </div>
          }
        </div>

        <div class="mt-10 text-center">
          <a
            routerLink="/changelog"
            class="inline-flex items-center gap-1.5 text-sm font-semibold text-crush-orange hover:text-crush-orangeLight transition-colors"
          >
            See the full 1.0 changelog →
          </a>
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
          <!-- Card 1: Skip the busywork -->
          <div
            class="relative overflow-hidden rounded-2xl border border-crush-border/70 dark:border-crush-border/40 bg-card p-8 hover:border-crush-orange/30 hover:bg-crush-surface/5 shadow-xl shadow-black/[0.03] dark:shadow-none transition-all duration-300 group flex flex-col justify-between"
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
                Skip the busywork
              </h3>
              <p
                class="text-sm text-crush-textMuted leading-relaxed group-hover:text-crush-text transition-colors duration-300"
              >
                Crush skips <code class="text-crush-orange/90">pnpm install</code> when node_modules
                is fresh, image pack when sources match, build when artifacts are newer. Warm runs
                are seconds; we don't pretend to make your framework faster.
              </p>
            </div>

            <!-- Dynamic UI Graphic -->
            <div
              class="always-dark mt-6 rounded-xl border border-crush-border/40 bg-crush-black/50 p-4 font-mono text-xs select-none relative overflow-hidden group-hover:border-crush-orange/20 transition-all duration-300"
            >
              <div
                class="flex items-center justify-between mb-3 border-b border-crush-border/30 pb-2"
              >
                <span class="text-crush-textMuted font-bold">WARM RUN</span>
                <span
                  class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-500/10 text-emerald-400"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                  Skipped 3 steps
                </span>
              </div>
              <div class="space-y-1.5 text-[10px]">
                <div class="flex justify-between">
                  <span class="text-crush-textMuted">image fingerprint</span>
                  <span class="text-crush-orange font-bold">fresh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-crush-textMuted">node_modules vs lockfile</span>
                  <span class="text-crush-orange font-bold">fresh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-crush-textMuted">build artifact mtime</span>
                  <span class="text-crush-orange font-bold">fresh</span>
                </div>
                <div class="flex justify-between border-t border-crush-border/30 pt-1.5 mt-1.5">
                  <span class="text-crush-text">crush overhead</span>
                  <span class="text-emerald-400 font-bold">~2s</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Card 2: Windows native -->
          <div
            class="relative overflow-hidden rounded-2xl border border-crush-border/70 dark:border-crush-border/40 bg-card p-8 hover:border-crush-orange/30 hover:bg-crush-surface/5 shadow-xl shadow-black/[0.03] dark:shadow-none transition-all duration-300 group flex flex-col justify-between"
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
                Deps already up
              </h3>
              <p
                class="text-sm text-crush-textMuted leading-relaxed group-hover:text-crush-text transition-colors duration-300"
              >
                Crush parses your <code class="text-crush-orange/90">compose.yml</code> /
                <code class="text-crush-orange/90">application.yml</code> and starts Postgres,
                Garnet (Redis-compat), MySQL natively — no containers. pgvector compiles against
                your host PG on first use.
              </p>
            </div>

            <!-- Dynamic UI Graphic -->
            <div
              class="always-dark mt-6 rounded-xl border border-crush-border/40 bg-crush-black/50 p-4 font-mono text-xs select-none relative overflow-hidden group-hover:border-crush-orange/20 transition-all duration-300"
            >
              <div
                class="flex items-center justify-between mb-3 border-b border-crush-border/30 pb-2"
              >
                <span class="text-crush-textMuted font-bold">DEP DRIVERS</span>
                <span class="text-crush-orangeLight text-[10px]">No docker</span>
              </div>
              <div class="space-y-1.5 text-[10px] leading-relaxed">
                <div class="flex items-center justify-between">
                  <span class="text-crush-text">postgres :5432</span>
                  <span class="text-emerald-400">● native</span>
                </div>
                <div class="flex items-center justify-between">
                  <span class="text-crush-text">garnet :6379</span>
                  <span class="text-emerald-400">● native</span>
                </div>
                <div class="flex items-center justify-between">
                  <span class="text-crush-text">vector ext</span>
                  <span class="text-crush-orange">● built</span>
                </div>
                <div
                  class="flex items-center justify-between border-t border-crush-border/30 pt-1.5 mt-1.5"
                >
                  <span class="text-crush-textMuted">from</span>
                  <span class="text-crush-textMuted">infra/docker-compose.yml</span>
                </div>
              </div>
            </div>
          </div>

          <!-- Card 3: AI diagnosis -->
          <div
            class="relative overflow-hidden rounded-2xl border border-crush-border/70 dark:border-crush-border/40 bg-card p-8 hover:border-crush-orange/30 hover:bg-crush-surface/5 shadow-xl shadow-black/[0.03] dark:shadow-none transition-all duration-300 group flex flex-col justify-between"
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
              class="always-dark mt-6 rounded-xl border border-crush-border/40 bg-crush-black/50 p-4 font-mono text-xs select-none relative overflow-hidden group-hover:border-crush-orange/20 transition-all duration-300"
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

    <!-- Stacks -->
    <section class="py-20 sm:py-28 relative overflow-hidden">
      <div
        class="absolute inset-0 bg-gradient-to-b from-transparent via-crush-orange/2 to-transparent pointer-events-none"
      ></div>
      <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 relative">
        <div class="mx-auto max-w-3xl text-center mb-14 select-none">
          <div
            class="mb-4 inline-flex items-center gap-1.5 rounded-full border border-crush-orange/20 bg-crush-orange/5 px-3 py-1 text-xs font-semibold text-crush-orange uppercase tracking-wider"
          >
            Your stack, natively
          </div>
          <h2 class="text-3xl font-bold text-white sm:text-4xl">Supported stacks and frameworks</h2>
          <p class="mt-4 text-lg text-crush-textMuted max-w-2xl mx-auto">
            Zero-config environments for high-performance stacks. Whether you're compiling
            <strong class="text-white font-semibold">Go</strong> binaries, running a
            <strong class="text-white font-semibold">Python FastAPI</strong> backend, or serving an
            <strong class="text-white font-semibold">Angular</strong> frontend via
            <strong class="text-white font-semibold">Fastify</strong>, Crush handles layer caching
            natively on Windows — no WSL2 required.
          </p>
        </div>

        <!-- Interactive Showcase Layout -->
        <div class="grid grid-cols-1 lg:grid-cols-12 gap-8 items-stretch select-none">
          <!-- Left side: Selectors (5 columns on large screen) -->
          <div
            class="lg:col-span-5 flex flex-col gap-2.5 max-h-[580px] overflow-y-auto pr-2 scrollbar-thin"
          >
            @for (stack of stacks; track stack.name; let idx = $index) {
              <button
                (click)="selectedStack.set(idx)"
                class="w-full text-left rounded-xl border p-4 transition-all duration-300 outline-none flex items-start gap-4 group"
                [ngClass]="
                  selectedStack() === idx
                    ? 'border-crush-orange/40 bg-crush-orange/5 shadow-[0_0_20px_rgba(224,85,64,0.06)]'
                    : 'border-crush-border/30 bg-crush-surface/10 hover:border-crush-border/60 hover:bg-crush-surface/20'
                "
              >
                <!-- Icon container -->
                <div
                  class="h-9 w-9 rounded-lg flex items-center justify-center shrink-0 transition-transform duration-300 group-hover:scale-105"
                  [style.background]="stack.color + '15'"
                  [style.border]="
                    '1px solid ' + (selectedStack() === idx ? stack.color : stack.color + '33')
                  "
                >
                  <svg
                    viewBox="0 0 24 24"
                    class="h-4.5 w-4.5 fill-current"
                    [style.color]="stack.color"
                  >
                    <path [attr.d]="stack.iconPath" />
                  </svg>
                </div>
                <!-- Content text -->
                <div class="space-y-1">
                  <div class="flex items-center gap-2">
                    <h4
                      class="text-sm font-bold transition-colors"
                      [ngClass]="
                        selectedStack() === idx
                          ? 'text-white'
                          : 'text-crush-textMuted group-hover:text-white'
                      "
                    >
                      {{ stack.name }}
                    </h4>
                    @if (selectedStack() === idx) {
                      <span
                        class="h-1.5 w-1.5 rounded-full bg-crush-orange shadow-[0_0_8px_rgba(224,85,64,0.8)] animate-pulse"
                      ></span>
                    }
                  </div>
                  <p class="text-[11px] text-crush-textMuted leading-relaxed line-clamp-2">
                    {{ stack.desc }}
                  </p>
                </div>
              </button>
            }
          </div>

          <!-- Right side: macOS High-Fidelity Terminal Mock (7 columns on large screen) -->
          <div class="lg:col-span-7 flex flex-col">
            <div
              class="w-full flex-1 rounded-2xl border border-crush-border/40 bg-[#07070b]/90 backdrop-blur-md shadow-2xl shadow-crush-orange/5 overflow-hidden flex flex-col font-mono text-[11px] leading-relaxed relative border-t-crush-border/60"
            >
              <!-- macOS Window Header Bar -->
              <div
                class="flex items-center justify-between px-4 py-3 bg-[#0d0d12] border-b border-crush-border/30 select-none shrink-0"
              >
                <!-- Window Controls Dots -->
                <div class="flex items-center gap-1.5">
                  <span
                    class="h-3 w-3 rounded-full bg-[#ff5f56] border border-[#e0443e] block"
                  ></span>
                  <span
                    class="h-3 w-3 rounded-full bg-[#ffbd2e] border border-[#dea123] block"
                  ></span>
                  <span
                    class="h-3 w-3 rounded-full bg-[#27c93f] border border-[#1aab29] block"
                  ></span>
                </div>
                <!-- Window Title -->
                <div
                  class="text-[10px] text-crush-textMuted tracking-wider flex items-center gap-1.5 font-sans font-semibold"
                >
                  <svg
                    viewBox="0 0 24 24"
                    class="h-3 w-3 fill-none stroke-current stroke-2.5 text-crush-orangeLight"
                  >
                    <path
                      d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3zM6 21a3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3v12a3 3 0 0 0 3 3z"
                    />
                  </svg>
                  crush-term — {{ stacks[selectedStack()].name }}
                </div>
                <!-- Spacing block -->
                <div class="w-12"></div>
              </div>

              <!-- Terminal Output Display Body -->
              <div
                class="p-6 overflow-y-auto flex-1 font-mono text-left select-text relative min-h-[360px]"
              >
                <!-- Glowing dynamic accent reflection inside terminal -->
                <div
                  class="absolute inset-0 pointer-events-none opacity-[0.03] transition-all duration-700 blur-[80px]"
                  [style.background]="
                    'radial-gradient(circle at 50% 50%, ' +
                    stacks[selectedStack()].color +
                    ', transparent)'
                  "
                ></div>
                <!-- Terminal logs pre-wrap block -->
                <pre
                  class="relative z-10 text-crush-text whitespace-pre-wrap"
                ><code [innerHTML]="stacks[selectedStack()].terminal"></code></pre>
              </div>
            </div>
          </div>
        </div>

        <p class="mt-8 text-center text-xs text-crush-textMuted select-none">
          Any language that compiles to a native binary or runs on a POSIX-compatible runtime works
          out of the box.
          <a
            routerLink="/docs/stacks"
            class="text-crush-orange hover:text-crush-orangeLight transition-colors ml-1 font-semibold"
            >View full compatibility list &rarr;</a
          >
        </p>
      </div>
    </section>

    <hr class="max-w-7xl mx-auto border-crush-border/30" />

    <!-- How It Works -->
    <section class="py-20 sm:py-28 relative overflow-hidden">
      <div class="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto max-w-2xl text-center mb-14 select-none">
          <div
            class="mb-4 inline-flex items-center gap-1.5 rounded-full border border-crush-orange/20 bg-crush-orange/5 px-3 py-1 text-xs font-semibold text-crush-orange uppercase tracking-wider"
          >
            Under the hood
          </div>
          <h2 class="text-3xl font-bold text-white sm:text-4xl">
            Native performance, no VM required
          </h2>
          <p class="mt-4 text-lg text-crush-textMuted">
            What's actually happening when you type <code class="text-crush-orange">crush</code> on
            Windows
          </p>
        </div>

        <div class="grid gap-6 md:grid-cols-2 mb-8">
          <div
            class="rounded-2xl border border-crush-border/70 dark:border-crush-border/40 bg-card p-7 hover:border-crush-orange/30 hover:bg-crush-surface/5 shadow-lg shadow-black/[0.02] dark:shadow-none transition-colors duration-200"
          >
            <div
              class="flex h-10 w-10 items-center justify-center rounded-xl bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight mb-5"
            >
              <svg
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-none stroke-current stroke-2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <rect x="3" y="3" width="18" height="18" rx="2" />
                <path d="M3 9h18M9 21V9" />
              </svg>
            </div>
            <h3 class="text-base font-bold text-white mb-3">Windows Job Objects</h3>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              Windows Job Objects are a first-class NT kernel primitive for grouping processes,
              enforcing resource limits (CPU, memory, I/O), and isolating namespaces. They are the
              Windows equivalent of Linux cgroups — the isolation mechanism Docker uses on Linux.
              Crush builds its entire process isolation layer directly on Job Objects, bypassing any
              hypervisor or VM entirely.
            </p>
          </div>
          <div
            class="rounded-2xl border border-crush-border/70 dark:border-crush-border/40 bg-card p-7 hover:border-crush-orange/30 hover:bg-crush-surface/5 shadow-lg shadow-black/[0.02] dark:shadow-none transition-colors duration-200"
          >
            <div
              class="flex h-10 w-10 items-center justify-center rounded-xl bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight mb-5"
            >
              <svg
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-none stroke-current stroke-2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="3" />
                <path d="M12 2v3M12 19v3M2 12h3M19 12h3" />
                <path
                  d="M4.93 4.93l2.12 2.12M16.95 16.95l2.12 2.12M4.93 19.07l2.12-2.12M16.95 7.05l2.12-2.12"
                />
              </svg>
            </div>
            <h3 class="text-base font-bold text-white mb-3">Zero VM layer — direct syscalls</h3>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              Docker Desktop routes every syscall through a WSL2 Linux kernel running inside a
              Hyper-V VM. Every file read, network packet, and process fork crosses that VM
              boundary. Crush eliminates that layer entirely — your application's syscalls land
              directly on the Windows NT kernel, reducing cold-start from 8–30 seconds to under one
              second and idle memory from ~500 MB to under 20 MB.
            </p>
          </div>
        </div>

        <div
          class="always-dark rounded-2xl border border-crush-border/40 bg-crush-black/60 p-6 font-mono text-xs select-none"
        >
          <div class="mb-4 text-[10px] font-bold uppercase tracking-wider text-crush-textMuted">
            Process architecture comparison
          </div>
          <div class="grid gap-6 md:grid-cols-2">
            <div class="space-y-2">
              <div class="text-[10px] font-bold text-crush-orange uppercase tracking-wide mb-3">
                Crush (native Windows)
              </div>
              <div class="flex flex-wrap items-center gap-1.5">
                <span
                  class="px-2 py-1 rounded bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight text-[10px]"
                  >Your App</span
                >
                <span class="text-crush-textMuted text-[10px]">→</span>
                <span
                  class="px-2 py-1 rounded bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight text-[10px]"
                  >Job Object</span
                >
                <span class="text-crush-textMuted text-[10px]">→</span>
                <span
                  class="px-2 py-1 rounded bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight text-[10px]"
                  >NT Kernel</span
                >
              </div>
              <div class="text-[9px] text-emerald-400 pt-1">
                ✓ no VM hop · ~30 MB idle (no daemon)
              </div>
            </div>
            <div class="space-y-2 opacity-50">
              <div class="text-[10px] font-bold text-crush-textMuted uppercase tracking-wide mb-3">
                Docker Desktop (VM-based)
              </div>
              <div class="flex flex-wrap items-center gap-1.5">
                <span
                  class="px-2 py-1 rounded bg-crush-surface border border-crush-border/30 text-crush-textMuted text-[10px]"
                  >Your App</span
                >
                <span class="text-crush-textMuted text-[10px]">→</span>
                <span
                  class="px-2 py-1 rounded bg-crush-surface border border-crush-border/30 text-crush-textMuted text-[10px]"
                  >WSL2 VM</span
                >
                <span class="text-crush-textMuted text-[10px]">→</span>
                <span
                  class="px-2 py-1 rounded bg-crush-surface border border-crush-border/30 text-crush-textMuted text-[10px]"
                  >Linux Kernel</span
                >
                <span class="text-crush-textMuted text-[10px]">→</span>
                <span
                  class="px-2 py-1 rounded bg-crush-surface border border-crush-border/30 text-crush-textMuted text-[10px]"
                  >NT Kernel</span
                >
              </div>
              <div class="text-[9px] text-red-400/70 pt-1">✗ 8–30s cold start · ~500 MB idle</div>
            </div>
          </div>
        </div>
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
        <div
          class="mb-8 rounded-xl border border-crush-orange/20 bg-crush-orange/5 px-6 py-4 flex items-start gap-4"
        >
          <div
            class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-crush-orange/10 border border-crush-orange/20 text-crush-orangeLight"
          >
            <svg
              viewBox="0 0 24 24"
              class="h-4 w-4 fill-none stroke-current stroke-2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="3" y="3" width="18" height="18" rx="2" />
              <path d="M3 9h18M9 21V9" />
            </svg>
          </div>
          <div>
            <p class="text-sm font-semibold text-white">
              Already have a Dockerfile? Crush runs it as-is.
            </p>
            <p class="mt-1 text-sm text-crush-textMuted leading-relaxed">
              Crush builds out-of-the-box hostable environments from your project — no config
              needed. When you're ready to deploy, export a standard
              <code
                class="px-1 py-0.5 rounded bg-crush-surface border border-crush-border text-crush-text text-xs"
                >Dockerfile</code
              >
              or
              <code
                class="px-1 py-0.5 rounded bg-crush-surface border border-crush-border text-crush-text text-xs"
                >docker-compose.yml</code
              >
              to ship to any VPS, CI pipeline, or cloud provider.
            </p>
          </div>
        </div>

        <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          @for (cloud of clouds; track cloud.name) {
            <div
              class="rounded-lg border border-crush-border/70 dark:border-crush-border/30 bg-card px-5 py-4 flex items-center gap-4 hover:border-crush-orange/30 hover:bg-crush-surface/5 shadow-md shadow-black/[0.02] dark:shadow-none transition-colors group"
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
    <section id="download" class="py-20 sm:py-28">
      <div class="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8">
        <div class="mx-auto max-w-2xl text-center mb-12">
          <h2 class="text-3xl font-bold text-white sm:text-4xl">Download Crush</h2>
          <p class="mt-4 text-lg text-crush-textMuted">
            Get the desktop app or the CLI — pick your platform and you're running in seconds.
          </p>
        </div>
        <app-download-block />
        <div class="mt-8 text-center">
          <a
            routerLink="/docs/installation"
            class="text-sm text-crush-orange hover:text-crush-orangeLight transition-colors"
          >
            Full installation guide &amp; all package managers &rarr;
          </a>
        </div>
      </div>
    </section>

    <hr class="max-w-7xl mx-auto border-crush-border/30" />

    <!-- FAQ -->
    <section class="py-20 sm:py-28 relative overflow-hidden font-sans">
      <!-- Background Ambient Light -->
      <div
        class="absolute -right-32 top-1/2 -translate-y-1/2 w-80 h-80 bg-crush-orange/3 blur-[120px] pointer-events-none rounded-full"
      ></div>

      <div class="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 relative">
        <div class="mx-auto max-w-2xl text-center mb-16 select-none">
          <div
            class="mb-4 inline-flex items-center gap-1.5 rounded-full border border-crush-orange/20 bg-crush-orange/5 px-3 py-1 text-xs font-semibold text-crush-orange uppercase tracking-wider"
          >
            FAQ
          </div>
          <h2 class="text-3xl font-extrabold text-white sm:text-4xl tracking-tight">
            Frequently Asked Questions
          </h2>
          <p
            class="mt-4 text-base sm:text-lg text-crush-textMuted max-w-xl mx-auto leading-relaxed"
          >
            Everything you need to know about Crush as a daemonless, native Windows container
            runtime.
          </p>
        </div>

        <!-- SpartanUI styled Accordion Container -->
        <div class="divide-y divide-crush-border/30 border-t border-b border-crush-border/30">
          @for (faq of faqs; track faq.q; let idx = $index) {
            <div class="group py-2">
              <button
                (click)="toggleFaq(idx)"
                class="flex w-full items-center justify-between py-4 text-left font-semibold text-crush-text hover:text-crush-orangeLight transition-colors duration-200 select-none outline-none group"
              >
                <span class="text-sm sm:text-base tracking-wide flex items-center gap-3">
                  <!-- Bullet/State indicator -->
                  <span
                    class="h-1.5 w-1.5 rounded-full transition-all duration-300"
                    [ngClass]="
                      activeFaq() === idx
                        ? 'bg-crush-orange scale-125 shadow-[0_0_8px_rgba(224,85,64,0.8)]'
                        : 'bg-crush-border/80 group-hover:bg-crush-text'
                    "
                  ></span>
                  {{ faq.q }}
                </span>

                <!-- Rotating Chevron -->
                <svg
                  viewBox="0 0 24 24"
                  class="h-5 w-5 fill-none stroke-current stroke-2 text-crush-textMuted group-hover:text-crush-text transition-transform duration-300"
                  [ngClass]="activeFaq() === idx ? 'rotate-180 text-crush-orange' : 'rotate-0'"
                >
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </button>

              <!-- Collapsible Content -->
              <div
                class="overflow-hidden transition-all duration-300 ease-in-out"
                [ngClass]="
                  activeFaq() === idx ? 'max-h-52 opacity-100 pb-5 pl-4.5' : 'max-h-0 opacity-0'
                "
              >
                <p class="text-sm text-crush-textMuted leading-relaxed pr-4 select-text">
                  {{ faq.a }}
                </p>
              </div>
            </div>
          }
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
          class="relative overflow-hidden rounded-3xl border border-crush-border/70 dark:border-crush-border/40 bg-card p-12 sm:p-16 text-center shadow-2xl shadow-black/[0.04] dark:shadow-none hover:border-crush-orange/30 transition-all duration-500 group"
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
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-current">
                <path
                  d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"
                />
              </svg>
              Star us on GitHub
              <span
                class="px-1.5 py-0.5 rounded-full bg-crush-surface border border-crush-border/60 text-[10px] font-mono text-crush-text"
                >⭐</span
              >
            </a>
            <span class="w-px h-4 bg-crush-border/50 hidden sm:block"></span>
            <a
              href="https://github.com/Chidi09/crush/issues"
              target="_blank"
              rel="noopener"
              class="flex items-center gap-1.5 text-crush-textMuted hover:text-crush-orange transition-colors duration-200"
            >
              <svg
                viewBox="0 0 24 24"
                class="h-4 w-4 fill-none stroke-current stroke-2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="8" x2="12" y2="12" />
                <line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
              Contribute — browse open issues
            </a>
            <span class="w-px h-4 bg-crush-border/50 hidden sm:block"></span>
            <span class="flex items-center gap-1.5 text-crush-textMuted">
              <svg viewBox="0 0 24 24" class="h-4 w-4 fill-current text-[#5865F2]">
                <path
                  d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057c.002.022.015.043.03.053a19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"
                />
              </svg>
              Discord — coming soon
            </span>
          </div>

          <!-- Built by -->
          <div class="mt-8 flex flex-col items-center gap-4">
            <p class="text-[11px] uppercase tracking-widest text-crush-textMuted font-semibold">
              Built by
            </p>
            <div class="flex items-center gap-3">
              <!-- Founder -->
              <a
                href="https://github.com/Chidi09"
                target="_blank"
                rel="noopener"
                class="flex items-center gap-2.5 rounded-full border border-crush-border/50 bg-crush-surface/40 px-4 py-2 hover:border-crush-orange/40 hover:bg-crush-surface/60 transition-all duration-200 group"
              >
                <img
                  src="/founder.png"
                  alt="Chidi"
                  class="h-6 w-6 rounded-full shrink-0 object-cover border border-crush-orange/20"
                />
                <span
                  class="text-sm font-medium text-crush-text group-hover:text-white transition-colors"
                  >Chidi</span
                >
                <svg
                  viewBox="0 0 24 24"
                  class="h-3 w-3 fill-none stroke-current stroke-2 text-crush-textMuted group-hover:text-crush-orange transition-colors"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                  <polyline points="15 3 21 3 21 9" />
                  <line x1="10" y1="14" x2="21" y2="3" />
                </svg>
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
  activeFaq = signal<number | null>(null);
  selectedStack = signal<number>(0);

  // Headline capabilities introduced in 1.0 (full list lives in /changelog).
  whatsNew = [
    {
      cmd: 'crush deploy --strategy blue-green',
      title: 'Zero-downtime deploys',
      body: 'Crush brings the new release up beside the old, health-checks it, then atomically flips traffic and drains the old one. Auto-rolls back if the new release is unhealthy.',
    },
    {
      cmd: 'crush tunnel  ·  crush run --tunnel',
      title: 'Localhost tunneling',
      body: 'Expose a local port for webhooks (Paystack, Stripe, Clerk) over a free cloudflared tunnel — no account, no domain. Crush detects when your project needs one.',
    },
    {
      cmd: 'crush db snapshot  ·  crush db restore',
      title: 'Database time machine',
      body: 'Freeze and restore exact Postgres/MySQL state in one command. Wraps native pg_dump/mysqldump and finds your connection automatically.',
    },
    {
      cmd: 'crush mail',
      title: 'Local mail catcher',
      body: 'An embedded SMTP sink on :1025 captures every dev email instead of sending it, and renders it in the GUI Mailbox tab. SMTP env injected for you.',
    },
    {
      cmd: 'crush lint',
      title: 'Cross-OS eject linter',
      body: 'Catches case-sensitive import paths that pass on Windows but break in a Linux container — before CI does. Runs automatically on crush eject.',
    },
    {
      cmd: 'crush detect',
      title: 'Managed service detection',
      body: 'Surfaces the BaaS/managed services your app talks to — Supabase, Firebase, Neon, Clerk, Stripe, Paystack, Sentry and more — read from your env files.',
    },
  ];

  toggleFaq(idx: number): void {
    this.activeFaq.set(this.activeFaq() === idx ? null : idx);
  }
  clouds = [
    {
      name: 'AWS EC2 / ECS / EKS',
      iconPath: 'M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z',
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

  stacks = [
    {
      name: 'Go',
      color: '#00ADD8',
      iconPath:
        'M1.811 10.231c-.047 0-.058-.023-.035-.059l.246-.315c.023-.035.081-.058.128-.058h4.172c.046 0 .058.035.035.07l-.199.303c-.023.036-.082.07-.117.07zM.047 11.306c-.047 0-.059-.023-.035-.058l.245-.316c.023-.035.082-.058.129-.058h5.328c.047 0 .07.035.058.07l-.093.28c-.012.047-.058.07-.105.07zm2.828 1.075c-.047 0-.059-.035-.035-.07l.163-.292c.023-.035.07-.07.117-.07h2.337c.047 0 .07.035.07.082l-.023.28c0 .047-.047.082-.082.082zm12.129-2.36c-.736.187-1.239.327-1.963.514-.176.046-.187.058-.34-.117-.174-.199-.303-.327-.548-.444-.737-.362-1.45-.257-2.115.175-.795.514-1.204 1.274-1.192 2.22.011.935.654 1.706 1.577 1.835.795.105 1.46-.175 1.987-.77.105-.13.198-.27.315-.434H10.47c-.245 0-.304-.152-.222-.35.152-.362.432-.97.596-1.274a.315.315 0 01.292-.187h4.253c-.023.316-.023.631-.07.947a4.983 4.983 0 01-.958 2.29c-.841 1.11-1.94 1.8-3.33 1.986-1.145.152-2.209-.07-3.143-.77-.865-.655-1.356-1.52-1.484-2.595-.152-1.274.222-2.419.993-3.424.83-1.086 1.928-1.776 3.272-2.02 1.098-.2 2.15-.07 3.096.571.62.41 1.063.97 1.356 1.648.07.105.023.164-.117.2m3.868 6.461c-1.064-.024-2.034-.328-2.852-1.029a3.665 3.665 0 01-1.262-2.255c-.21-1.32.152-2.489.947-3.529.853-1.122 1.881-1.706 3.272-1.95 1.192-.21 2.314-.095 3.33.595.923.63 1.496 1.484 1.648 2.605.198 1.578-.257 2.863-1.344 3.962-.771.783-1.718 1.273-2.805 1.495-.315.06-.63.07-.934.106zm2.78-4.72c-.011-.153-.011-.27-.034-.387-.21-1.157-1.274-1.81-2.384-1.554-1.087.245-1.788.935-2.045 2.033-.21.912.234 1.835 1.075 2.21.643.28 1.285.244 1.905-.07.923-.48 1.425-1.228 1.484-2.233z',
      desc: 'go build runs natively with module + build caching. Warm rebuilds skip the install and emit a fresh binary in seconds.',
      terminal: `<span class="text-crush-textMuted">~/projects/go-api $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Go module detected: <span class="text-white font-semibold">github.com/user/go-api</span> (Go v1.22.2)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/2] Base runtime environment (golang:1.22-alpine) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/2] Running incremental native binary compilation...
           $ <span class="text-crush-textMuted">go build -ldflags="-w -s" -o /app/server .</span>
           ✓ Incremental go binary compiled in <span class="text-emerald-400 font-bold">0.42s</span> (Go cache hit: 100%)

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">go-api:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">24.3 MB</span> (layer cached)

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Creating isolated bridge network sandbox (<span class="text-white font-semibold">crush-bridge-0</span>)
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:8080</span>
   ↳ Assigning Windows Job Object policies (CPU rate: 2.0, RAM limit: 256MB)

<span class="text-emerald-400 font-bold">✓ Go server running natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:8080</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">12ms</span> (Total pipeline duration: 0.49s!)`,
    },
    {
      name: 'Python',
      color: '#3776AB',
      iconPath:
        'M14.25.18l.9.2.73.26.59.3.45.32.34.34.25.34.16.33.1.3.04.26.02.2-.01.13V8.5l-.05.63-.13.55-.21.46-.26.38-.3.31-.33.25-.35.19-.35.14-.33.1-.3.07-.26.04-.21.02H8.77l-.69.05-.59.14-.5.22-.41.27-.33.32-.27.35-.2.36-.15.37-.1.35-.07.32-.04.27-.02.21v3.06H3.17l-.21-.03-.28-.07-.32-.12-.35-.18-.36-.26-.36-.36-.35-.46-.32-.59-.28-.73-.21-.88-.14-1.05-.05-1.23.06-1.22.16-1.04.24-.87.32-.71.36-.57.4-.44.42-.33.42-.24.4-.16.36-.1.32-.05.24-.01h.16l.06.01h8.16v-.83H6.18l-.01-2.75-.02-.37.05-.34.11-.31.17-.28.25-.26.31-.23.38-.2.44-.18.51-.15.58-.12.64-.1.71-.06.77-.04.84-.02 1.27.05zm-6.3 1.98l-.23.33-.08.41.08.41.23.34.33.22.41.09.41-.09.33-.22.23-.34.08-.41-.08-.41-.23-.33-.33-.22-.41-.09-.41.09zm13.09 3.95l.28.06.32.12.35.18.36.27.36.35.35.47.32.59.28.73.21.88.14 1.04.05 1.23-.06 1.23-.16 1.04-.24.86-.32.71-.36.57-.4.45-.42.33-.42.24-.4.16-.36.09-.32.05-.24.02-.16-.01h-8.22v.82h5.84l.01 2.76.02.36-.05.34-.11.31-.17.29-.25.25-.31.24-.38.2-.44.17-.51.15-.58.13-.64.09-.71.07-.77.04-.84.01-1.27-.04-1.07-.14-.9-.2-.73-.25-.59-.3-.45-.33-.34-.34-.25-.34-.16-.33-.1-.3-.04-.25-.02-.2.01-.13v-5.34l.05-.64.13-.54.21-.46.26-.38.3-.32.33-.24.35-.2.35-.14.33-.1.3-.06.26-.04.21-.02.13-.01h5.84l.69-.05.59-.14.5-.21.41-.28.33-.32.27-.35.2-.36.15-.36.1-.35.07-.32.04-.28.02-.21V6.07h2.09l.14.01zm-6.47 14.25l-.23.33-.08.41.08.41.23.33.33.23.41.08.41-.08.33-.23.23-.33.08-.41-.08-.41-.23-.33-.33-.23-.41-.08-.41.08z',
      desc: 'FastAPI, Django, and Flask backends spin up in milliseconds. pip installs cache between rebuilds in isolated Job Object environments — no WSL2 required.',
      terminal: `<span class="text-crush-textMuted">~/projects/fastapi-backend $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Python stack detected: <span class="text-white font-semibold">requirements.txt</span> (Python v3.11.8)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/3] Base runtime environment (python:3.11-slim) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/3] pip packages cache mount (34 dependencies) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [3/3] Preparing application source code layers... Done (<span class="text-emerald-400 font-bold">0.05s</span>)

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">fastapi-backend:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">88.1 MB</span> (cached)

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Creating isolated bridge network sandbox (<span class="text-white font-semibold">crush-bridge-0</span>)
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:8000</span>
   ↳ Spawning worker process inside Job Object...
     $ <span class="text-crush-textMuted">uvicorn main:app --host 0.0.0.0 --port 8000 --workers 4</span>
     <span class="text-crush-textMuted">INFO:     Started server process [1284]</span>
     <span class="text-crush-textMuted">INFO:     Uvicorn running on http://0.0.0.0:8000 (Press CTRL+C to quit)</span>

<span class="text-emerald-400 font-bold">✓ FastAPI service running natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:8000</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">0.18s</span> (Total pipeline duration: 0.26s!)`,
    },
    {
      name: 'Node.js',
      color: '#5FA04E',
      iconPath:
        'M11.998,24c-0.321,0-0.641-0.084-0.922-0.247l-2.936-1.737c-0.438-0.245-0.224-0.332-0.08-0.383c0.585-0.203,0.703-0.25,1.328-0.604c0.065-0.037,0.151-0.023,0.218,0.017l2.256,1.339c0.082,0.045,0.197,0.045,0.272,0l8.795-5.076c0.082-0.047,0.134-0.141,0.134-0.238V6.921c0-0.099-0.053-0.192-0.137-0.242l-8.791-5.072c-0.081-0.047-0.189-0.047-0.271,0L3.075,6.68C2.99,6.729,2.936,6.825,2.936,6.921v10.15c0,0.097,0.054,0.189,0.139,0.235l2.409,1.392c1.307,0.654,2.108-0.116,2.108-0.89V7.787c0-0.142,0.114-0.253,0.256-0.253h1.115c0.139,0,0.255,0.112,0.255,0.253v10.021c0,1.745-0.95,2.745-2.604,2.745c-0.508,0-0.909,0-2.026-0.551L2.28,18.675c-0.57-0.329-0.922-0.945-0.922-1.604V6.921c0-0.659,0.353-1.275,0.922-1.603l8.795-5.082c0.557-0.315,1.296-0.315,1.848,0l8.794,5.082c0.57,0.329,0.924,0.944,0.924,1.603v10.15c0,0.659-0.354,1.273-0.924,1.604l-8.794,5.078C12.643,23.916,12.324,24,11.998,24zM19.099,13.993c0-1.9-1.284-2.406-3.987-2.763c-2.731-0.361-3.009-0.548-3.009-1.187c0-0.528,0.235-1.233,2.258-1.233c1.807,0,2.473,0.389,2.747,1.607c0.024,0.115,0.129,0.199,0.247,0.199h1.141c0.071,0,0.138-0.031,0.186-0.081c0.048-0.054,0.074-0.123,0.067-0.196c-0.177-2.098-1.571-3.076-4.388-3.076c-2.508,0-4.004,1.058-4.004,2.833c0,1.925,1.488,2.457,3.895,2.695c2.88,0.282,3.103,0.703,3.103,1.269c0,0.983-0.789,1.402-2.642,1.402c-2.327,0-2.839-0.584-3.011-1.742c-0.02-0.124-0.126-0.215-0.253-0.215h-1.137c-0.141,0-0.254,0.112-0.254,0.253c0,1.482,0.806,3.248,4.655,3.248C17.501,17.007,19.099,15.91,19.099,13.993z',
      desc: 'Fastify, Express, and NestJS run with native I/O performance. npm and pnpm installs cache automatically between container rebuilds.',
      terminal: `<span class="text-crush-textMuted">~/projects/node-service $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Node.js stack detected: <span class="text-white font-semibold">package.json</span> (Node v20.11.0, pnpm package manager)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/3] Base runtime environment (node:20-alpine) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/3] Resolving pnpm dependency storage ... <span class="text-emerald-400 font-semibold">Cache Hit</span> (node_modules/ up-to-date)
   ↳ [3/3] Copying application assets... Done (<span class="text-emerald-400 font-bold">0.08s</span>)

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">node-service:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">45.2 MB</span>

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Creating isolated bridge network sandbox (<span class="text-white font-semibold">crush-bridge-0</span>)
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:3000</span>
   ↳ Spawning worker process inside Job Object...
     $ <span class="text-crush-textMuted">node dist/index.js</span>
     <span class="text-crush-textMuted">[10:14:32] Fastify: Server listening on http://0.0.0.0:3000</span>

<span class="text-emerald-400 font-bold">✓ Fastify server running natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:3000</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">82ms</span> (Total pipeline duration: 0.19s!)`,
    },
    {
      name: 'Rust',
      color: '#CE422B',
      iconPath:
        'M23.8346 11.7033l-1.0073-.6236a13.7268 13.7268 0 00-.0283-.2936l.8656-.8069a.3483.3483 0 00-.1154-.578l-1.1066-.414a8.4958 8.4958 0 00-.087-.2856l.6904-.9587a.3462.3462 0 00-.2257-.5446l-1.1663-.1894a9.3574 9.3574 0 00-.1407-.2622l.49-1.0761a.3437.3437 0 00-.0274-.3361.3486.3486 0 00-.3006-.154l-1.1845.0416a6.7444 6.7444 0 00-.1873-.2268l.2723-1.153a.3472.3472 0 00-.417-.4172l-1.1532.2724a14.0183 14.0183 0 00-.2278-.1873l.0415-1.1845a.3442.3442 0 00-.49-.328l-1.076.491c-.0872-.0476-.1742-.0952-.2623-.1407l-.1903-1.1673A.3483.3483 0 0016.256.955l-.9597.6905a8.4867 8.4867 0 00-.2855-.086l-.414-1.1066a.3483.3483 0 00-.5781-.1154l-.8069.8666a9.2936 9.2936 0 00-.2936-.0284L12.2946.1683a.3462.3462 0 00-.5892 0l-.6236 1.0073a13.7383 13.7383 0 00-.2936.0284L9.9803.3374a.3462.3462 0 00-.578.1154l-.4141 1.1065c-.0962.0274-.1903.0567-.2855.086L7.744.955a.3483.3483 0 00-.5447.2258L7.009 2.348a9.3574 9.3574 0 00-.2622.1407l-1.0762-.491a.3462.3462 0 00-.49.328l.0416 1.1845a7.9826 7.9826 0 00-.2278.1873L3.8413 3.425a.3472.3472 0 00-.4171.4171l.2713 1.1531c-.0628.075-.1255.1509-.1863.2268l-1.1845-.0415a.3462.3462 0 00-.328.49l.491 1.0761a9.167 9.167 0 00-.1407.2622l-1.1662.1894a.3483.3483 0 00-.2258.5446l.6904.9587a13.303 13.303 0 00-.087.2855l-1.1065.414a.3483.3483 0 00-.1155.5781l.8656.807a9.2936 9.2936 0 00-.0283.2935l-1.0073.6236a.3442.3442 0 000 .5892l1.0073.6236c.008.0982.0182.1964.0283.2936l-.8656.8079a.3462.3462 0 00.1155.578l1.1065.4141c.0273.0962.0567.1914.087.2855l-.6904.9587a.3452.3452 0 00.2268.5447l1.1662.1893c.0456.088.0922.1751.1408.2622l-.491 1.0762a.3462.3462 0 00.328.49l1.1834-.0415c.0618.0769.1235.1528.1873.2277l-.2713 1.1541a.3462.3462 0 00.4171.4161l1.153-.2713c.075.0638.151.1255.2279.1863l-.0415 1.1845a.3442.3442 0 00.49.327l1.0761-.49c.087.0486.1741.0951.2622.1407l.1903 1.1662a.3483.3483 0 00.5447.2268l.9587-.6904a9.299 9.299 0 00.2855.087l.414 1.1066a.3452.3452 0 00.5781.1154l.8079-.8656c.0972.0111.1954.0203.2936.0294l.6236 1.0073a.3472.3472 0 00.5892 0l.6236-1.0073c.0982-.0091.1964-.0183.2936-.0294l.8069.8656a.3483.3483 0 00.578-.1154l.4141-1.1066a8.4626 8.4626 0 00.2855-.087l.9587.6904a.3452.3452 0 00.5447-.2268l.1903-1.1662c.088-.0456.1751-.0931.2622-.1407l1.0762.49a.3472.3472 0 00.49-.327l-.0415-1.1845a6.7267 6.7267 0 00.2267-.1863l1.1531.2713a.3472.3472 0 00.4171-.416l-.2713-1.1542c.0628-.0749.1255-.1508.1863-.2278l1.1845.0415a.3442.3442 0 00.328-.49l-.49-1.076c.0475-.0872.0951-.1742.1407-.2623l1.1662-.1893a.3483.3483 0 00.2258-.5447l-.6904-.9587.087-.2855 1.1066-.414a.3462.3462 0 00.1154-.5781l-.8656-.8079c.0101-.0972.0202-.1954.0283-.2936l1.0073-.6236a.3442.3442 0 000-.5892zm-6.7413 8.3551a.7138.7138 0 01.2986-1.396.714.714 0 11-.2997 1.396zm-.3422-2.3142a.649.649 0 00-.7715.5l-.3573 1.6685c-1.1035.501-2.3285.7795-3.6193.7795a8.7368 8.7368 0 01-3.6951-.814l-.3574-1.6684a.648.648 0 00-.7714-.499l-1.473.3158a8.7216 8.7216 0 01-.7613-.898h7.1676c.081 0 .1356-.0141.1356-.088v-2.536c0-.074-.0536-.0881-.1356-.0881h-2.0966v-1.6077h2.2677c.2065 0 1.1065.0587 1.394 1.2088.0901.3533.2875 1.5044.4232 1.8729.1346.413.6833 1.2381 1.2685 1.2381h3.5716a.7492.7492 0 00.1296-.0131 8.7874 8.7874 0 01-.8119.9526zM6.8369 20.024a.714.714 0 11-.2997-1.396.714.714 0 01.2997 1.396zM4.1177 8.9972a.7137.7137 0 11-1.304.5791.7137.7137 0 011.304-.579zm-.8352 1.9813l1.5347-.6824a.65.65 0 00.33-.8585l-.3158-.7147h1.2432v5.6025H3.5669a8.7753 8.7753 0 01-.2834-3.348zm6.7343-.5437V8.7836h2.9601c.153 0 1.0792.1772 1.0792.8697 0 .575-.7107.7815-1.2948.7815zm10.7574 1.4862c0 .2187-.008.4363-.0243.651h-.9c-.09 0-.1265.0586-.1265.1477v.413c0 .973-.5487 1.1846-1.0296 1.2382-.4576.0517-.9648-.1913-1.0275-.4717-.2704-1.5186-.7198-1.8436-1.4305-2.4034.8817-.5599 1.799-1.386 1.799-2.4915 0-1.1936-.819-1.9458-1.3769-2.3153-.7825-.5163-1.6491-.6195-1.883-.6195H5.4682a8.7651 8.7651 0 014.907-2.7699l1.0974 1.151a.648.648 0 00.9182.0213l1.227-1.1743a8.7753 8.7753 0 016.0044 4.2762l-.8403 1.8982a.652.652 0 00.33.8585l1.6178.7188c.0283.2875.0425.577.0425.8717zm-9.3006-9.5993a.7128.7128 0 11.984 1.0316.7137.7137 0 01-.984-1.0316zm8.3389 6.71a.7107.7107 0 01.9395-.3625.7137.7137 0 11-.9405.3635z',
      desc: 'cargo runs natively with target/ preserved across runs. Warm rebuilds are incremental — Crush adds no overhead on top.',
      terminal: `<span class="text-crush-textMuted">~/projects/rust-microservice $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Rust workspace detected: <span class="text-white font-semibold">Cargo.toml</span> (stable-x86_64-pc-windows-msvc)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/2] Base compile environment (rust:1.76-alpine) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/2] Running compilation with target/ cache mount...
           $ <span class="text-crush-textMuted">cargo build --release</span>
           <span class="text-crush-textMuted">Compiling my-rust-service v0.1.0 (/app)</span>
           ✓ Incremental compile complete in <span class="text-emerald-400 font-bold">0.58s</span> (Cargo cache hit: 98%)

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">rust-microservice:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">12.4 MB</span> (super-lightweight OCI static)

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Creating isolated bridge network sandbox (<span class="text-white font-semibold">crush-bridge-0</span>)
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:8080</span>
   ↳ Spawning optimized binary inside Job Object sandbox...
     $ <span class="text-crush-textMuted">./target/release/rust-microservice</span>

<span class="text-emerald-400 font-bold">✓ Actix-web server running natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:8080</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">4ms</span> (Total pipeline duration: 0.65s!)`,
    },
    {
      name: '.NET / ASP.NET Core',
      color: '#512BD4',
      iconPath:
        'M24 8.77h-2.468v7.565h-1.425V8.77h-2.462V7.53H24zm-6.852 7.565h-4.821V7.53h4.63v1.24h-3.205v2.494h2.953v1.234h-2.953v2.604h3.396zm-6.708 0H8.882L4.78 9.863a2.896 2.896 0 0 1-.258-.51h-.036c.032.189.048.592.048 1.21v5.772H3.157V7.53h1.659l3.965 6.32c.167.261.275.442.323.54h.024c-.04-.233-.06-.629-.06-1.185V7.529h1.372zm-8.703-.693a.868.829 0 0 1-.869.829.868.829 0 0 1-.868-.83.868.829 0 0 1 .868-.828.868.829 0 0 1 .869.829Z',
      desc: 'First-class Windows support — ASP.NET Core and Blazor run directly against the NT kernel. No Linux compatibility shim or virtual machine required.',
      terminal: `<span class="text-crush-textMuted">~/projects/BlazorApp $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ .NET solution detected: <span class="text-white font-semibold">BlazorApp.csproj</span> (.NET v8.0.3)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/3] Base runtime environment (dotnet/aspnet:8.0) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/3] Restoring NuGet dependencies cache ... <span class="text-emerald-400 font-semibold">Cache Hit</span> (NuGet layer unchanged)
   ↳ [3/3] Compiling and publishing app assemblies...
           $ <span class="text-crush-textMuted">dotnet publish -c Release -o /app</span>
           ✓ Assembly publishing complete in <span class="text-emerald-400 font-bold">0.84s</span>

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">blazorapp:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">62.7 MB</span>

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Booting natively against the NT Kernel (<span class="text-amber-400 font-semibold">no WSL2 or Hyper-V VM required</span>)
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:5000</span>
   ↳ Spawning worker process inside Job Object...
     $ <span class="text-crush-textMuted">dotnet BlazorApp.dll --urls "http://0.0.0.0:5000"</span>
     <span class="text-crush-textMuted">info: Microsoft.Hosting.Lifetime[14]</span>
     <span class="text-crush-textMuted">      Now listening on: http://0.0.0.0:5000</span>

<span class="text-emerald-400 font-bold">✓ ASP.NET Core app running natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:5000</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">0.15s</span> (Total pipeline duration: 1.05s!)`,
    },
    {
      name: 'Java / JVM',
      color: '#ED8B00',
      iconPath:
        'M11.915 0 11.7.215C9.515 2.4 7.47 6.39 6.046 10.483c-1.064 1.024-3.633 2.81-3.711 3.551-.093.87 1.746 2.611 1.55 3.235-.198.625-1.304 1.408-1.014 1.939.1.188.823.011 1.277-.491a13.389 13.389 0 0 0-.017 2.14c.076.906.27 1.668.643 2.232.372.563.956.911 1.667.911.397 0 .727-.114 1.024-.264.298-.149.571-.33.91-.5.68-.34 1.634-.666 3.53-.604 1.903.062 2.872.39 3.559.704.687.314 1.15.664 1.925.664.767 0 1.395-.336 1.807-.9.412-.563.631-1.33.72-2.24.06-.623.055-1.32 0-2.066.454.45 1.117.604 1.213.424.29-.53-.816-1.314-1.013-1.937-.198-.624 1.642-2.366 1.549-3.236-.08-.748-2.707-2.568-3.748-3.586C16.428 6.374 14.308 2.394 12.13.215zm.175 6.038a2.95 2.95 0 0 1 2.943 2.942 2.95 2.95 0 0 1-2.943 2.943A2.95 2.95 0 0 1 9.148 8.98a2.95 2.95 0 0 1 2.942-2.942zM8.685 7.983a3.515 3.515 0 0 0-.145.997c0 1.951 1.6 3.55 3.55 3.55 1.95 0 3.55-1.598 3.55-3.55 0-.329-.046-.648-.132-.951.334.095.64.208.915.336a42.699 42.699 0 0 1 2.042 5.829c.678 2.545 1.01 4.92.846 6.607-.082.844-.29 1.51-.606 1.94-.315.431-.713.651-1.315.651-.593 0-.932-.27-1.673-.61-.741-.338-1.825-.694-3.792-.758-1.974-.064-3.073.293-3.821.669-.375.188-.659.373-.911.5s-.466.2-.752.2c-.53 0-.876-.209-1.16-.64-.285-.43-.474-1.101-.545-1.948-.141-1.693.176-4.069.823-6.614a43.155 43.155 0 0 1 1.934-5.783c.348-.167.749-.31 1.192-.425zm-3.382 4.362a.216.216 0 0 1 .13.031c-.166.56-.323 1.116-.463 1.665a33.849 33.849 0 0 0-.547 2.555 3.9 3.9 0 0 0-.2-.39c-.58-1.012-.914-1.642-1.16-2.08.315-.24 1.679-1.755 2.24-1.781zm13.394.01c.562.027 1.926 1.543 2.24 1.783-.246.438-.58 1.068-1.16 2.08a4.428 4.428 0 0 0-.163.309 32.354 32.354 0 0 0-.562-2.49 40.579 40.579 0 0 0-.482-1.652.216.216 0 0 1 .127-.03z',
      desc: 'Spring Boot, Micronaut, and Quarkus with pre-warmed container images. Hibernate, Kafka, and JDBC drivers work out of the box — no Hyper-V overhead.',
      terminal: `<span class="text-crush-textMuted">~/projects/spring-api $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Java Maven package detected: <span class="text-white font-semibold">pom.xml</span> (OpenJDK v17.0.10)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/3] Base runtime environment (eclipse-temurin:17-jre) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/3] Maven local repository mount (.m2/ repository cache hit) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [3/3] Compiling and packaging JVM binary...
           $ <span class="text-crush-textMuted">./mvnw package -DskipTests</span>
           ✓ JAR package compiled successfully in <span class="text-emerald-400 font-bold">1.1s</span>

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">spring-api:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">142.6 MB</span>

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Creating isolated bridge network sandbox (<span class="text-white font-semibold">crush-bridge-0</span>)
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:8080</span>
   ↳ Booting pre-warmed JVM worker process inside Job Object sandbox...
     $ <span class="text-crush-textMuted">java -jar target/spring-api-0.0.1.jar</span>
     <span class="text-crush-textMuted">:: Spring Boot ::                (v3.2.3)</span>
     <span class="text-crush-textMuted">Tomcat initialized with port(s): 8080 (http)</span>

<span class="text-emerald-400 font-bold">✓ Spring Boot app listening natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:8080</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">0.75s</span> (Total pipeline duration: 1.94s!)`,
    },
    {
      name: 'Angular / React / Vue',
      color: '#DD0031',
      iconPath:
        'M16.712 17.711H7.288l-1.204 2.916L12 24l5.916-3.373-1.204-2.916ZM14.692 0l7.832 16.855.814-12.856L14.692 0ZM9.308 0 .662 3.999l.814 12.856L9.308 0Zm-.405 13.93h6.198L12 6.396 8.903 13.93Z',
      desc: 'Serve Angular, React, and Vue builds via a built-in Crush static server or proxy to your backend. Export to Dockerfile for Vercel, Netlify, or any CDN edge.',
      terminal: `<span class="text-crush-textMuted">~/projects/angular-dashboard $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Single Page Application stack detected: <span class="text-white font-semibold">package.json</span> (Angular v18, npm)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/3] Base runtime web server environment (nginx:alpine) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/3] Compiling production build web assets...
           $ <span class="text-crush-textMuted">npm run build --configuration=production</span>
           ✓ Web assets compiled in <span class="text-emerald-400 font-bold">1.34s</span> (dist/ folder ready)
   ↳ [3/3] Copying static bundle to Nginx html directory... Done

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">angular-dashboard:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">18.1 MB</span>

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Serving static assets via built-in Crush static routing proxy
   ↳ Binding local socket interface: <span class="text-white font-semibold">http://localhost:4200</span>

<span class="text-emerald-400 font-bold">✓ Angular application served natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:4200</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">5ms</span> (Total pipeline duration: 1.41s!)`,
    },
    {
      name: 'Bun / Deno',
      color: '#FBF0DF',
      iconPath:
        'M12 22.596c6.628 0 12-4.338 12-9.688 0-3.318-2.057-6.248-5.219-7.986-1.286-.715-2.297-1.357-3.139-1.89C14.058 2.025 13.08 1.404 12 1.404c-1.097 0-2.334.785-3.966 1.821a49.92 49.92 0 0 1-2.816 1.697C2.057 6.66 0 9.59 0 12.908c0 5.35 5.372 9.687 12 9.687v.001ZM10.599 4.715c.334-.759.503-1.58.498-2.409 0-.145.202-.187.23-.029.658 2.783-.902 4.162-2.057 4.624-.124.048-.199-.121-.103-.209a5.763 5.763 0 0 0 1.432-1.977Zm2.058-.102a5.82 5.82 0 0 0-.782-2.306v-.016c-.069-.123.086-.263.185-.172 1.962 2.111 1.307 4.067.556 5.051-.082.103-.23-.003-.189-.126a5.85 5.85 0 0 0 .23-2.431Zm1.776-.561a5.727 5.727 0 0 0-1.612-1.806v-.014c-.112-.085-.024-.274.114-.218 2.595 1.087 2.774 3.18 2.459 4.407a.116.116 0 0 1-.049.071.11.11 0 0 1-.153-.026.122.122 0 0 1-.022-.083a5.891 5.891 0 0 0-.737-2.331Zm-5.087.561c-.617.546-1.282.76-2.063 1-.117 0-.195-.078-.156-.181 1.752-.909 2.376-1.649 2.999-2.778 0 0 .155-.118.188.085 0 .304-.349 1.329-.968 1.874Zm4.945 11.237a2.957 2.957 0 0 1-.937 1.553c-.346.346-.8.565-1.286.62a2.178 2.178 0 0 1-1.327-.62 2.955 2.955 0 0 1-.925-1.553.244.244 0 0 1 .064-.198.234.234 0 0 1 .193-.069h3.965a.226.226 0 0 1 .19.07c.05.053.073.125.063.197Zm-5.458-2.176a1.862 1.862 0 0 1-2.384-.245 1.98 1.98 0 0 1-.233-2.447c.207-.319.503-.566.848-.713a1.84 1.84 0 0 1 1.092-.11c.366.075.703.261.967.531a1.98 1.98 0 0 1 .408 2.114 1.931 1.931 0 0 1-.698.869v.001Zm8.495.005a1.86 1.86 0 0 1-2.381-.253 1.964 1.964 0 0 1-.547-1.366c0-.384.11-.76.32-1.079.207-.319.503-.567.849-.713a1.844 1.844 0 0 1 1.093-.108c.367.076.704.262.968.534a1.98 1.98 0 0 1 .4 2.117 1.932 1.932 0 0 1-.702.868Z',
      desc: 'Modern runtimes with native I/O bindings. Crush runs Bun and Deno workloads without any WSL2 configuration — batteries included on Windows 10 and above.',
      terminal: `<span class="text-crush-textMuted">~/projects/bun-app $</span> <span class="text-crush-orange font-bold">crush</span>

<span class="text-sky-400 font-bold">🔍 Detecting project stack...</span>
   ↳ Bun runtime stack detected: <span class="text-white font-semibold">package.json</span> (Bun v1.1.4)

<span class="text-purple-400 font-bold">📦 Building containerized layer tree...</span>
   ↳ [1/3] Base runtime environment (oven/bun:alpine) ... <span class="text-emerald-400 font-semibold">Cache Hit</span>
   ↳ [2/3] Bun lockfile dependency cache check ... <span class="text-emerald-400 font-semibold">Cache Hit</span> (bun.lockb unchanged)
   ↳ [3/3] Packing application TypeScript source code... Done (<span class="text-emerald-400 font-bold">0.01s</span>)

<span class="text-emerald-400 font-bold">✨ Successfully crushed to image:</span> <span class="text-white font-semibold">bun-app:latest</span>
   ⚡ Total compressed image size: <span class="text-emerald-400 font-bold">28.4 MB</span>

<span class="text-sky-400 font-bold">🚀 Starting native container environment...</span>
   ↳ Spawning worker process inside Job Object using Bun's native NT I/O bindings...
     $ <span class="text-crush-textMuted">bun run src/index.ts</span>
     <span class="text-crush-textMuted">[Bun] Server listening on http://0.0.0.0:3000</span>

<span class="text-emerald-400 font-bold">✓ Bun application running natively on</span> <span class="text-sky-400 underline font-semibold">http://localhost:3000</span>
   ⚡ Cold boot elapsed: <span class="text-emerald-400 font-bold">8ms</span> (Total pipeline duration: 0.12s!)`,
    },
  ];

  faqs = [
    {
      q: 'What is the difference between Crush and Docker Desktop?',
      a: 'Docker Desktop runs a full Linux virtual machine (via WSL2 or Hyper-V) to host its container daemon — every container startup requires that VM to be running, and all system calls cross the VM boundary. Crush uses Windows Job Objects and NT kernel APIs to run containers natively on Windows with no daemon and no VM. Cold-start time drops from 8–30 seconds to under one second.',
    },
    {
      q: 'Can I run existing Dockerfiles on Windows without Hyper-V?',
      a: 'Yes. crush build . parses and executes standard Dockerfiles natively without Hyper-V, WSL2, or Docker Desktop. Multi-stage builds, ARG, ENV, and COPY instructions are fully supported. The resulting image is OCI-compatible and can be pushed to Docker Hub, GitHub Container Registry, or any standard OCI registry.',
    },
    {
      q: 'How do Windows Job Objects compare to Linux cgroups?',
      a: 'Linux cgroups are the mechanism Docker uses to limit CPU, memory, and I/O for containers on Linux. Windows Job Objects are the NT kernel equivalent — a first-class OS primitive for grouping and constraining processes. Crush builds its isolation layer directly on Job Objects, giving zero VM overhead and tighter OS integration than any hypervisor-based solution.',
    },
    {
      q: 'Can I deploy Crush containers to Hetzner, AWS, or other VPS providers?',
      a: 'Yes. Crush exports standard OCI-compatible images, and crush export generates a standard Dockerfile and docker-compose.yml from any Crush project. These artifacts deploy directly to Hetzner VPS, AWS EC2, DigitalOcean Droplets, Azure Container Instances, GCP Cloud Run, or bare-metal Linux — no vendor lock-in.',
    },
    {
      q: 'Is Crush a drop-in replacement for Docker Desktop?',
      a: 'For local Windows development, yes. crush run, crush build, crush push, and crush pull mirror the Docker CLI surface. Existing docker-compose.yml files run with crush compose up. The key difference is no background daemon — nothing consuming 500 MB of RAM while you are not actively using containers.',
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta,
    @Inject(DOCUMENT) private document: Document
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Crush — Lightweight Docker Desktop Alternative for Windows (No WSL2)');
    this.meta.updateTag({
      name: 'description',
      content:
        'A fast, lightweight Docker Desktop alternative for native Windows development. Run docker-compose dependencies like Postgres & Redis with sub-second startup and zero VM memory overhead.',
    });
    const script = this.document.createElement('script');
    script.type = 'application/ld+json';
    script.text = JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'FAQPage',
      mainEntity: this.faqs.map((f) => ({
        '@type': 'Question',
        name: f.q,
        acceptedAnswer: { '@type': 'Answer', text: f.a },
      })),
    });
    this.document.head.appendChild(script);
  }
}
