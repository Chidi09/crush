import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-gui',
  standalone: true,
  imports: [DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange"
              >Feature</span
            >
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              GUI Desktop Dashboard
            </h1>
            <p class="text-base text-crush-textMuted">
              Manage your local projects, cloud deployments, branch previews, and process statistics
              in a beautiful dashboard.
            </p>
          </div>

          <!-- Section 1: Dashboard -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Desktop Dashboard</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              The Crush GUI is a high-performance, offline-first application engineered using Svelte
              5 and Tauri. It enables rapid, visual control over all your active development
              services, static frontends, and backend run configurations without looking up CLI
              shell flags.
            </p>
          </section>

          <!-- Section 2: Features -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">Key Capabilities</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="group p-5 rounded-xl border border-crush-border/40 bg-card hover:border-crush-orange/30 transition-all duration-300">
                <div class="h-10 w-10 mb-3 flex items-center justify-center rounded-lg bg-crush-orange/10 text-crush-orange border border-crush-orange/20 group-hover:scale-110 transition-transform">
                  <svg viewBox="0 0 24 24" class="h-5 w-5 fill-none stroke-current stroke-2"><rect x="3" y="3" width="18" height="7" rx="1"/><rect x="3" y="14" width="18" height="7" rx="1"/><circle cx="7" cy="6.5" r=".5" fill="currentColor"/><circle cx="7" cy="17.5" r=".5" fill="currentColor"/></svg>
                </div>
                <h4 class="font-bold text-white mb-1.5">Service Control Panel</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">
                  Power on or off Postgres, Redis, Mongo, or MinIO databases dynamically. Clear
                  cached tables or files in one click.
                </p>
              </div>
              <div class="group p-5 rounded-xl border border-crush-border/40 bg-card hover:border-crush-orange/30 transition-all duration-300">
                <div class="h-10 w-10 mb-3 flex items-center justify-center rounded-lg bg-crush-orange/10 text-crush-orange border border-crush-orange/20 group-hover:scale-110 transition-transform">
                  <svg viewBox="0 0 24 24" class="h-5 w-5 fill-none stroke-current stroke-2"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
                </div>
                <h4 class="font-bold text-white mb-1.5">Live Log Streaming</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">
                  View structured real-time stdout/stderr streams, filtered by service modules,
                  formatted in a responsive terminal.
                </p>
              </div>
              <div class="group p-5 rounded-xl border border-crush-border/40 bg-card hover:border-crush-orange/30 transition-all duration-300">
                <div class="h-10 w-10 mb-3 flex items-center justify-center rounded-lg bg-crush-orange/10 text-crush-orange border border-crush-orange/20 group-hover:scale-110 transition-transform">
                  <svg viewBox="0 0 24 24" class="h-5 w-5 fill-none stroke-current stroke-2"><path d="M12 2a4 4 0 0 1 4 4c1.1 0 2 .9 2 2a3 3 0 0 1-.5 1.7A3 3 0 0 1 18 12a3 3 0 0 1-1 2.2V16a4 4 0 0 1-8 0v-1.8A3 3 0 0 1 8 12a3 3 0 0 1 .5-2.3A3 3 0 0 1 8 8c0-1.1.9-2 2-2a4 4 0 0 1 2-4z"/></svg>
                </div>
                <h4 class="font-bold text-white mb-1.5">AI Diagnose</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">
                  Reads error frames, traces call stacks, and suggests immediate line-by-line code
                  patches using your configured AI provider.
                </p>
              </div>
              <div class="group p-5 rounded-xl border border-crush-border/40 bg-card hover:border-crush-orange/30 transition-all duration-300">
                <div class="h-10 w-10 mb-3 flex items-center justify-center rounded-lg bg-crush-orange/10 text-crush-orange border border-crush-orange/20 group-hover:scale-110 transition-transform">
                  <svg viewBox="0 0 24 24" class="h-5 w-5 fill-none stroke-current stroke-2"><polyline points="3 12 7 12 10 4 14 20 17 12 21 12"/></svg>
                </div>
                <h4 class="font-bold text-white mb-1.5">Resource Statistics</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">
                  Real-time graphs tracking CPU percentage and memory consumed by host processes,
                  helping you spot leaks.
                </p>
              </div>
            </div>
          </section>
        </article>
      </div>
    </div>
  `,
})
export default class GuiPageComponent implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit() {
    this.title.setTitle('GUI Desktop Dashboard - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content:
        'Manage local projects, cloud deployments, and process statistics in a beautiful desktop dashboard.',
    });
  }
}
