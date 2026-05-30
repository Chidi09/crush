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
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange">Feature</span>
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              GUI Desktop Dashboard
            </h1>
            <p class="text-base text-crush-textMuted">
              Manage your local projects, cloud deployments, branch previews, and process statistics in a beautiful dashboard.
            </p>
          </div>

          <!-- Section 1: Dashboard -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Desktop Dashboard</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              The Crush GUI is a high-performance, offline-first application engineered using Svelte 5 and Tauri. It enables rapid, visual control over all your active development services, static frontends, and backend run configurations without looking up CLI shell flags.
            </p>
          </section>

          <!-- Section 2: Features -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">Key Capabilities</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Service Control Panel</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">Power on or off Postgres, Redis, Mongo, or MinIO databases dynamically. Clear cached tables or files in one click.</p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Live Log Streaming</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">View structured real-time stdout/stderr stdout streams, filtered by service modules, formatted in a responsive terminal.</p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">AI Diagnose</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">Integrates directly with Claude to parse error frames, trace call stacks, and suggest immediate line-by-line code patches.</p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Resource Statistics</h4>
                <p class="text-xs text-crush-textMuted font-light leading-relaxed">Real-time system graphs tracking CPU percentage and memory bytes consumed by host processes, helping you spot memory leaks.</p>
              </div>
            </div>
          </section>
        </article>
      </div>
    </div>
  `,
})
export class GuiPageComponent implements OnInit {
  constructor(private title: Title, private meta: Meta) {}

  ngOnInit() {
    this.title.setTitle('GUI Desktop Dashboard - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content: 'Manage local projects, cloud deployments, and process statistics in a beautiful desktop dashboard.',
    });
  }
}
