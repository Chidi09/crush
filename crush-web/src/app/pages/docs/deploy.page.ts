import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

interface Provider {
  name: string;
  tag: string; // --target value
  blurb: string;
  auth: string; // one-time auth command
  deploy: string; // deploy command
}

const PROVIDERS: Provider[] = [
  { name: 'Railway', tag: 'railway', blurb: 'Instant deploys with managed env vars and add-ons.', auth: 'railway login', deploy: 'crush deploy --target railway' },
  { name: 'Fly.io', tag: 'fly', blurb: 'Edge deploys on Firecracker microVMs, close to users.', auth: 'flyctl auth login', deploy: 'crush deploy --target fly' },
  { name: 'Google Cloud Run', tag: 'gcp', blurb: 'Fully-managed, scale-to-zero containers on GCP.', auth: 'gcloud auth login', deploy: 'crush deploy --target gcp' },
  { name: 'AWS App Runner', tag: 'aws', blurb: 'Pushes to ECR, runs a managed App Runner service.', auth: 'aws configure', deploy: 'crush deploy --target aws' },
  { name: 'Azure Container Apps', tag: 'azure', blurb: 'Pushes to ACR and deploys a container app.', auth: 'az login', deploy: 'crush deploy --target azure' },
  { name: 'DigitalOcean', tag: 'digitalocean', blurb: 'App Platform deploys from a generated app spec.', auth: 'doctl auth init', deploy: 'crush deploy --target digitalocean' },
  { name: 'Render', tag: 'render', blurb: 'Blueprint-based deploys via the Render API.', auth: 'export RENDER_API_KEY=…', deploy: 'crush deploy --target render' },
  { name: 'Vercel', tag: 'vercel', blurb: 'Frontend and serverless targets.', auth: 'vercel login', deploy: 'crush deploy --target vercel' },
  { name: 'Netlify', tag: 'netlify', blurb: 'Static and edge-function targets.', auth: 'netlify login', deploy: 'crush deploy --target netlify' },
  { name: 'Hetzner VPS', tag: 'hetzner', blurb: 'Registry-free: ships the image over SSH to a VPS.', auth: 'export HCLOUD_TOKEN=…', deploy: 'crush deploy --target hetzner' },
  { name: 'Any host (SSH)', tag: 'ssh', blurb: 'Copy + run the image on any Linux box over SSH.', auth: 'ssh-copy-id user@host', deploy: 'crush deploy --target ssh --host user@host' },
];

