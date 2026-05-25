import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-security',
  standalone: true,
  imports: [RouterLink, DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange">Guide</span>
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">Security</h1>
            <p class="text-base text-crush-textMuted">
              Secret scanning, cryptographically signed releases, and kernel runtime isolation
              shields.
            </p>
          </div>

          <!-- Section 1 -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">
              Secret Detection & Auto-Migration
            </h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              During compiling stages, Crush automatically reviews project directories for leaked
              variables (AWS keys, private certificates, DB strings) and alerts builders immediately
              before execution.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-2">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">crush CLI</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >build scan</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted">~/my-api $</span>
                <span class="text-crush-orange font-bold">crush</span> build
                <span class="text-crush-textMuted"
                  >↳ detected: Node.js 20 · TypeScript · Express</span
                >
                <span class="text-amber-400"
                  >⚠ Secret warning: AWS_ACCESS_KEY_ID inside src/config.ts:L14</span
                >
                <span class="text-crush-textMuted"
                  >↳ Run '<span class="text-crush-orange font-bold">crush</span> secrets set
                  AWS_ACCESS_KEY_ID' to migrate</span
                >
                <span class="text-emerald-400"
                  >✓ build complete (secret excluded automatically)</span
                >
              </div>
            </div>
          </section>

          <!-- Section 2 -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Encrypted Secret Store</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Sensitive credentials are encrypted natively using local AES-256-GCM algorithms and
              decapsulated only in memory space at container runtime boot phases.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-2">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >secrets</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"
                  ># Encrypt new credentials in the local vault</span
                >
                <span class="text-crush-orange font-bold">crush</span> secrets set DATABASE_URL
                "postgres://user:pass&#64;host:5432/db"

                <span class="text-crush-textMuted"
                  ># Bind credentials inside the Crushfile context</span
                >
                [runtime] secrets = ["DATABASE_URL"]
              </div>
            </div>
          </section>

          <!-- Section 3 -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Release Signature Audits</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Every installation package is cryptographically signed at build time inside GitHub
              Actions environments using keyless Sigstore / Cosign infrastructures.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-2">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >signature check</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"
                  ># Verify signing blob integrity using public Cosign chains</span
                >
                cosign verify-blob crush-linux-amd64.tar.gz \\ --signature
                crush-linux-amd64.tar.gz.sig \\ --certificate crush-linux-amd64.tar.gz.pem
              </div>
            </div>
          </section>

          <!-- Section 4: Vulnerability -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Vulnerability Scanning</h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Scan constructed container images for outdated dependencies or insecure nested
              packages directly inside your terminal loop.
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
              >
                <div class="flex items-center gap-2">
                  <span class="w-2 h-2 rounded-full bg-[#ff5f56]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#ffbd2e]"></span>
                  <span class="w-2 h-2 rounded-full bg-[#27c93f]"></span>
                  <span class="text-[10px] text-crush-textMuted font-mono ml-2">Terminal</span>
                </div>
                <span class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                  >scanner</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"># Execute scanning cycle</span>
                <span class="text-crush-orange font-bold">crush</span> scan myapp:latest

                <span class="text-crush-textMuted"
                  ># Export SPDX-compliant Software Bill of Materials (SBOM)</span
                >
                <span class="text-crush-orange font-bold">crush</span> sbom myapp:latest --format
                spdx-json
              </div>
            </div>
          </section>

          <!-- Section 5: Runtime -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none font-sans">
              Runtime Isolation Parameters
            </h2>
            <p class="text-sm text-crush-textMuted mb-6 leading-relaxed">
              When executing container processes, Crush enforces high-level Linux and Windows kernel
              runtime isolation properties:
            </p>

            <div class="grid gap-4 sm:grid-cols-2">
              <div
                class="rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5 hover:border-crush-orange/20 transition-all duration-300"
              >
                <h4 class="font-bold text-white mb-2 text-sm">Capability Dropping</h4>
                <p class="text-xs text-crush-textMuted leading-relaxed">
                  Processes discard high-level administrative system calls immediately at start
                  phase, running under highly restricted user scopes.
                </p>
              </div>

              <div
                class="rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5 hover:border-crush-orange/20 transition-all duration-300"
              >
                <h4 class="font-bold text-white mb-2 text-sm">Seccomp Filters</h4>
                <p class="text-xs text-crush-textMuted leading-relaxed">
                  A default system call filter intercepts and disables high-risk system calls at the
                  OS kernel layer.
                </p>
              </div>

              <div
                class="rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5 hover:border-crush-orange/20 transition-all duration-300"
              >
                <h4 class="font-bold text-white mb-2 text-sm">Read-Only rootfs</h4>
                <p class="text-xs text-crush-textMuted leading-relaxed">
                  Supports full file-system locks to compile a fully immutable sandbox filesystem
                  preventing runtime script injections.
                </p>
              </div>

              <div
                class="rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5 hover:border-crush-orange/20 transition-all duration-300"
              >
                <h4 class="font-bold text-white mb-2 text-sm">Job Object Resource Limits</h4>
                <p class="text-xs text-crush-textMuted leading-relaxed">
                  Strictly defines CPU cycles and memory allocations inside Windows native process
                  chains, guarding hosts from memory exhaustion.
                </p>
              </div>
            </div>
          </section>

          <!-- Footer Navigation Links -->
          <div
            class="flex items-center justify-between border-t border-crush-border/30 pt-8 mt-16 select-none"
          >
            <a
              routerLink="/docs/windows"
              class="inline-flex items-center gap-2 text-sm text-crush-textMuted hover:text-white transition-colors"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-4 w-4"
              >
                <line x1="19" y1="12" x2="5" y2="12" />
                <polyline points="12 19 5 12 12 5" />
              </svg>
              Windows Guide
            </a>
            <a
              routerLink="/docs"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Back to Overview
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-4 w-4"
              >
                <line x1="5" y1="12" x2="19" y2="12" />
                <polyline points="12 5 19 12 12 19" />
              </svg>
            </a>
          </div>
        </article>
      </div>
    </div>
  `,
})
export default class SecurityPage implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Security — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Crush security features — secret detection, encrypted store, image signing, vulnerability scanning.',
    });
  }
}
