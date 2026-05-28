import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmBadgeDirective } from '../ui/badge';

@Component({
  selector: 'page-changelog',
  standalone: true,
  imports: [RouterLink, HlmBadgeDirective],
  template: `
    <div class="mx-auto max-w-3xl px-4 py-12 sm:px-6 lg:px-8">
      <h1 class="text-3xl font-bold text-white mb-2">Changelog</h1>
      <p class="text-lg text-crush-textMuted mb-12">Version history.</p>

      @for (release of releases; track release.version) {
        <div class="border-l-2 border-crush-border/50 pl-6 pb-12 relative">
          <div
            class="absolute left-[-9px] top-0 h-4 w-4 rounded-full border-2 border-crush-orange bg-crush-black"
          ></div>
          <div class="flex items-center gap-3 mb-2">
            <h2 class="text-xl font-bold text-white font-mono">{{ release.version }}</h2>
            <span
              hlmBadge
              variant="outline"
              class="border-crush-orange/30 bg-crush-orange/10 text-crush-orange hover:bg-crush-orange/20"
              >{{ release.date }}</span
            >
          </div>
          <ul class="space-y-2">
            @for (item of release.items; track item) {
              <li class="text-sm text-crush-textMuted flex items-start gap-2">
                <span class="text-crush-orange mt-0.5 shrink-0">-</span>
                <span>{{ item }}</span>
              </li>
            }
          </ul>
        </div>
      }
    </div>
  `,
})
export default class ChangelogPage implements OnInit {
  releases = [
    {
      version: 'v0.8.0-alpha',
      date: 'In Development',
      items: [
        'Scaffolded Tauri 2 + SvelteKit desktop app GUI shell',
        'Implemented Tauri command bindings for process control, image management, and native settings',
        'Wired real backends (crush-api Unix socket, crush-proto OCI gate, and crush-tui sparklines)',
        'Created high-fidelity brand icons generator to avoid windres resources compilation errors'
      ],
    },
    {
      version: 'v0.7.74',
      date: '2026-05-28',
      items: [
        'First stable native Windows dev runner release series',
        'Automatic local stack detection (Next, Nuxt, Vite, AnalogJS, Spring Boot, FastAPI, Django, Go, Rust, Rails, Laravel, Phoenix, .NET)',
        'Auto-starts native dependencies (PostgreSQL, MySQL) and synchronizes database credentials on startup',
        'Integrates Microsoft Garnet as a zero-VM, ultra-low memory Redis-compatible host cache on Windows',
        'Compiles pgvector natively against your host Postgres via local MSVC on first use',
        'Enforces complete process-tree termination on Ctrl+C via kernel-level Windows Job Objects',
        'Supports Turborepo, Nx, pnpm-workspaces, and multi-service monorepos',
        'Provides `--memory`, `--cpus`, and `--priority` resources capping via Job Object boundaries',
        'Adds crush eject to write standardized, production-ready Dockerfile and docker-compose.yml files',
        'Adds crush update to self-update the CLI binary directly from GitHub releases',
        'Adds AI-powered crush debug command to analyze local stack traces and suggest quick fixes'
      ],
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Changelog — Crush');
    this.meta.updateTag({
      name: 'description',
      content: 'Crush version history and release notes.',
    });
  }
}
