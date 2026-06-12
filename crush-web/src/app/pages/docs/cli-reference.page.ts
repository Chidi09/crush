import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

interface CliCmd {
  name: string;
  desc: string;
  usage: string;
  flags?: string;
}

const COMMANDS: CliCmd[] = [
  {
    name: 'crush',
    desc: 'Detect stack, start deps, build, and run — all in one command. The default workflow.',
    usage: 'crush [--rebuild] [--repack] [--watch] [--memory 1G] [--cpus 0.5] [--platform linux/amd64]',
  },
  {
    name: 'crush detect',
    desc: 'Print what Crush detects about the current project — stack, framework, port, monorepo structure — without building or running.',
    usage: 'crush detect\ncrush detect --json',
  },
  {
    name: 'crush build',
    desc: 'Build a Crush image from the current project directory.',
    usage: 'crush build [--tag myapp:latest] [--platform linux/amd64] [--rebuild] [--repack]',
  },
  {
    name: 'crush run',
    desc: 'Run a previously-built Crush image.',
    usage: 'crush run <image> [--port 3000] [-d] [--restart always] [--memory 512M]',
  },
  {
    name: 'crush eject',
    desc: 'Write a real Dockerfile + docker-compose.yml from the detected configuration. The generated files are marked with # crush:eject so Crush ignores them on future detection runs.',
    usage: 'crush eject [--out ./deploy]',
  },
  { name: 'crush ps', desc: 'List running containers.', usage: 'crush ps [-a]\ncrush ps --format json' },
  {
    name: 'crush logs',
    desc: 'Stream or tail logs for a running container.',
    usage: 'crush logs <container-id> [-f] [--tail 100]',
  },
  {
    name: 'crush stop',
    desc: 'Stop a running container and its full process tree.',
    usage: 'crush stop <container-id>',
  },
  {
    name: 'crush inspect',
    desc: 'Show detailed metadata for a container or image.',
    usage: 'crush inspect <container-id|image-ref>',
  },
  {
    name: 'crush stats',
    desc: 'Live TUI dashboard with real-time CPU and memory sparklines for all running containers.',
    usage: 'crush stats',
  },
  {
    name: 'crush debug',
    desc: 'AI-powered crash diagnosis. Sends the container\'s stack trace to Claude and returns an explanation and suggested fix. Requires ANTHROPIC_API_KEY.',
    usage: 'crush debug <container-id>',
  },
  {
    name: 'crush watch',
    desc: 'Watch mode — hot-restart on source file changes. On Windows, use the --watch flag on the default command instead of this subcommand (the subcommand uses Linux overlayfs).',
    usage: 'crush --watch\n# Windows: crush --watch (flag)\n# Linux:   crush watch',
  },
  {
    name: 'crush images',
    desc: 'List locally-stored Crush images.',
    usage: 'crush images\ncrush images --format json',
  },
  {
    name: 'crush pull',
    desc: 'Pull an image from an OCI registry.',
    usage: 'crush pull <registry>/<repo>:<tag>',
  },
  {
    name: 'crush push',
    desc: 'Push a locally-built image to an OCI registry.',
    usage: 'crush push <registry>/<repo>:<tag>',
  },
  {
    name: 'crush compose',
    desc: 'Run a multi-service application from docker-compose.yml. Crush parses the file and starts native deps (postgres, redis, mysql); full lifecycle management is in progress.',
    usage: 'crush compose up [-d]\ncrush compose down\ncrush compose ps',
  },
  {
    name: 'crush services',
    desc: 'Inspect and control the native dependency processes Crush manages (postgres, garnet, mysql, etc.).',
    usage: 'crush services ps\ncrush services ps --format json\ncrush services ps --all-projects\ncrush services stop <name>',
  },
  {
    name: 'crush history',
    desc: 'View recent build outcomes for the current project.',
    usage: 'crush history\ncrush history --format json',
  },
  {
    name: 'crush migrate',
    desc: 'Convert an existing Dockerfile into a Crushfile. The generated Crushfile will include the detected base image, build steps, and exposed ports.',
    usage: 'crush migrate [--dockerfile ./Dockerfile]',
  },
  {
    name: 'crush secrets',
    desc: 'Manage encrypted project secrets. Secrets are stored with AES-256-GCM and injected into the container environment at runtime.',
    usage: 'crush secrets set <KEY> <value>\ncrush secrets list\ncrush secrets remove <KEY>\ncrush secrets export --to vault',
  },
  {
    name: 'crush scan',
    desc: 'Scan an image for known CVEs and dependency vulnerabilities.',
    usage: 'crush scan <image>',
  },
  {
    name: 'crush sbom',
    desc: 'Generate a Software Bill of Materials for an image.',
    usage: 'crush sbom <image> [--format spdx-json]',
  },
  {
    name: 'crush network',
    desc: 'Manage container networks.',
    usage: 'crush network create <name>\ncrush network ls\ncrush network remove <name>',
  },
  {
    name: 'crush volume',
    desc: 'Manage persistent volumes.',
    usage: 'crush volume create <name>\ncrush volume ls\ncrush volume remove <name>',
  },
  {
    name: 'crush deploy',
    desc: 'Deploy a built image to a remote host.',
    usage: 'crush deploy [--host user@server] [--image myapp:latest]',
  },
  {
    name: 'crush rollback',
    desc: 'Roll back to the previous deployed image on the target host.',
    usage: 'crush rollback [--host user@server]',
  },
  {
    name: 'crush update',
    desc: 'Self-update Crush from the latest GitHub release.',
    usage: 'crush update [--version v0.8.1]',
  },
  {
    name: 'crush system',
    desc: 'System information and maintenance.',
    usage: 'crush system info\ncrush system prune',
  },
  {
    name: 'crush completions',
    desc: 'Generate shell completion scripts.',
    usage: 'crush completions bash\ncrush completions zsh\ncrush completions fish\ncrush completions powershell',
  },
];