@Component({
  selector: 'page-deploy',
  standalone: true,
  imports: [RouterLink, DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange">Feature</span>
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">Cloud Deployments</h1>
            <p class="text-base text-crush-textMuted">
              Build a real OCI image from your project and ship it to a VPS, a registry, or eleven
              managed cloud providers — with one command and zero proprietary config.
            </p>
          </div>

          <!-- TL;DR -->
          <div class="rounded-xl border border-crush-orange/20 bg-crush-orange/5 p-5 mb-12">
            <p class="text-sm text-crush-text leading-relaxed">
              <strong>TL;DR</strong> — From any project folder, run
              <code class="text-crush-orange">crush deploy</code>. Crush detects your stack, builds an
              optimized image, and deploys it to a target it suggests. Override with
              <code class="text-crush-orange">--target &lt;provider&gt;</code>. Want to leave anytime?
              <code class="text-crush-orange">crush eject</code> writes a standard Dockerfile.
            </p>
          </div>

          <!-- The workflow -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">How deployment works</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-5">
              Locally, Crush runs your app natively. To deploy, it crosses into the Docker world the
              rest of the cloud understands: it builds a standard, content-addressed OCI image, then
              hands that image to your chosen provider. Nothing is locked to Crush — the artifact is
              a normal container image.
            </p>
            <div class="grid gap-3 sm:grid-cols-3">
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/20">
                <div class="text-crush-orange text-xs font-bold mb-1">1 · Detect &amp; build</div>
                <p class="text-xs text-crush-textMuted leading-relaxed">Stack detection picks the base image, build steps, port, and start command, then builds a layered OCI image.</p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/20">
                <div class="text-crush-orange text-xs font-bold mb-1">2 · Ship</div>
                <p class="text-xs text-crush-textMuted leading-relaxed">The image is pushed to the provider's registry, or streamed straight to a VPS over SSH (registry-free).</p>
              </div>
              <div class="p-4 rounded-xl border border-crush-border/30 bg-crush-surface/20">
                <div class="text-crush-orange text-xs font-bold mb-1">3 · Run</div>
                <p class="text-xs text-crush-textMuted leading-relaxed">The provider runs the container with your ports and environment, and returns the live URL.</p>
              </div>
            </div>
          </section>

          <!-- Prerequisites -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Prerequisites</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Crush wraps each provider's official CLI and reuses its authenticated session — your
              credentials never pass through Crush. Install and log in to the CLI for the target you
              want once; the <strong>Settings</strong> tab in the GUI shows which provider CLIs Crush
              can find on your PATH.
            </p>
            <div class="rounded-xl border border-crush-border/40 bg-crush-black/60 p-4 font-mono text-sm text-crush-text overflow-x-auto">
              <div class="text-crush-textMuted"># example: authenticate Fly.io once, then deploy</div>
              <div>flyctl auth login</div>
              <div>crush deploy --target fly</div>
            </div>
          </section>

          <!-- Providers -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-2">Supported targets</h2>
            <p class="text-sm text-crush-textMuted mb-6 leading-relaxed">
              Each row shows the one-time authentication and the deploy command. Stack detection
              filters targets — a static site won't be offered backend-only options.
            </p>
            <div class="space-y-3">
              @for (p of providers; track p.tag) {
                <div class="rounded-xl border border-crush-border/30 bg-crush-surface/20 p-4">
                  <div class="flex items-center justify-between gap-3 mb-2">
                    <h3 class="text-sm font-bold text-white">{{ p.name }}</h3>
                    <code class="text-[11px] text-crush-textMuted">--target {{ p.tag }}</code>
                  </div>
                  <p class="text-xs text-crush-textMuted mb-3 leading-relaxed">{{ p.blurb }}</p>
                  <div class="grid gap-2 sm:grid-cols-2 font-mono text-[12px]">
                    <div class="rounded-lg border border-crush-border/30 bg-crush-black/50 px-3 py-2">
                      <span class="text-crush-textMuted text-[10px] uppercase block mb-0.5">Authenticate</span>
                      <span class="text-crush-text break-all">{{ p.auth }}</span>
                    </div>
                    <div class="rounded-lg border border-crush-border/30 bg-crush-black/50 px-3 py-2">
                      <span class="text-crush-textMuted text-[10px] uppercase block mb-0.5">Deploy</span>
                      <span class="text-crush-text break-all">{{ p.deploy }}</span>
                    </div>
                  </div>
                </div>
              }
            </div>
          </section>

          <!-- Environment & secrets -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Environment variables &amp; secrets</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Variables from your <code>Crushfile</code>, <code>.env</code>, or compose
              <code>environment:</code> are carried into the deployment. Pass extra ones with
              <code>--env</code>, repeatable. Secrets are forwarded to the provider's own secret store
              rather than baked into the image.
            </p>
            <div class="rounded-xl border border-crush-border/40 bg-crush-black/60 p-4 font-mono text-sm text-crush-text overflow-x-auto">
              <div>crush deploy --target fly --env NODE_ENV=production --env API_KEY=$API_KEY</div>
            </div>
          </section>

          <!-- Registry-free / VPS -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Deploy to a plain VPS (registry-free)</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              No registry account needed. Crush builds the image, streams it over SSH to your server,
              and runs it. Ideal for Hetzner, a DigitalOcean Droplet, or any bare-metal Linux box.
            </p>
            <div class="rounded-xl border border-crush-border/40 bg-crush-black/60 p-4 font-mono text-sm text-crush-text overflow-x-auto">
              <div class="text-crush-textMuted"># ship the current project to a server you can SSH into</div>
              <div>crush deploy --target ssh --host deploy&#64;203.0.113.10</div>
            </div>
          </section>

          <!-- Eject -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Eject — generate a Dockerfile</h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Need a custom pipeline, or want zero lock-in? <code>crush eject</code> writes a
              production-ready <code>Dockerfile</code> and <code>docker-compose.yml</code> for your
              detected stack, so you can build and deploy with plain Docker anywhere.
            </p>
            <div class="rounded-xl border border-crush-border/40 bg-crush-black/60 p-4 font-mono text-sm text-crush-text overflow-x-auto">
              <div>crush eject --force</div>
              <div class="text-crush-textMuted"># → writes Dockerfile + docker-compose.yml in the project root</div>
            </div>
          </section>

          <!-- CI/CD -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Use in CI/CD</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              The CLI is a single binary, so it drops into any pipeline. Authenticate the provider via
              its CLI or environment token, then run the same deploy command non-interactively.
            </p>
            <div class="rounded-xl border border-crush-border/40 bg-crush-black/60 p-4 font-mono text-sm text-crush-text overflow-x-auto">
              <div class="text-crush-textMuted"># GitHub Actions steps</div>
              <div>- run: curl -fsSL https://crush-web-six.vercel.app/install.sh | sh</div>
              <div>- run: crush deploy --target fly</div>
              <div class="text-crush-textMuted">&nbsp;&nbsp;# expose FLY_API_TOKEN from your CI secrets as an env var</div>
            </div>
          </section>

          <!-- Troubleshooting -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Troubleshooting</h2>
            <div class="space-y-3 text-sm">
              <div class="rounded-lg border border-crush-border/30 bg-crush-surface/20 p-4">
                <p class="text-white font-semibold mb-1">"Provider CLI not found"</p>
                <p class="text-xs text-crush-textMuted leading-relaxed">Install the target's CLI and ensure it's on your PATH. Re-check under Settings → provider status.</p>
              </div>
              <div class="rounded-lg border border-crush-border/30 bg-crush-surface/20 p-4">
                <p class="text-white font-semibold mb-1">"Not authenticated"</p>
                <p class="text-xs text-crush-textMuted leading-relaxed">Run the provider's login command (see the table above) or export its API token, then retry.</p>
              </div>
              <div class="rounded-lg border border-crush-border/30 bg-crush-surface/20 p-4">
                <p class="text-white font-semibold mb-1">App starts but isn't reachable</p>
                <p class="text-xs text-crush-textMuted leading-relaxed">Make sure your app binds <code>0.0.0.0</code> (not <code>127.0.0.1</code>) and the port matches your Crushfile/compose <code>ports:</code>.</p>
              </div>
            </div>
          </section>

          <!-- Next -->
          <div class="border-t border-crush-border/30 pt-6 flex flex-wrap gap-4 text-sm">
            <a routerLink="/docs/crushfile" class="text-crush-orange hover:text-crush-orangeLight">Crushfile schema →</a>
            <a routerLink="/docs/services" class="text-crush-orange hover:text-crush-orangeLight">Native services →</a>
            <a routerLink="/docs/docker-migration" class="text-crush-orange hover:text-crush-orangeLight">Docker migration →</a>
          </div>
        </article>
      </div>
    </div>
  `,
})
export default class DeployPageComponent implements OnInit {
  providers = PROVIDERS;

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit() {
    this.title.setTitle('Cloud Deployments - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content:
        'Deploy your project to Railway, Fly.io, Cloud Run, AWS, Azure, DigitalOcean, a VPS over SSH, and more — one command, no lock-in. Crush builds a standard OCI image and ships it.',
    });
  }
}
