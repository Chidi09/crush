import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-services',
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
              Native Services
            </h1>
            <p class="text-base text-crush-textMuted">
              Run premium native services—Postgres, Redis, MongoDB, MinIO—directly on host processes
              without VM overhead.
            </p>
          </div>

          <!-- Section 1: Philosophy -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">No VMs, No Memory Hogs</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              Unlike Docker Desktop, which runs a heavy Linux VM (consuming 2-4 GB of RAM even
              idle), Crush spawns local database and queue processes directly on your host OS using
              sandboxed Job Objects. They boot in milliseconds and consume less than 30MB of RAM.
            </p>
          </section>

          <!-- Section 2: Core Native Services -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">Supported Databases & Storage</h2>
            <div class="space-y-4">
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <div class="flex items-center justify-between mb-2">
                  <h4 class="font-bold text-white">PostgreSQL & pgvector</h4>
                  <span
                    class="text-xs px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-semibold font-mono"
                    >Port: 5432</span
                  >
                </div>
                <p class="text-xs text-crush-textMuted">
                  Natively loaded standard Postgres binaries complete with vector embedding storage
                  capabilities for AI models.
                </p>
              </div>

              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <div class="flex items-center justify-between mb-2">
                  <h4 class="font-bold text-white">Redis Cache & Queue</h4>
                  <span
                    class="text-xs px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-semibold font-mono"
                    >Port: 6379</span
                  >
                </div>
                <p class="text-xs text-crush-textMuted">
                  High-performance native Redis key-value cache store with disk-backed logging
                  persistence.
                </p>
              </div>

              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <div class="flex items-center justify-between mb-2">
                  <h4 class="font-bold text-white">MongoDB</h4>
                  <span
                    class="text-xs px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-semibold font-mono"
                    >Port: 27017</span
                  >
                </div>
                <p class="text-xs text-crush-textMuted">
                  NoSQL JSON document store for applications needing highly dynamic schema designs.
                </p>
              </div>

              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <div class="flex items-center justify-between mb-2">
                  <h4 class="font-bold text-white">MinIO (S3 Local Storage)</h4>
                  <span
                    class="text-xs px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-semibold font-mono"
                    >Port: 9000</span
                  >
                </div>
                <p class="text-xs text-crush-textMuted">
                  Complete local Amazon S3 bucket alternative. Store files, images, and static
                  assets locally with AWS sdk compatibility.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 3: Detection -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">External Service Auto-Detection</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              Crush is smart. During project startup, it auto-detects environment keys for
              third-party platforms like <strong>Supabase</strong>, <strong>Upstash</strong>,
              <strong>Firebase</strong>, <strong>Clerk</strong>, and <strong>Auth0</strong>. If
              present, it bypasses spinning up a duplicate local instance and seamlessly connects
              your runtime engine.
            </p>
          </section>
        </article>
      </div>
    </div>
  `,
})
export class ServicesPageComponent implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit() {
    this.title.setTitle('Native Services - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content:
        'Run high-fidelity native services (Postgres, Redis, MongoDB, MinIO S3) with zero VM memory overhead.',
    });
  }
}
