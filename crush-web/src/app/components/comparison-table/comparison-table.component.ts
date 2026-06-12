import { Component } from '@angular/core';
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
    feature: 'Warm-run overhead (dev iteration)',
    crush: '~2s',
    docker: '15s + image pull',
    winner: 'crush',
    status: 'done',
    type: 'latency',
  },
  {
    feature: 'Memory footprint (idle)',
    crush: '~30 MB (no daemon)',
    docker: '~2 GB (Hyper-V VM)',
    winner: 'crush',
    status: 'done',
    type: 'ram',
  },
  {
    feature: 'Process model',
    crush: 'Native app + Job Object',
    docker: 'WSL2 VM + Linux ns',
    winner: 'crush',
    status: 'done',
    type: 'kernel',
  },
  {
    feature: 'Ctrl+C kills full tree',
    crush: 'Yes (Job Object)',
    docker: 'Container stop',
    winner: 'tie',
    status: 'done',
    type: 'cleanup',
  },
];

const FEATURE_ROWS: ComparisonRow[] = [
  {
    feature: 'Auto-detect project stack',
    crush: 'Built in',
    docker: 'Manual Dockerfile',
    winner: 'crush',
    status: 'done',
    type: 'detect',
  },
  {
    feature: 'Postgres dep on-demand',
    crush: 'Native + auto-creds',
    docker: 'docker-compose up',
    winner: 'crush',
    status: 'done',
    type: 'deps',
  },
  {
    feature: 'pgvector on Windows',
    crush: 'Builds via MSVC',
    docker: 'Container image',
    winner: 'tie',
    status: 'done',
    type: 'extension',
  },
  {
    feature: 'Eject to Dockerfile + compose',
    crush: 'crush eject',
    docker: 'N/A (canonical)',
    winner: 'tie',
    status: 'done',
    type: 'prod',
  },
  {
    feature: 'Distribution binary size',
    crush: '~11 MB single .exe',
    docker: '~600 MB installer',
    winner: 'crush',
    status: 'done',
    type: 'binary',
  },
];

