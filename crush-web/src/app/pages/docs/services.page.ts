import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';
import { BrandIconComponent } from '../../components/brand-icon/brand-icon.component';

@Component({
  selector: 'page-services',
  standalone: true,
  imports: [DocsSidebarComponent, BrandIconComponent],
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
              Run premium native services—Postgres, MySQL, MariaDB, Redis, MongoDB, SQLite, MinIO—directly on host processes
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
            <div class="grid gap-4 sm:grid-cols-2">
              @for (s of services; track s.name) {
                <div class="group rounded-xl border border-crush-border/40 bg-card p-5 hover:border-crush-orange/30 hover:bg-crush-surface/5 transition-all duration-300">
                  <div class="flex items-center gap-3 mb-3">
                    <div class="h-11 w-11 shrink-0 flex items-center justify-center rounded-lg bg-crush-surface/60 border border-crush-border/40 group-hover:scale-110 transition-transform duration-300">
                      <app-brand-icon [name]="s.icon" [size]="22" />
                    </div>
                    <div class="min-w-0 flex-1">
                      <h4 class="font-bold text-white leading-tight">{{ s.name }}</h4>
                      <div class="flex items-center gap-2 mt-1">
                        <code class="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono">{{ s.port === 'file' ? 'file-based' : ':' + s.port }}</code>
                        <code class="text-[10px] text-crush-textMuted font-mono">{{ s.env }}</code>
                      </div>
                    </div>
                  </div>
                  <p class="text-xs text-crush-textMuted leading-relaxed">{{ s.desc }}</p>
                </div>
              }
            </div>
            <p class="text-xs text-crush-textMuted mt-5 leading-relaxed">
              Connection strings are injected automatically as environment variables — list a service
              in your <code>Crushfile</code> (or compose file) and it starts on <code>crush run</code>.
            </p>
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
export default class ServicesPageComponent implements OnInit {
  services = [
    {
      name: 'PostgreSQL',
      icon: 'postgres',
      port: '5432',
      env: 'DATABASE_URL',
      desc: 'Full-featured relational database. Crush provisions an isolated data directory per project and injects the connection string automatically.',
    },
    {
      name: 'Redis',
      icon: 'redis',
      port: '6379',
      env: 'REDIS_URL',
      desc: 'In-memory cache and queue backend. Boots in milliseconds for sessions, rate-limiting, and background job queues.',
    },
    {
      name: 'MongoDB',
      icon: 'mongodb',
      port: '27017',
      env: 'MONGODB_URI',
      desc: 'Document store for schema-flexible workloads. Runs natively with a per-project data path and no VM overhead.',
    },
    {
      name: 'MinIO',
      icon: 'minio',
      port: '9000',
      env: 'S3_ENDPOINT',
      desc: 'S3-compatible object storage for files and assets. Drop-in replacement for AWS S3 during local development.',
    },
    {
      name: 'MySQL',
      icon: 'mysql',
      port: '3306',
      env: 'DATABASE_URL',
      desc: 'The world’s most popular open-source relational database. Crush boots it natively with a per-project data directory and synthesizes the connection string.',
    },
    {
      name: 'MariaDB',
      icon: 'mariadb',
      port: '3306',
      env: 'DATABASE_URL',
      desc: 'Community-driven MySQL fork. Detected from your Compose or Spring config and run as a host process — no container, no VM.',
    },
    {
      name: 'SQLite',
      icon: 'sqlite',
      port: 'file',
      env: 'DATABASE_URL',
      desc: 'Zero-config embedded database — no server to run. Crush points your app at a per-project .db file, perfect for local dev and tests.',
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit() {
    this.title.setTitle('Native Services - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content:
        'Run high-fidelity native services (Postgres, MySQL, MariaDB, Redis, MongoDB, SQLite, MinIO S3) with zero VM memory overhead.',
    });
  }
}