@Component({
  selector: 'page-cli-reference',
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
              >Reference</span
            >
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              CLI Reference
            </h1>
            <p class="text-base text-crush-textMuted">
              Complete command line interface handbook with syntax guides.
            </p>
          </div>

          <!-- Global flags callout -->
          <div class="mb-10 rounded-xl border border-crush-border/40 bg-crush-surface/10 p-5">
            <h2 class="text-base font-bold text-white mb-3 select-none">Global flags</h2>
            <p class="text-xs text-crush-textMuted mb-4 leading-relaxed">These flags work on the default <code class="text-crush-orange">crush</code> command and most subcommands. Run <code class="text-crush-orange">crush &lt;cmd&gt; --help</code> for the full set.</p>
            <div class="grid gap-2 sm:grid-cols-2 font-mono text-xs">
              @for (flag of globalFlags; track flag.flag) {
                <div class="flex flex-col gap-0.5 rounded-lg border border-crush-border/30 bg-crush-black/40 px-3 py-2">
                  <span class="text-crush-orangeLight font-semibold">{{ flag.flag }}</span>
                  <span class="text-crush-textMuted">{{ flag.desc }}</span>
                </div>
              }
            </div>
          </div>

          <div class="space-y-6">
            @for (cmd of commands; track cmd.name) {
              <div
                class="group relative overflow-hidden rounded-xl border border-crush-border/40 bg-gradient-to-b from-crush-surface/30 to-crush-surface/10 p-6 hover:border-crush-orange/20 transition-all duration-300"
              >
                <div class="flex items-start justify-between gap-4 mb-3 select-none">
                  <h3
                    class="font-mono text-lg font-bold text-white group-hover:text-crush-orangeLight transition-colors"
                    [innerHTML]="highlightCrush(cmd.name)"
                  ></h3>
                  <span
                    class="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-semibold bg-crush-orange/10 text-crush-orangeLight border border-crush-orange/20 uppercase tracking-wider"
                  >
                    {{ cmd.name.split(' ').length > 1 ? 'subcommand' : 'core' }}
                  </span>
                </div>
                <p class="text-sm text-crush-textMuted mb-4 leading-relaxed">{{ cmd.desc }}</p>

                <div
                  class="rounded-lg border border-crush-border/50 bg-crush-black/50 overflow-hidden"
                >
                  <div
                    class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30 select-none"
                  >
                    <div class="flex items-center gap-2">
                      <span class="w-1.5 h-1.5 rounded-full bg-[#ff5f56]"></span>
                      <span class="w-1.5 h-1.5 rounded-full bg-[#ffbd2e]"></span>
                      <span class="w-1.5 h-1.5 rounded-full bg-[#27c93f]"></span>
                      <span class="text-[9px] text-crush-textMuted font-mono ml-1.5"
                        >Usage Syntax</span
                      >
                    </div>
                    <span
                      class="text-[9px] text-crush-textMuted uppercase tracking-wider font-semibold"
                      >cli</span
                    >
                  </div>
                  <div class="p-3 font-mono text-xs text-crush-text overflow-x-auto whitespace-pre">
                    <code [innerHTML]="highlightCrush(cmd.usage)"></code>
                  </div>
                </div>
              </div>
            }
          </div>

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
              routerLink="/docs/crushfile"
              class="inline-flex items-center gap-2 text-sm text-crush-orange hover:text-crush-orangeLight transition-colors font-bold"
            >
              Crushfile Schema
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
export default class CliReferencePage implements OnInit {
  commands = COMMANDS;

  globalFlags = [
    { flag: '--rebuild', desc: 'Bust the warm-run cache; re-runs install + build steps.' },
    { flag: '--repack', desc: 'Force image re-pack even if source fingerprints match.' },
    { flag: '--watch', desc: 'Hot-restart on source file changes.' },
    { flag: '--memory <limit>', desc: 'Memory cap via Job Object, e.g. 512M or 2G.' },
    { flag: '--cpus <n>', desc: 'CPU limit, e.g. 0.5 for half a core.' },
    { flag: '--priority <level>', desc: 'Process priority: low | normal | high (Windows).' },
    { flag: '--platform <target>', desc: 'Target platform, e.g. linux/amd64 or linux/arm64.' },
    { flag: '--no-proxy', desc: 'Skip the reverse proxy (monorepos with separate backend+frontend).' },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  highlightCrush(text: string): string {
    if (!text) return '';
    // Highlight ONLY the keyword crush (word boundaries) in orange
    return text
      .replace(/\bcrush\b/g, '<span class="text-crush-orange font-bold">crush</span>')
      .replace(/\bCrush\b/g, '<span class="text-crush-orange font-bold">Crush</span>');
  }

  ngOnInit(): void {
    this.title.setTitle('CLI Reference — Crush');
    this.meta.updateTag({
      name: 'description',
      content: 'Complete Crush CLI reference — all commands, flags, and usage examples.',
    });
  }
}
