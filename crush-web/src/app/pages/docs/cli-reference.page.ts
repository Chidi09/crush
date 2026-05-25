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
    desc: 'Auto-detect stack, build, and run in one command.',
    usage: 'crush [--platform linux/amd64]',
  },
  {
    name: 'crush build',
    desc: 'Build a Crush image from the current project.',
    usage: 'crush build [--platform linux/amd64] [--tag myapp:latest]',
  },
  {
    name: 'crush run',
    desc: 'Run a Crush image.',
    usage: 'crush run <image> [--port 3000] [-d] [--restart always]',
  },
  { name: 'crush ps', desc: 'List running containers.', usage: 'crush ps [-a]' },
  { name: 'crush logs', desc: 'View container logs.', usage: 'crush logs <container-id> [-f]' },
  {
    name: 'crush debug',
    desc: 'Debug a container — attach a shell.',
    usage: 'crush debug <container-id>',
  },
  { name: 'crush watch', desc: 'Watch mode — auto-rebuild on file changes.', usage: 'crush watch' },
  {
    name: 'crush compose',
    desc: 'Run multi-container applications from docker-compose.yml.',
    usage: 'crush compose up [-d]',
  },
  {
    name: 'crush migrate',
    desc: 'Migrate a Dockerfile to Crushfile.',
    usage: 'crush migrate [--dockerfile ./Dockerfile]',
  },
  {
    name: 'crush push',
    desc: 'Push an image to an OCI registry.',
    usage: 'crush push <registry>/<repo>:<tag>',
  },
  {
    name: 'crush pull',
    desc: 'Pull an image from an OCI registry.',
    usage: 'crush pull <registry>/<repo>:<tag>',
  },
  {
    name: 'crush secrets',
    desc: 'Manage encrypted secrets.',
    usage: 'crush secrets set <key> <value>\ncrush secrets list\ncrush secrets export --to vault',
  },
  {
    name: 'crush network',
    desc: 'Manage container networks.',
    usage: 'crush network create <name>\ncrush network ls',
  },
  {
    name: 'crush volume',
    desc: 'Manage persistent volumes.',
    usage: 'crush volume create <name>\ncrush volume ls',
  },
  { name: 'crush scan', desc: 'Scan an image for vulnerabilities.', usage: 'crush scan <image>' },
  {
    name: 'crush sbom',
    desc: 'Generate an SBOM for an image.',
    usage: 'crush sbom <image> [--format spdx-json]',
  },
  {
    name: 'crush system',
    desc: 'System information and diagnostics.',
    usage: 'crush system info\ncrush system prune',
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
