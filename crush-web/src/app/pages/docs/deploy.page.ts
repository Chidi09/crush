import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-deploy',
  standalone: true,
  imports: [RouterLink, DocsSidebarComponent],
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
              Cloud Deployments
            </h1>
            <p class="text-base text-crush-textMuted">
              Eject Compose stacks to Dockerfiles or deploy directly across 11 popular cloud
              providers with zero-configuration.
            </p>
          </div>

          <!-- Section 1: Overview -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">The Deploy Workflow</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Crush is not only a local development runner; it is a fully integrated
              build-and-deployment platform. With a single command, you can deploy your local
              service compose stacks to cloud servers, VPS machines, or serverless infrastructure
              without learning proprietary setup configurations.
            </p>
          </section>

          <!-- Section 2: Providers -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">11 Supported Providers</h2>
            <p class="text-sm text-crush-textMuted mb-6 leading-relaxed">
              Crush wraps popular cloud provider CLIs to securely deploy your application. Tokens
              and secrets are retrieved from each provider's official CLI authentication state so
              that your passwords remain securely encrypted.
            </p>

            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Railway</h4>
                <p class="text-xs text-crush-textMuted">
                  Deploys instantly with continuous integration and native env-variable management.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Fly.io</h4>
                <p class="text-xs text-crush-textMuted">
                  Edge serverless deployment close to your users using Firecracker microVMs.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">DigitalOcean</h4>
                <p class="text-xs text-crush-textMuted">
                  Deploy to App Platform or spin up standard droplets running Docker.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Google Cloud Run</h4>
                <p class="text-xs text-crush-textMuted">
                  Fully managed container execution serverless scaling to zero automatically.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">AWS App Runner</h4>
                <p class="text-xs text-crush-textMuted">
                  Sleek, high-performance container runner without ECS/EKS configuration burden.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Render</h4>
                <p class="text-xs text-crush-textMuted">
                  Sleek PaaS alternative for web services, static sites, background workers.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Hetzner VPS</h4>
                <p class="text-xs text-crush-textMuted">
                  High CPU, low cost deployments running directly via registry-free SSH pushes.
                </p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/30">
                <h4 class="font-bold text-white mb-1.5">Vercel & Netlify</h4>
                <p class="text-xs text-crush-textMuted">
                  Deploy static frontends or serverless API backends with instant URL generation.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 3: Ejecting -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">Crush Eject (Dockerfile Generation)</h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Tired of vendor lock-in? Or need custom Docker files for environments? Run
              <code>crush eject</code> to instantly generate standard, production-ready
              <code>Dockerfile</code> and <code>docker-compose.yml</code> structures tailored to
              your technology stack.
            </p>
            <div class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden">
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30"
              >
                <span class="text-[10px] text-crush-textMuted font-mono">Terminal</span>
                <span class="text-[9px] text-crush-textMuted uppercase font-semibold">eject</span>
              </div>
              <div class="p-4 font-mono text-sm overflow-x-auto text-crush-text">
                <code>crush eject --force</code>
              </div>
            </div>
          </section>
        </article>
      </div>
    </div>
  `,
})
export class DeployPageComponent implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit() {
    this.title.setTitle('Cloud Deployments - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content: 'Eject Compose stacks to Dockerfiles or deploy across 11 popular cloud providers.',
    });
  }
}
