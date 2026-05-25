import { Component, signal } from '@angular/core';
import { CommonModule } from '@angular/common';

interface ComparisonRow {
  feature: string;
  crush: string;
  docker: string;
  winner: 'crush' | 'docker' | 'tie';
  status: 'done' | 'wip';
  type: string;
}

const PERFORMANCE_ROWS: ComparisonRow[] = [
  {
    feature: 'Startup latency',
    crush: '0.28s',
    docker: '15.0s',
    winner: 'crush',
    status: 'done',
    type: 'latency',
  },
  {
    feature: 'Memory footprint (idle)',
    crush: '25 MB',
    docker: '2.0 GB',
    winner: 'crush',
    status: 'done',
    type: 'ram',
  },
  {
    feature: 'First-run bootstrap',
    crush: '3 seconds',
    docker: '5+ minutes',
    winner: 'crush',
    status: 'done',
    type: 'setup',
  },
  {
    feature: 'Process architecture',
    crush: 'Native NT Jobs',
    docker: 'WSL2 Hypervisor',
    winner: 'crush',
    status: 'done',
    type: 'kernel',
  },
];

const FEATURE_ROWS: ComparisonRow[] = [
  {
    feature: 'Hot-reload filesystem sync',
    crush: 'Virtio-FS direct',
    docker: '9P translation',
    winner: 'crush',
    status: 'done',
    type: 'sync',
  },
  {
    feature: 'Cross-platform Linux builds',
    crush: 'Firecracker VM',
    docker: 'WSL2 VM overhead',
    winner: 'crush',
    status: 'done',
    type: 'compile',
  },
  {
    feature: 'Docker-compose compatibility',
    crush: 'Full support',
    docker: 'Native daemon',
    winner: 'crush',
    status: 'done',
    type: 'compose',
  },
  {
    feature: 'Distribution binary size',
    crush: '~15 MB',
    docker: '~2.0 GB',
    winner: 'crush',
    status: 'done',
    type: 'binary',
  },
];

const SECURITY_ROWS: ComparisonRow[] = [
  {
    feature: 'Secret scanning & filters',
    crush: 'Automatic built-in',
    docker: 'Manual scan',
    winner: 'crush',
    status: 'done',
    type: 'secrets',
  },
  {
    feature: 'Cryptographic release check',
    crush: 'Cosign signed',
    docker: 'None',
    winner: 'crush',
    status: 'done',
    type: 'signature',
  },
  {
    feature: 'Vulnerability scanners',
    crush: 'Crush scan CLI',
    docker: 'Third-party plug',
    winner: 'crush',
    status: 'done',
    type: 'scanner',
  },
  {
    feature: 'Software Bill of Materials',
    crush: 'SPDX-JSON exports',
    docker: 'Experimental',
    winner: 'crush',
    status: 'done',
    type: 'sbom',
  },
];

