import { Component } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';

interface DocLink {
  path: string;
  label: string;
  icon: string; // SVG path or shorthand
}

interface DocGroup {
  title: string;
  links: DocLink[];
}

const GROUPS: DocGroup[] = [
  {
    title: 'Getting Started',
    links: [
      {
        path: '/docs',
        label: 'Overview',
        icon: 'M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6',
      },
      {
        path: '/docs/getting-started',
        label: 'Quick Start',
        icon: 'M13 10V3L4 14h7v7l9-11h-7z',
      },
      {
        path: '/docs/installation',
        label: 'Installation',
        icon: 'M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4',
      },
    ],
  },
  {
    title: 'Advanced Features',
    links: [
      {
        path: '/docs/deploy',
        label: 'Cloud Deployments',
        icon: 'M13 10V3L4 14h7v7l9-11h-7z',
      },
      {
        path: '/docs/services',
        label: 'Native Services',
        icon: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z M12 6v6h6',
      },
      {
        path: '/docs/gui',
        label: 'GUI Dashboard',
        icon: 'M4 5a1 1 0 011-1h14a1 1 0 011 1v12a1 1 0 01-1 1H5a1 1 0 01-1-1V5z M4 13h16',
      },
      {
        path: '/docs/branch-previews',
        label: 'Branch Previews',
        icon: 'M18 10a3 3 0 11-3-3 3 3 0 013 3z M6 10a3 3 0 11-3-3 3 3 0 013 3z M12 10a3 3 0 11-3-3 3 3 0 013 3z',
      },
    ],
  },
  {
    title: 'Reference',
    links: [
      {
        path: '/docs/cli-reference',
        label: 'CLI Reference',
        icon: 'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z',
      },
      {
        path: '/docs/crushfile',
        label: 'Crushfile Schema',
        icon: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
      },
    ],
  },
  {
    title: 'Guides',
    links: [
      {
        path: '/docs/docker-migration',
        label: 'Docker Migration',
        icon: 'M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4',
      },
      {
        path: '/docs/windows',
        label: 'Windows Guide',
        icon: 'M0 0h11.377v11.373H0zm12.623 0H24v11.373H12.623zM0 12.627h11.377V24H0zm12.623 0H24V24H12.623z',
      },
      {
        path: '/docs/security',
        label: 'Security & Secrets',
        icon: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z',
      },
    ],
  },
];

@Component({
  selector: 'app-docs-sidebar',
  standalone: true,
  imports: [RouterLink, RouterLinkActive],
  template: `
    <aside class="w-64 shrink-0 hidden md:block select-none">
      <nav class="sticky top-24 space-y-8 pr-4">
        @for (group of groups; track group.title) {
          <div>
            <h4 class="text-xs font-bold text-crush-orange/80 uppercase tracking-wider px-3 mb-3">
              {{ group.title }}
            </h4>
            <div class="space-y-1">
              @for (link of group.links; track link.path) {
                <a
                  [routerLink]="link.path"
                  routerLinkActive="bg-crush-orange/10 text-white font-semibold border-crush-orange shadow-[inset_3px_0_0_0_#e05540]"
                  [routerLinkActiveOptions]="{ exact: link.path === '/docs' }"
                  class="flex items-center gap-3 px-3 py-2 text-sm text-crush-textMuted hover:text-white border-l border-crush-border/30 hover:border-crush-orange/30 hover:bg-crush-surface/30 transition-all rounded-r-lg"
                >
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="h-4 w-4 shrink-0 opacity-70 group-hover:opacity-100"
                  >
                    <path [attr.d]="link.icon" />
                  </svg>
                  <span>{{ link.label }}</span>
                </a>
              }
            </div>
          </div>
        }
      </nav>
    </aside>
  `,
})
export class DocsSidebarComponent {
  groups = GROUPS;
}
