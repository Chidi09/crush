import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';
import { InstallBlockComponent } from '../../components/install-block/install-block.component';
import { DownloadBlockComponent } from '../../components/download-block/download-block.component';

@Component({
  selector: 'page-installation',
  standalone: true,
  imports: [RouterLink, DocsSidebarComponent, InstallBlockComponent, DownloadBlockComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange">Guide</span>
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              Installation
            </h1>
            <p class="text-base text-crush-textMuted">
              Install Crush across Windows, macOS, Linux, and cloud servers.
            </p>
          </div>

          <!-- Section 0: Direct downloads (GUI + CLI) -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-6 flex items-center gap-2.5 select-none">
              Download
            </h2>
            <app-download-block />
          </section>

          <!-- Section 1: Quick Install (package managers) -->
          <section class="mb-14">
            <h2 class="text-xl font-bold text-white mb-6 flex items-center gap-2.5 select-none">
              Install via package manager
            </h2>
            <app-install-block />
          </section>

          <!-- Section 2: APT -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4 flex items-center gap-2 select-none">
              APT repository
              <span class="text-xs font-normal text-crush-textMuted">(Debian / Ubuntu)</span>
              <span class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20">coming soon</span>
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              The APT repository is planned. For now, use the direct download above or install the <code class="text-crush-orange">.deb</code> from the GitHub releases page. The commands below show the planned form for when the repo goes live.
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
                  >apt-get</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"># Add official signing repository</span>
                echo "deb [signed-by=/usr/share/keyrings/crush.gpg] https://apt.crushrun.dev/ stable
                main" | sudo tee /etc/apt/sources.list.d/crush.list

                <span class="text-crush-textMuted"># Update package feeds and install</span>
                sudo apt update && sudo apt install crush
              </div>
            </div>
          </section>

          <!-- Section 3: DNF -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4 flex items-center gap-2 select-none">
              RPM repository
              <span class="text-xs font-normal text-crush-textMuted"
                >(Fedora / Red Hat / Rocky)</span
              >
              <span class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20">coming soon</span>
            </h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              RPM repository support is planned. Currently install via the direct download or from source. The commands below show the planned form.
            </p>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed hidden">
              Manage installation package cycles natively through standard yum or dnf repository
              configurations.
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
                  >dnf</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"># Configure system package feed</span>
                sudo dnf config-manager --add-repo https://rpm.crushrun.dev/crush.repo

                <span class="text-crush-textMuted"># Install binary suite</span>
                sudo dnf install crush
              </div>
            </div>
          </section>

          <!-- Section 4: AUR -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4 flex items-center gap-2 select-none">
              Arch User Repository
              <span class="text-xs font-normal text-crush-textMuted">(Arch Linux / Manjaro)</span>
              <span class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20">coming soon</span>
            </h2>

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
                  >pacman / aur</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"># Using yay aur manager</span>
                yay -S crush-bin

                <span class="text-crush-textMuted"># Using paru aur manager</span>
                paru -S crush-bin
              </div>
            </div>
          </section>

          <!-- Section 5: Manual Binary Releases -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Manual Binary Downloads</h2>
            <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">
              Static pre-compiled binaries are built for every commit and release tag. Download
              packages directly from the official
              <a
                href="https://github.com/Chidi09/crush/releases"
                target="_blank"
                rel="noopener"
                class="text-crush-orange hover:text-crush-orangeLight transition-colors font-semibold"
                >GitHub Releases</a
              >
              portal.
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
                  >cosign verification</span
                >
              </div>
              <div
                class="p-4 font-mono text-sm overflow-x-auto text-crush-text leading-relaxed whitespace-pre"
              >
                <span class="text-crush-textMuted"
                  ># Verify image or binary signature integrity using cosign</span
                >
                cosign verify-blob crush-$(uname -s)-$(uname -m).tar.gz \\ --signature crush-$(uname
                -s)-$(uname -m).tar.gz.sig \\ --certificate crush-$(uname -s)-$(uname -m).tar.gz.pem
              </div>
            </div>

            <div
              class="flex gap-3.5 p-4 rounded-xl border border-emerald-500/15 bg-emerald-500/5 text-xs sm:text-sm select-none"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="h-5 w-5 text-emerald-400 shrink-0 mt-0.5"
              >
                <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <p class="font-bold text-emerald-400 mb-0.5">Supply Chain Security</p>
                <p class="text-crush-textMuted leading-relaxed">
                  All distribution packages and releases are cryptographically signed at build time
                  inside GitHub Actions environments using keyless Sigstore / Cosign
                  infrastructures.
                </p>
              </div>
            </div>
          </section>

          <!-- Section 6: Verification -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4 select-none">Verify Your Install</h2>
            <p class="text-sm text-crush-textMuted mb-4">
              Confirm your local installations were linked properly:
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
                  >shell</span
                >
              </div>
              <div class="p-4 font-mono text-sm overflow-x-auto text-crush-text">
                <code><span class="text-crush-orange font-bold">crush</span> --version</code>
              </div>
            </div>
          </section>

          <!-- Footer Navigation Links -->
          <div
            class="flex items-center justify-between border-t border-crush-border/30 pt-8 mt-16 select-none"
          >
            <a
              routerLink="/docs"
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
              Back to Overview
            </a>
            <a
              routerLink="/docs/getting-started"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Quick Start Guide
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
export default class InstallationPage implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Installation — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Install Crush on Windows, macOS, Linux. curl, Homebrew, Cargo, Winget, Scoop, APT, DNF, AUR.',
    });
  }
}