@Component({
  selector: 'app-comparison-table',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="w-full space-y-6 font-sans">
      <!-- Premium Header Section (SpartanUI style) -->
      <div
        class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 border-b border-crush-border/30 pb-4 select-none"
      >
        <div>
          <h4 class="text-sm font-semibold text-white tracking-wide">Crush vs. Docker Desktop</h4>
          <p class="text-xs text-crush-textMuted mt-0.5">
            A comprehensive unified comparison of performance, developer features, and security.
          </p>
        </div>
        <div class="flex items-center gap-2">
          <span class="text-xs text-crush-textMuted font-medium">Scope:</span>
          <span
            class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-bold bg-crush-orange/10 text-crush-orangeLight border border-crush-orange/20 uppercase tracking-wider"
          >
            Unified Audit
          </span>
        </div>
      </div>

      <!-- High-Fidelity Table Container -->
      <div
        class="overflow-x-auto rounded-xl border border-crush-border/30 bg-[#0c0c10]/40 backdrop-blur-md shadow-2xl shadow-crush-orange/5"
      >
        <table class="w-full border-collapse text-sm text-left">
          <!-- Table Header -->
          <thead>
            <tr
              class="border-b border-crush-border/40 bg-crush-dark/50 select-none text-xs font-semibold uppercase tracking-wider text-crush-textMuted"
            >
              <th class="px-6 py-4 w-12 text-center">
                <!-- Custom checkbox header design -->
                <div class="flex items-center justify-center">
                  <span
                    class="h-3.5 w-3.5 rounded border border-crush-border/80 bg-crush-black/50 block relative"
                  >
                    <span
                      class="absolute inset-0.5 bg-crush-orange rounded-[1px] opacity-25"
                    ></span>
                  </span>
                </div>
              </th>
              <th class="px-6 py-4">Feature</th>
              <th class="px-6 py-4">Metric/Type</th>
              <th class="px-6 py-4">
                <div class="flex items-center gap-2">
                  <span
                    class="h-2 w-2 rounded-full bg-crush-orange shadow-[0_0_8px_rgba(224,85,64,0.6)] animate-pulse-glow"
                  ></span>
                  <span class="text-sm font-bold text-white tracking-wide normal-case">Crush</span>
                </div>
              </th>
              <th class="px-6 py-4">
                <div class="flex items-center gap-2">
                  <span
                    class="h-2 w-2 rounded-full bg-docker-blue shadow-[0_0_8px_rgba(13,183,237,0.6)]"
                  ></span>
                  <span class="text-sm font-bold text-white tracking-wide normal-case"
                    >Docker Desktop</span
                  >
                </div>
              </th>
              <th class="px-6 py-4 text-center">Status</th>
            </tr>
          </thead>

          <!-- Table Body -->
          <tbody class="divide-y divide-crush-border/20">
            <!-- PERFORMANCE SECTION -->
            <tr
              class="bg-crush-dark/60 text-[10px] font-bold uppercase tracking-widest text-crush-orangeLight select-none border-t border-b border-crush-border/30"
            >
              <td
                colspan="6"
                class="px-6 py-2.5 bg-gradient-to-r from-crush-orange/5 via-transparent to-transparent"
              >
                Performance Metrics
              </td>
            </tr>
            @for (row of performanceRows; track row.feature) {
              <tr class="hover:bg-crush-surface/30 transition-all duration-200 group">
                <!-- Checkbox Row Selector -->
                <td class="px-6 py-4 text-center">
                  <div class="flex items-center justify-center select-none">
                    <span
                      class="h-3.5 w-3.5 rounded border border-crush-border/60 bg-crush-black/40 block hover:border-crush-orange/40 transition-colors"
                    ></span>
                  </div>
                </td>

                <!-- Feature -->
                <td
                  class="px-6 py-4 font-semibold text-crush-text group-hover:text-white transition-colors"
                >
                  {{ row.feature }}
                </td>

                <!-- Type Badge -->
                <td class="px-6 py-4 select-none">
                  <span
                    class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold bg-crush-surface/80 text-crush-textMuted border border-crush-border/30 uppercase tracking-wider font-mono"
                  >
                    {{ row.type }}
                  </span>
                </td>

                <!-- Crush Spec (Green highlight if winner) -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'crush') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.crush }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-crush-surface/40 text-crush-textMuted border border-crush-border/30"
                    >
                      {{ row.crush }}
                    </span>
                  }
                </td>

                <!-- Docker Desktop Spec -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'docker') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-blue-500/10 text-blue-400 border border-blue-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-crush-surface/40 text-crush-textMuted border border-crush-border/30 font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  }
                </td>

                <!-- Status column (SpartanUI styled loader/checkmark check) -->
                <td class="px-6 py-4 text-center select-none w-28">
                  @if (row.status === 'done') {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                    >
                      <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Done
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20"
                    >
                      <svg
                        viewBox="0 0 24 24"
                        class="h-3 w-3 fill-none stroke-current stroke-2.5 animate-spin"
                      >
                        <line x1="12" y1="2" x2="12" y2="6" />
                        <line x1="12" y1="18" x2="12" y2="22" />
                        <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" />
                        <line x1="16.24" y1="16.24" x2="19.07" y2="19.07" />
                        <line x1="2" y1="12" x2="6" y2="12" />
                        <line x1="18" y1="12" x2="22" y2="12" />
                        <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" />
                        <line x1="16.24" y1="7.76" x2="19.07" y2="4.93" />
                      </svg>
                      WIP
                    </span>
                  }
                </td>
              </tr>
            }

            <!-- FEATURE SECTION -->
            <tr
              class="bg-crush-dark/60 text-[10px] font-bold uppercase tracking-widest text-crush-orangeLight select-none border-t border-b border-crush-border/30"
            >
              <td
                colspan="6"
                class="px-6 py-2.5 bg-gradient-to-r from-crush-orange/5 via-transparent to-transparent"
              >
                Developer Features
              </td>
            </tr>
            @for (row of featureRows; track row.feature) {
              <tr class="hover:bg-crush-surface/30 transition-all duration-200 group">
                <!-- Checkbox Row Selector -->
                <td class="px-6 py-4 text-center">
                  <div class="flex items-center justify-center select-none">
                    <span
                      class="h-3.5 w-3.5 rounded border border-crush-border/60 bg-crush-black/40 block hover:border-crush-orange/40 transition-colors"
                    ></span>
                  </div>
                </td>

                <!-- Feature -->
                <td
                  class="px-6 py-4 font-semibold text-crush-text group-hover:text-white transition-colors"
                >
                  {{ row.feature }}
                </td>

                <!-- Type Badge -->
                <td class="px-6 py-4 select-none">
                  <span
                    class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold bg-crush-surface/80 text-crush-textMuted border border-crush-border/30 uppercase tracking-wider font-mono"
                  >
                    {{ row.type }}
                  </span>
                </td>

                <!-- Crush Spec (Green highlight if winner) -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'crush') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.crush }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-crush-surface/40 text-crush-textMuted border border-crush-border/30"
                    >
                      {{ row.crush }}
                    </span>
                  }
                </td>

                <!-- Docker Desktop Spec -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'docker') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-blue-500/10 text-blue-400 border border-blue-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-crush-surface/40 text-crush-textMuted border border-crush-border/30 font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  }
                </td>

                <!-- Status column (SpartanUI styled loader/checkmark check) -->
                <td class="px-6 py-4 text-center select-none w-28">
                  @if (row.status === 'done') {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                    >
                      <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Done
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20"
                    >
                      <svg
                        viewBox="0 0 24 24"
                        class="h-3 w-3 fill-none stroke-current stroke-2.5 animate-spin"
                      >
                        <line x1="12" y1="2" x2="12" y2="6" />
                        <line x1="12" y1="18" x2="12" y2="22" />
                        <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" />
                        <line x1="16.24" y1="16.24" x2="19.07" y2="19.07" />
                        <line x1="2" y1="12" x2="6" y2="12" />
                        <line x1="18" y1="12" x2="22" y2="12" />
                        <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" />
                        <line x1="16.24" y1="7.76" x2="19.07" y2="4.93" />
                      </svg>
                      WIP
                    </span>
                  }
                </td>
              </tr>
            }

            <!-- SECURITY SECTION -->
            <tr
              class="bg-crush-dark/60 text-[10px] font-bold uppercase tracking-widest text-crush-orangeLight select-none border-t border-b border-crush-border/30"
            >
              <td
                colspan="6"
                class="px-6 py-2.5 bg-gradient-to-r from-crush-orange/5 via-transparent to-transparent"
              >
                Security & Supply Chain
              </td>
            </tr>
            @for (row of securityRows; track row.feature) {
              <tr class="hover:bg-crush-surface/30 transition-all duration-200 group">
                <!-- Checkbox Row Selector -->
                <td class="px-6 py-4 text-center">
                  <div class="flex items-center justify-center select-none">
                    <span
                      class="h-3.5 w-3.5 rounded border border-crush-border/60 bg-crush-black/40 block hover:border-crush-orange/40 transition-colors"
                    ></span>
                  </div>
                </td>

                <!-- Feature -->
                <td
                  class="px-6 py-4 font-semibold text-crush-text group-hover:text-white transition-colors"
                >
                  {{ row.feature }}
                </td>

                <!-- Type Badge -->
                <td class="px-6 py-4 select-none">
                  <span
                    class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold bg-crush-surface/80 text-crush-textMuted border border-crush-border/30 uppercase tracking-wider font-mono"
                  >
                    {{ row.type }}
                  </span>
                </td>

                <!-- Crush Spec (Green highlight if winner) -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'crush') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.crush }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-crush-surface/40 text-crush-textMuted border border-crush-border/30"
                    >
                      {{ row.crush }}
                    </span>
                  }
                </td>

                <!-- Docker Desktop Spec -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'docker') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-blue-500/10 text-blue-400 border border-blue-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-crush-surface/40 text-crush-textMuted border border-crush-border/30 font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  }
                </td>

                <!-- Status column (SpartanUI styled loader/checkmark check) -->
                <td class="px-6 py-4 text-center select-none w-28">
                  @if (row.status === 'done') {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                    >
                      <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Done
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20"
                    >
                      <svg
                        viewBox="0 0 24 24"
                        class="h-3 w-3 fill-none stroke-current stroke-2.5 animate-spin"
                      >
                        <line x1="12" y1="2" x2="12" y2="6" />
                        <line x1="12" y1="18" x2="12" y2="22" />
                        <line x1="4.93" y1="4.93" x2="7.76" y2="7.76" />
                        <line x1="16.24" y1="16.24" x2="19.07" y2="19.07" />
                        <line x1="2" y1="12" x2="6" y2="12" />
                        <line x1="18" y1="12" x2="22" y2="12" />
                        <line x1="4.93" y1="19.07" x2="7.76" y2="16.24" />
                        <line x1="16.24" y1="7.76" x2="19.07" y2="4.93" />
                      </svg>
                      WIP
                    </span>
                  }
                </td>
              </tr>
            }
          </tbody>
        </table>
      </div>
    </div>
  `,
})
export class ComparisonTableComponent {
  performanceRows = PERFORMANCE_ROWS;
  featureRows = FEATURE_ROWS;
  securityRows = SECURITY_ROWS;
}
