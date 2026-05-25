import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-docs',
  standalone: true,
  imports: [RouterLink, DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <div class="flex-1 min-w-0">
          <div class="border-b border-crush-border/30 pb-8 mb-12 select-none">
            <span
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-crush-orange/10 text-crush-orange border border-crush-orange/20 mb-4 uppercase tracking-wider"
            >
              Documentation Portal
            </span>
            <h1 class="text-4xl font-extrabold text-white tracking-tight sm:text-5xl mb-4">
              Documentation
            </h1>
            <p class="text-lg text-crush-textMuted max-w-2xl">
              Everything you need to configure, build, run, and scale containers natively on
              Windows.
            </p>
          </div>

          <div class="grid gap-6 sm:grid-cols-2">
            @for (card of cards; track card.path) {
              <a
                [routerLink]="card.path"
                class="group relative block overflow-hidden rounded-xl border border-crush-border/40 bg-gradient-to-b from-crush-surface/30 to-crush-surface/10 p-6 hover:border-crush-orange/30 hover:bg-crush-surface/20 transition-all duration-300 no-underline cursor-pointer flex flex-col justify-between"
              >
                <!-- Subtle Glow effect -->
                <div
                  class="absolute -right-8 -top-8 w-24 h-24 rounded-full bg-crush-orange/3 blur-xl group-hover:bg-crush-orange/8 transition-all duration-500 pointer-events-none"
                ></div>

                <div>
                  <div class="flex items-center gap-4 mb-4 select-none">
                    <div
                      class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-crush-surface border border-crush-border/60 text-crush-orangeLight group-hover:scale-110 group-hover:border-crush-orange/40 group-hover:text-crush-orange transition-all duration-300"
                    >
                      <svg
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        class="h-5 w-5"
                      >
                        <path [attr.d]="card.icon" />
                      </svg>
                    </div>
                    <h3
                      class="text-lg font-bold text-white group-hover:text-crush-orangeLight transition-colors"
                    >
                      {{ card.title }}
                    </h3>
                  </div>
                  <p class="text-sm text-crush-textMuted leading-relaxed mb-6">
                    {{ card.description }}
                  </p>
                </div>

                <div
                  class="flex items-center gap-1.5 text-xs font-bold text-crush-orange group-hover:text-crush-orangeLight transition-colors"
                >
                  <span>Explore guide</span>
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    class="h-3.5 w-3.5 transition-transform group-hover:translate-x-1"
                  >
                    <line x1="5" y1="12" x2="19" y2="12" />
                    <polyline points="12 5 19 12 12 19" />
                  </svg>
                </div>
              </a>
            }
          </div>
        </div>
      </div>
    </div>
  `,
})
export default class DocsPage implements OnInit {
  cards = [
    {
      path: '/docs/getting-started',
      title: 'Getting Started',
      description:
        'Your first container in 5 minutes. Detect, initialize, build, and run packages.',
      icon: 'M13 10V3L4 14h7v7l9-11h-7z',
    },
    {
      path: '/docs/installation',
      title: 'Installation',
      description:
        'Find every supported install method across Windows, macOS, Linux, and specific server platforms.',
      icon: 'M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4',
    },
    {
      path: '/docs/cli-reference',
      title: 'CLI Reference',
      description:
        'Exhaustive commands directory listing every command, operational flag, and usage example.',
      icon: 'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z',
    },
    {
      path: '/docs/crushfile',
      title: 'Crushfile Schema',
      description:
        'Complete TOML configuration schema for full container build and runtime isolation parameters.',
      icon: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
    },
    {
      path: '/docs/docker-migration',
      title: 'Docker Migration',
      description:
        'Clean migration roadmap to replace multi-stage Dockerfiles and transition to daemonless jobs.',
      icon: 'M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4',
    },
    {
      path: '/docs/windows',
      title: 'Windows Native',
      description:
        'Understand the internals: Job Objects, direct system calls, and microVM builder environments.',
      icon: 'M0 0h11.377v11.373H0zm12.623 0H24v11.373H12.623zM0 12.627h11.377V24H0zm12.623 0H24V24H12.623z',
    },
    {
      path: '/docs/security',
      title: 'Security',
      description:
        'Integrity features including COSING verification, AES-256 secret vaulting, and automatic vulnerability scans.',
      icon: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z',
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Documentation — Crush');
    this.meta.updateTag({
      name: 'description',
      content: 'Crush documentation — install, configure, build, and deploy containers on Windows.',
    });
  }
}
