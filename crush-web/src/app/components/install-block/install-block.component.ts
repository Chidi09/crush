import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { CopyService } from '../../lib/copy.service';

interface InstallMethod {
  label: string;
  filename: string;
  command: string;
  iconPath: string;
}

const METHODS: InstallMethod[] = [
  {
    label: 'Linux / macOS',
    filename: 'install.sh',
    command: 'curl -fsSL https://crush-web-six.vercel.app/install.sh | sh',
    iconPath: 'M2 4h20v16H2V4zm2 4v10h16V8H4zm3 2h2v2H7v-2zm4 0h6v2h-6v-2z', // Terminal SVG
  },
  {
    label: 'Windows (PowerShell)',
    filename: 'install.ps1',
    command: 'irm https://crush-web-six.vercel.app/install.ps1 | iex',
    iconPath:
      'M0 0h11.377v11.373H0zm12.623 0H24v11.373H12.623zM0 12.627h11.377V24H0zm12.623 0H24V24H12.623z', // Windows SVG
  },
  {
    label: 'Homebrew',
    filename: 'brew-install.sh',
    command: 'brew install crushcontainer/tap/crush',
    iconPath:
      'M2 21h18v-2H2v2zM20 8h-2V5h2V3h-2V1h-2v2h-2V1h-2v2h-2V1H8v2H6V1H4v2h2V5H4v3H2v10h18V8zM6 5h12v11H6V5z', // Brewery/Brew Mug
  },
  {
    label: 'Cargo',
    filename: 'Cargo.toml',
    command: 'cargo install crush-cli',
    iconPath:
      'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zm0-3a7 7 0 1 1 0-14 7 7 0 0 1 0 14zm-1-10h2v4h-2V9zm0 5h2v2h-2v-2z', // Rust Gear SVG
  },
  {
    label: 'Winget',
    filename: 'winget.cmd',
    command: 'winget install Crush.Crush',
    iconPath:
      'M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2v9zM4 7v12h16V8h-9.5l-2-3H4v2z', // Package folder SVG
  },
  {
    label: 'Scoop',
    filename: 'scoop.ps1',
    command:
      'scoop bucket add crush https://github.com/crushcontainer/scoop-bucket\nscoop install crush',
    iconPath: 'M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm1 14h-2v-2h2zm0-4h-2V7h2z', // Scoop bucket SVG
  },
];

