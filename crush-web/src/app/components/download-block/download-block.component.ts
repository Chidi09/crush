import { Component, OnInit, signal, computed, inject, PLATFORM_ID } from '@angular/core';
import { CommonModule, isPlatformBrowser } from '@angular/common';
import { CopyService } from '../../lib/copy.service';
import { RELEASE } from './release.data';

const REPO = 'Chidi09/crush';
const RELEASES = `https://github.com/${REPO}/releases`;

interface Asset {
  name: string;
  url: string;
  size: number;
}
interface DownloadLink {
  os: 'windows' | 'macos' | 'linux';
  label: string; // e.g. "Installer (.msi)"
  sub?: string; // e.g. "Recommended"
  url: string; // direct asset, or releases page as fallback
  size?: number;
}

// Package-manager install commands for the CLI (always available, version-agnostic).
const CLI_METHODS = [
  { label: 'Linux / macOS', filename: 'install.sh', command: 'curl -fsSL https://crush-web-six.vercel.app/install.sh | sh' },
  { label: 'Windows (PowerShell)', filename: 'install.ps1', command: 'irm https://crush-web-six.vercel.app/install.ps1 | iex' },
  { label: 'Homebrew', filename: 'brew', command: 'brew install crushcontainer/tap/crush' },
  { label: 'Cargo', filename: 'cargo', command: 'cargo install crush-cli' },
  { label: 'Winget', filename: 'winget', command: 'winget install Crush.Crush' },
  { label: 'Scoop', filename: 'scoop', command: 'scoop bucket add crush https://github.com/crushcontainer/scoop-bucket\nscoop install crush' },
];