const SECURITY_ROWS: ComparisonRow[] = [
  {
    feature: 'Secret scanning',
    crush: 'Built-in, pre-execution (10+ patterns)',
    docker: 'Scout plugin (opt-in)',
    winner: 'crush',
    status: 'done',
    type: 'secrets',
  },
  {
    feature: 'AI crash diagnosis',
    crush: 'crush debug (Anthropic Claude)',
    docker: 'None',
    winner: 'crush',
    status: 'done',
    type: 'debug',
  },
  {
    feature: 'Vulnerability scanner',
    crush: 'crush scan (ships, growing CVE db)',
    docker: 'Docker Scout',
    winner: 'tie',
    status: 'done',
    type: 'scanner',
  },
  {
    feature: 'SBOM (SPDX-JSON)',
    crush: 'crush sbom --format spdx-json',
    docker: 'docker sbom',
    winner: 'tie',
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
        class="overflow-x-auto rounded-xl border border-border bg-card shadow-2xl shadow-black/[0.03] dark:shadow-crush-orange/5 transition-all duration-300"
      >
        <table class="w-full border-collapse text-sm text-left">
          <!-- Table Header -->
          <thead>
            <tr
              class="border-b border-border bg-muted/40 dark:bg-crush-dark/50 select-none text-xs font-semibold uppercase tracking-wider text-muted-foreground dark:text-crush-textMuted"
            >
              <th class="px-6 py-4 w-12 text-center">
                <!-- Custom checkbox header design -->
                <div class="flex items-center justify-center">
                  <span
                    class="h-3.5 w-3.5 rounded border border-border bg-muted dark:border-crush-border/80 dark:bg-crush-black/50 block relative"
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
                  <span class="text-sm font-bold text-crush-text tracking-wide normal-case"
                    >Crush</span
                  >
                </div>
              </th>
              <th class="px-6 py-4">
                <div class="flex items-center gap-2">
                  <span
                    class="h-2 w-2 rounded-full bg-docker-blue shadow-[0_0_8px_rgba(13,183,237,0.6)]"
                  ></span>
                  <span class="text-sm font-bold text-crush-text tracking-wide normal-case"
                    >Docker Desktop</span
                  >
                </div>
              </th>
              <th class="px-6 py-4 text-center">Status</th>
            </tr>
          </thead>

          <!-- Table Body -->
          <tbody class="divide-y divide-border/30">
            <!-- PERFORMANCE SECTION -->
            <tr
              class="bg-muted/80 dark:bg-crush-dark/60 text-[10px] font-bold uppercase tracking-widest text-crush-orange select-none border-t border-b border-border"
            >
              <td
                colspan="6"
                class="px-6 py-2.5 bg-gradient-to-r from-crush-orange/5 via-transparent to-transparent"
              >
                Performance Metrics
              </td>
            </tr>
            @for (row of performanceRows; track row.feature) {
              <tr
                class="hover:bg-muted/40 dark:hover:bg-crush-surface/30 border-b border-border/30 dark:border-crush-border/20 transition-all duration-200 group"
              >
                <!-- Checkbox Row Selector -->
                <td class="px-6 py-4 text-center">
                  <div class="flex items-center justify-center select-none">
                    <span
                      class="h-3.5 w-3.5 rounded border border-border bg-muted dark:border-crush-border/60 dark:bg-crush-black/40 block hover:border-crush-orange/40 transition-colors"
                    ></span>
                  </div>
                </td>

                <!-- Feature -->
                <td
                  class="px-6 py-4 font-semibold text-crush-text group-hover:text-crush-orange transition-colors"
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
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.crush }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-muted/60 dark:bg-crush-surface/40 text-muted-foreground dark:text-crush-textMuted border border-border dark:border-crush-border/30"
                    >
                      {{ row.crush }}
                    </span>
                  }
                </td>

                <!-- Docker Desktop Spec -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'docker') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-blue-50 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-muted/60 dark:bg-crush-surface/40 text-muted-foreground dark:text-crush-textMuted border border-border dark:border-crush-border/30 font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  }
                </td>

                <!-- Status column (SpartanUI styled loader/checkmark check) -->
                <td class="px-6 py-4 text-center select-none w-28">
                  @if (row.status === 'done') {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20"
                    >
                      <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Done
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-200 dark:border-amber-500/20"
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
              class="bg-muted/80 dark:bg-crush-dark/60 text-[10px] font-bold uppercase tracking-widest text-crush-orange select-none border-t border-b border-border"
            >
              <td
                colspan="6"
                class="px-6 py-2.5 bg-gradient-to-r from-crush-orange/5 via-transparent to-transparent"
              >
                Developer Features
              </td>
            </tr>
            @for (row of featureRows; track row.feature) {
              <tr
                class="hover:bg-muted/40 dark:hover:bg-crush-surface/30 border-b border-border/30 dark:border-crush-border/20 transition-all duration-200 group"
              >
                <!-- Checkbox Row Selector -->
                <td class="px-6 py-4 text-center">
                  <div class="flex items-center justify-center select-none">
                    <span
                      class="h-3.5 w-3.5 rounded border border-border bg-muted dark:border-crush-border/60 dark:bg-crush-black/40 block hover:border-crush-orange/40 transition-colors"
                    ></span>
                  </div>
                </td>

                <!-- Feature -->
                <td
                  class="px-6 py-4 font-semibold text-crush-text group-hover:text-crush-orange transition-colors"
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
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.crush }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-muted/60 dark:bg-crush-surface/40 text-muted-foreground dark:text-crush-textMuted border border-border dark:border-crush-border/30"
                    >
                      {{ row.crush }}
                    </span>
                  }
                </td>

                <!-- Docker Desktop Spec -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'docker') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-blue-50 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-muted/60 dark:bg-crush-surface/40 text-muted-foreground dark:text-crush-textMuted border border-border dark:border-crush-border/30 font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  }
                </td>

                <!-- Status column (SpartanUI styled loader/checkmark check) -->
                <td class="px-6 py-4 text-center select-none w-28">
                  @if (row.status === 'done') {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20"
                    >
                      <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Done
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-200 dark:border-amber-500/20"
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
              class="bg-muted/80 dark:bg-crush-dark/60 text-[10px] font-bold uppercase tracking-widest text-crush-orange select-none border-t border-b border-border"
            >
              <td
                colspan="6"
                class="px-6 py-2.5 bg-gradient-to-r from-crush-orange/5 via-transparent to-transparent"
              >
                Security & Supply Chain
              </td>
            </tr>
            @for (row of securityRows; track row.feature) {
              <tr
                class="hover:bg-muted/40 dark:hover:bg-crush-surface/30 border-b border-border/30 dark:border-crush-border/20 transition-all duration-200 group"
              >
                <!-- Checkbox Row Selector -->
                <td class="px-6 py-4 text-center">
                  <div class="flex items-center justify-center select-none">
                    <span
                      class="h-3.5 w-3.5 rounded border border-border bg-muted dark:border-crush-border/60 dark:bg-crush-black/40 block hover:border-crush-orange/40 transition-colors"
                    ></span>
                  </div>
                </td>

                <!-- Feature -->
                <td
                  class="px-6 py-4 font-semibold text-crush-text group-hover:text-crush-orange transition-colors"
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
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.crush }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-muted/60 dark:bg-crush-surface/40 text-muted-foreground dark:text-crush-textMuted border border-border dark:border-crush-border/30"
                    >
                      {{ row.crush }}
                    </span>
                  }
                </td>

                <!-- Docker Desktop Spec -->
                <td class="px-6 py-4 font-medium">
                  @if (row.winner === 'docker') {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-bold bg-blue-50 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-500/20 shadow-sm animate-fade-in font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center px-2.5 py-1 rounded-lg text-xs font-semibold bg-muted/60 dark:bg-crush-surface/40 text-muted-foreground dark:text-crush-textMuted border border-border dark:border-crush-border/30 font-mono"
                    >
                      {{ row.docker }}
                    </span>
                  }
                </td>

                <!-- Status column (SpartanUI styled loader/checkmark check) -->
                <td class="px-6 py-4 text-center select-none w-28">
                  @if (row.status === 'done') {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-500/20"
                    >
                      <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      Done
                    </span>
                  } @else {
                    <span
                      class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-200 dark:border-amber-500/20"
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