@Component({
  selector: 'app-install-block',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="w-full max-w-3xl mx-auto space-y-6">
      <!-- Premium Glass Segmented Control / Tabs -->
      <div
        class="always-dark flex flex-wrap justify-center gap-2 p-1.5 rounded-xl border border-crush-border/40 bg-crush-dark/40 backdrop-blur-md"
      >
        @for (method of methods; track method.label) {
          <button
            (click)="selectedMethod.set(method)"
            class="flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold tracking-wide transition-all duration-300 select-none outline-none"
            [ngClass]="
              selectedMethod().label === method.label
                ? 'bg-gradient-to-r from-crush-orange to-crush-orangeLight text-white shadow-lg shadow-crush-orange/15 scale-[1.02]'
                : 'text-crush-textMuted hover:text-white hover:bg-crush-surface/50'
            "
          >
            <svg
              role="img"
              viewBox="0 0 24 24"
              class="h-3.5 w-3.5 fill-current"
              [ngClass]="
                selectedMethod().label === method.label
                  ? 'text-white'
                  : 'text-crush-textMuted group-hover:text-white'
              "
            >
              <path [attr.d]="method.iconPath" />
            </svg>
            {{ method.label }}
          </button>
        }
      </div>

      <!-- Professional macOS-Style Developer Terminal -->
      <div
        class="always-dark group relative rounded-xl border border-crush-border/50 bg-crush-black/85 shadow-2xl shadow-crush-orange/5 overflow-hidden transition-all duration-300 hover:border-crush-orange/30"
      >
        <!-- Terminal Header -->
        <div
          class="flex items-center justify-between px-4 py-3 border-b border-crush-border/30 bg-crush-dark/80"
        >
          <div class="flex items-center gap-1.5 select-none">
            <span class="w-3 h-3 rounded-full bg-[#ff5f56] block"></span>
            <span class="w-3 h-3 rounded-full bg-[#ffbd2e] block"></span>
            <span class="w-3 h-3 rounded-full bg-[#27c93f] block"></span>
          </div>
          <div
            class="text-[11px] font-mono font-medium text-crush-textMuted tracking-wider select-none"
          >
            {{ selectedMethod().filename }}
          </div>
          <div class="w-14"></div>
          <!-- Spacer to center title -->
        </div>

        <!-- Terminal Body / Code Block -->
        <div
          class="p-5 font-mono text-sm leading-relaxed text-crush-text overflow-x-auto relative min-h-[85px] flex items-center justify-between gap-6"
        >
          <div class="flex items-start gap-3">
            <span class="text-crush-orange/60 font-semibold select-none">$</span>
            <code class="text-crush-text font-medium select-text whitespace-pre-wrap break-all">{{
              selectedMethod().command
            }}</code>
          </div>

          <!-- Slick Floating Copy Button -->
          <button
            (click)="copy()"
            class="flex items-center gap-2 px-3.5 py-2 rounded-lg border border-crush-border/60 bg-crush-surface/40 text-xs font-semibold tracking-wide transition-all duration-300 hover:text-white hover:border-crush-orange/50 hover:bg-crush-surface/80 active:scale-95 shrink-0 self-center group/btn"
          >
            @if (isCopied()) {
              <svg
                viewBox="0 0 24 24"
                class="h-4 w-4 fill-none stroke-current stroke-2 text-crush-green animate-fade-in"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span class="text-crush-green animate-fade-in font-mono">Copied!</span>
            } @else {
              <svg
                viewBox="0 0 24 24"
                class="h-4 w-4 fill-none stroke-current stroke-2 text-crush-textMuted group-hover/btn:text-white animate-fade-in"
              >
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <span class="text-crush-textMuted font-mono group-hover/btn:text-white">Copy</span>
            }
          </button>
        </div>
      </div>

      <!-- Premium GUI Installer / Run from Source Promo Card -->
      <div
        class="rounded-xl border border-crush-border/30 bg-crush-surface/10 p-5 flex flex-col sm:flex-row items-center justify-between gap-4 transition-all duration-300 hover:border-crush-orange/20"
      >
        <div class="flex items-start gap-4">
          <div
            class="h-10 w-10 flex items-center justify-center rounded-lg bg-crush-orange/10 text-crush-orange shrink-0"
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="h-5 w-5"
            >
              <rect x="3" y="3" width="7" height="9" rx="1" />
              <rect x="14" y="3" width="7" height="5" rx="1" />
              <rect x="14" y="12" width="7" height="9" rx="1" />
              <rect x="3" y="16" width="7" height="5" rx="1" />
            </svg>
          </div>
          <div>
            <h4 class="text-sm font-semibold text-white">Looking for the Desktop GUI Dashboard?</h4>
            <p class="text-xs text-crush-textMuted mt-1 leading-relaxed">
              Crush features a stunning, offline-first desktop GUI to monitor native containers,
              view real-time log streams, and auto-diagnose errors using Claude. Standard installer
              bundles are arriving in v1.0; until then, launch natively from source.
            </p>
          </div>
        </div>
        <a
          href="https://github.com/Chidi09/crush#run-gui-from-source"
          target="_blank"
          rel="noopener"
          class="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-crush-surface border border-crush-border/60 hover:border-white hover:bg-crush-surface/80 transition-colors duration-200 shrink-0 select-none text-center"
        >
          View GUI Source
        </a>
      </div>
    </div>
  `,
})
export class InstallBlockComponent {
  private copyService = inject(CopyService);

  methods = METHODS;
  selectedMethod = signal(METHODS[0]!);
  copiedText = signal('Click to copy');
  isCopied = signal(false);

  async copy(): Promise<void> {
    const ok = await this.copyService.copy(this.selectedMethod().command);
    if (ok) {
      this.isCopied.set(true);
      this.copiedText.set('Copied!');
      setTimeout(() => {
        this.isCopied.set(false);
        this.copiedText.set('Click to copy');
      }, 2000);
    } else {
      this.copiedText.set('Failed to copy');
      setTimeout(() => this.copiedText.set('Click to copy'), 2000);
    }
  }
}