@Component({
  selector: 'app-download-block',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="w-full max-w-5xl mx-auto">
      <!-- Latest version pill -->
      <div class="flex justify-center mb-8">
        <a
          [href]="RELEASES"
          target="_blank"
          rel="noopener"
          class="always-dark inline-flex items-center gap-2 rounded-full border border-crush-border/40 bg-crush-dark/40 backdrop-blur-md px-4 py-1.5 text-xs font-semibold text-crush-textMuted hover:text-white hover:border-crush-orange/40 transition-colors"
        >
          <span class="w-2 h-2 rounded-full bg-crush-green block"></span>
          @if (version()) {
            Latest release {{ version() }}
          } @else {
            View all releases
          }
          <span *ngIf="detectedOs()" class="text-crush-orange">· detected {{ osLabel() }}</span>
        </a>
      </div>

      <div class="grid gap-6 lg:grid-cols-2">
        <!-- ── Desktop GUI ─────────────────────────────────────────── -->
        <div
          class="relative overflow-hidden rounded-2xl border border-crush-border/60 bg-card p-7 flex flex-col"
        >
          <div
            class="absolute -right-16 -top-16 w-36 h-36 rounded-full bg-crush-orange/5 blur-3xl pointer-events-none"
          ></div>
          <div class="flex items-center gap-3 mb-2">
            <div
              class="h-11 w-11 flex items-center justify-center rounded-xl bg-crush-orange/10 text-crush-orange border border-crush-orange/20 shrink-0"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-5 w-5">
                <rect x="2" y="3" width="20" height="14" rx="2" />
                <line x1="8" y1="21" x2="16" y2="21" />
                <line x1="12" y1="17" x2="12" y2="21" />
              </svg>
            </div>
            <div>
              <h3 class="text-lg font-bold text-white">Desktop App</h3>
              <p class="text-xs text-crush-textMuted">Full GUI dashboard — previews, logs, AI diagnosis</p>
            </div>
          </div>

          <div class="mt-5 space-y-5 flex-1">
            @for (grp of guiGroups(); track grp.os) {
              <div [class.order-first]="grp.os === detectedOs()">
                <div class="flex items-center gap-2 mb-2">
                  <svg viewBox="0 0 24 24" class="h-3.5 w-3.5 fill-crush-textMuted" role="img">
                    <path [attr.d]="osIcon(grp.os)" />
                  </svg>
                  <span class="text-xs font-semibold text-crush-textMuted uppercase tracking-wide">
                    {{ osTitle(grp.os) }}
                    <span *ngIf="grp.os === detectedOs()" class="text-crush-orange normal-case">· your platform</span>
                  </span>
                </div>
                <div class="flex flex-wrap gap-2">
                  @for (link of grp.links; track link.label) {
                    <a
                      [href]="link.url"
                      class="group inline-flex items-center gap-2 px-3.5 py-2 rounded-lg text-xs font-semibold transition-all duration-200"
                      [ngClass]="link.sub === 'Recommended'
                        ? 'bg-gradient-to-r from-crush-orange to-crush-orangeLight text-white shadow-lg shadow-crush-orange/15 hover:scale-[1.02]'
                        : 'text-crush-text bg-crush-surface border border-crush-border/60 hover:border-crush-orange/50 hover:text-white'"
                    >
                      <svg viewBox="0 0 24 24" class="h-3.5 w-3.5 fill-none stroke-current stroke-2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="7 10 12 15 17 10" />
                        <line x1="12" y1="15" x2="12" y2="3" />
                      </svg>
                      {{ link.label }}
                      <span *ngIf="link.sub" class="text-[10px] opacity-80">· {{ link.sub }}</span>
                    </a>
                  }
                </div>
              </div>
            }

            <!-- macOS note when no GUI build is published yet -->
            @if (!hasMacGui()) {
              <div class="flex items-center gap-2 text-xs text-crush-textMuted">
                <svg viewBox="0 0 24 24" class="h-3.5 w-3.5 fill-crush-textMuted" role="img">
                  <path [attr.d]="osIcon('macos')" />
                </svg>
                <span>macOS — <a [href]="GUI_SOURCE" target="_blank" rel="noopener" class="text-crush-orange hover:underline">build from source</a> (signed bundles coming soon)</span>
              </div>
            }
          </div>
        </div>

        <!-- ── CLI ──────────────────────────────────────────────────── -->
        <div
          class="relative overflow-hidden rounded-2xl border border-crush-border/60 bg-card p-7 flex flex-col"
        >
          <div class="flex items-center gap-3 mb-2">
            <div
              class="h-11 w-11 flex items-center justify-center rounded-xl bg-crush-surface text-crush-text border border-crush-border/60 shrink-0"
            >
              <svg viewBox="0 0 24 24" class="h-5 w-5 fill-none stroke-current stroke-2">
                <polyline points="4 17 10 11 4 5" />
                <line x1="12" y1="19" x2="20" y2="19" />
              </svg>
            </div>
            <div>
              <h3 class="text-lg font-bold text-white">Command-Line (CLI)</h3>
              <p class="text-xs text-crush-textMuted">A single binary — scriptable, CI-friendly</p>
            </div>
          </div>

          <!-- Direct binary downloads -->
          <div class="mt-5 space-y-2">
            @for (link of cliLinks(); track link.label) {
              <a
                [href]="link.url"
                class="flex items-center justify-between gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium bg-crush-surface/40 border border-crush-border/50 hover:border-crush-orange/50 hover:bg-crush-surface/80 transition-all duration-200"
                [class.ring-1]="link.os === detectedOs()"
                [class.ring-crush-orange]="link.os === detectedOs()"
              >
                <span class="flex items-center gap-2.5 text-crush-text">
                  <svg viewBox="0 0 24 24" class="h-3.5 w-3.5 fill-crush-textMuted" role="img">
                    <path [attr.d]="osIcon(link.os)" />
                  </svg>
                  {{ link.label }}
                </span>
                <span class="text-crush-textMuted font-mono text-[11px]">{{ link.size ? fmt(link.size) : 'download' }}</span>
              </a>
            }
          </div>

          <!-- Package managers -->
          <div class="mt-5 pt-5 border-t border-crush-border/30">
            <p class="text-xs font-semibold text-crush-textMuted uppercase tracking-wide mb-3">Or via a package manager</p>
            <div class="always-dark flex flex-wrap gap-1.5 mb-3">
              @for (m of cliMethods; track m.label) {
                <button
                  (click)="method.set(m)"
                  class="px-3 py-1.5 rounded-md text-[11px] font-semibold transition-all"
                  [ngClass]="method().label === m.label
                    ? 'bg-crush-orange/15 text-crush-orange border border-crush-orange/30'
                    : 'text-crush-textMuted border border-crush-border/40 hover:text-white'"
                >{{ m.label }}</button>
              }
            </div>
            <div class="always-dark rounded-lg border border-crush-border/50 bg-crush-black/80 px-4 py-3 flex items-center justify-between gap-4">
              <code class="font-mono text-xs text-crush-text whitespace-pre-wrap break-all">{{ method().command }}</code>
              <button
                (click)="copy()"
                class="shrink-0 px-2.5 py-1.5 rounded-md border border-crush-border/60 text-[11px] font-semibold text-crush-textMuted hover:text-white hover:border-crush-orange/50 transition-all"
              >{{ isCopied() ? 'Copied!' : 'Copy' }}</button>
            </div>
          </div>
        </div>
      </div>

      <p class="text-center text-xs text-crush-textMuted mt-6">
        All releases are built and published from CI.
        <a [href]="RELEASES" target="_blank" rel="noopener" class="text-crush-orange hover:underline">Browse every version, checksums, and changelog →</a>
      </p>
    </div>
  `,
})
export class DownloadBlockComponent implements OnInit {
  private copyService = inject(CopyService);
  private platformId = inject(PLATFORM_ID);

  readonly RELEASES = RELEASES;
  readonly GUI_SOURCE = 'https://github.com/Chidi09/crush#run-gui-from-source';

  cliMethods = CLI_METHODS;
  method = signal(CLI_METHODS[0]!);
  isCopied = signal(false);

  // Seeded from build-time release data (baked by scripts/fetch-release.mjs),
  // so there is no client-side GitHub API call and no per-visitor rate limit.
  version = signal<string>(RELEASE.version);
  detectedOs = signal<'windows' | 'macos' | 'linux' | ''>('');
  private assets = signal<Asset[]>(RELEASE.assets ?? []);

  // ── GUI downloads grouped by OS, with the recommended option first ──────
  guiGroups = computed(() => {
    const a = this.assets();
    const groups: { os: 'windows' | 'macos' | 'linux'; links: DownloadLink[] }[] = [];

    const winLinks: DownloadLink[] = [];
    const msi = a.find((x) => /\.msi$/i.test(x.name));
    const setup = a.find((x) => /-setup\.exe$/i.test(x.name));
    if (msi) winLinks.push({ os: 'windows', label: 'Installer (.msi)', sub: 'Recommended', url: msi.url, size: msi.size });
    if (setup) winLinks.push({ os: 'windows', label: 'Setup (.exe)', url: setup.url, size: setup.size });
    if (winLinks.length) groups.push({ os: 'windows', links: winLinks });

    const linLinks: DownloadLink[] = [];
    const appimage = a.find((x) => /\.AppImage$/i.test(x.name));
    const deb = a.find((x) => /\.deb$/i.test(x.name));
    if (appimage) linLinks.push({ os: 'linux', label: 'AppImage', sub: 'Recommended', url: appimage.url, size: appimage.size });
    if (deb) linLinks.push({ os: 'linux', label: '.deb', url: deb.url, size: deb.size });
    if (linLinks.length) groups.push({ os: 'linux', links: linLinks });

    const macDmg = a.find((x) => /\.(dmg|app\.tar\.gz)$/i.test(x.name));
    if (macDmg) groups.push({ os: 'macos', links: [{ os: 'macos', label: macDmg.name.endsWith('.dmg') ? 'Disk image (.dmg)' : 'App bundle', sub: 'Recommended', url: macDmg.url, size: macDmg.size }] });

    // Fallback to the releases page if the API hasn't resolved yet.
    if (!groups.length) {
      groups.push(
        { os: 'windows', links: [{ os: 'windows', label: 'Windows installer', sub: 'Recommended', url: RELEASES }] },
        { os: 'linux', links: [{ os: 'linux', label: 'Linux (AppImage / .deb)', url: RELEASES }] },
      );
    }
    return groups;
  });

  hasMacGui = computed(() => this.guiGroups().some((g) => g.os === 'macos'));

  // ── CLI binary downloads per OS ─────────────────────────────────────────
  cliLinks = computed(() => {
    const a = this.assets();
    const out: DownloadLink[] = [];
    // CLI binaries are named crush-<ver>-<os>-<arch>[.exe]; exclude GUI installers.
    const win = a.find((x) => /^crush[-_].*windows.*\.exe$/i.test(x.name) && !/-setup\.exe$/i.test(x.name));
    const mac = a.find((x) => /^crush[-_].*(darwin|macos|apple)/i.test(x.name));
    const lin = a.find((x) => /^crush[-_].*linux/i.test(x.name) && !/\.(deb|AppImage)$/i.test(x.name));
    if (win) out.push({ os: 'windows', label: 'Windows · x86_64', url: win.url, size: win.size });
    if (mac) out.push({ os: 'macos', label: 'macOS', url: mac.url, size: mac.size });
    if (lin) out.push({ os: 'linux', label: 'Linux · x86_64', url: lin.url, size: lin.size });
    if (!out.length) {
      out.push(
        { os: 'windows', label: 'Windows · x86_64', url: RELEASES },
        { os: 'linux', label: 'Linux · x86_64', url: RELEASES },
      );
    }
    // Put the visitor's platform first.
    const me = this.detectedOs();
    return out.sort((x, y) => (x.os === me ? -1 : y.os === me ? 1 : 0));
  });

  ngOnInit(): void {
    if (!isPlatformBrowser(this.platformId)) return;
    this.detectOs();
  }

  private detectOs(): void {
    const ua = (navigator.userAgent || navigator.platform || '').toLowerCase();
    if (ua.includes('win')) this.detectedOs.set('windows');
    else if (ua.includes('mac')) this.detectedOs.set('macos');
    else if (ua.includes('linux') || ua.includes('x11')) this.detectedOs.set('linux');
  }

  osLabel = computed(() => this.osTitle(this.detectedOs() || 'linux'));
  osTitle(os: string): string {
    return os === 'windows' ? 'Windows' : os === 'macos' ? 'macOS' : 'Linux';
  }
  osIcon(os: string): string {
    if (os === 'windows') return 'M0 0h11.377v11.373H0zm12.623 0H24v11.373H12.623zM0 12.627h11.377V24H0zm12.623 0H24V24H12.623z';
    if (os === 'macos') return 'M16.365 1.43c0 1.14-.493 2.27-1.177 3.08-.744.9-1.99 1.57-2.987 1.57-.12 0-.23-.02-.3-.03-.01-.06-.04-.22-.04-.39 0-1.15.572-2.27 1.206-2.98.804-.94 2.142-1.62 3.248-1.66.03.13.05.28.05.42zm4.565 15.71c-.03.07-.463 1.58-1.518 3.12-.945 1.34-1.94 2.71-3.43 2.71-1.517 0-1.9-.88-3.63-.88-1.698 0-2.302.91-3.67.91-1.377 0-2.332-1.26-3.428-2.8-1.287-1.82-2.323-4.63-2.323-7.28 0-4.28 2.797-6.55 5.552-6.55 1.448 0 2.675.95 3.6.95.865 0 2.222-1.01 3.902-1.01.613 0 2.886.06 4.374 2.19-.13.09-2.383 1.37-2.383 4.19 0 3.26 2.854 4.42 2.955 4.45z';
    return 'M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.134.063.198.114.333l.003.003c.391.778 1.113 1.132 1.884 1.071.771-.06 1.592-.536 2.257-1.306.631-.765 1.683-1.084 2.378-1.503.348-.21.629-.469.649-.853.023-.4-.2-.811-.714-1.376v-.097l-.003-.003c-.17-.2-.25-.535-.338-.926-.085-.401-.182-.786-.492-1.046h-.003c-.059-.054-.123-.067-.188-.135a.357.357 0 00-.19-.064c.431-1.278.264-2.55-.173-3.694-.533-1.41-1.465-2.638-2.175-3.483-.796-1.005-1.576-1.957-1.56-3.368.026-2.152.236-6.133-3.544-6.139z';
  }

  fmt(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    const u = ['KB', 'MB', 'GB'];
    let i = -1, n = bytes;
    do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
    return n.toFixed(n < 10 ? 1 : 0) + ' ' + u[i];
  }

  async copy(): Promise<void> {
    const ok = await this.copyService.copy(this.method().command);
    if (ok) {
      this.isCopied.set(true);
      setTimeout(() => this.isCopied.set(false), 2000);
    }
  }
}
