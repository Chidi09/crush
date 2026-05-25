import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmBadgeDirective } from '../ui/badge';

@Component({
  selector: 'page-changelog',
  standalone: true,
  imports: [RouterLink, HlmBadgeDirective],
  template: `
    <div class="mx-auto max-w-3xl px-4 py-12 sm:px-6 lg:px-8">
      <h1 class="text-3xl font-bold text-white mb-2">Changelog</h1>
      <p class="text-lg text-crush-textMuted mb-12">Version history.</p>

      @for (release of releases; track release.version) {
        <div class="border-l-2 border-crush-border/50 pl-6 pb-12 relative">
          <div
            class="absolute left-[-9px] top-0 h-4 w-4 rounded-full border-2 border-crush-orange bg-crush-black"
          ></div>
          <div class="flex items-center gap-3 mb-2">
            <h2 class="text-xl font-bold text-white font-mono">{{ release.version }}</h2>
            <span
              hlmBadge
              variant="outline"
              class="border-crush-orange/30 bg-crush-orange/10 text-crush-orange hover:bg-crush-orange/20"
              >{{ release.date }}</span
            >
          </div>
          <ul class="space-y-2">
            @for (item of release.items; track item) {
              <li class="text-sm text-crush-textMuted flex items-start gap-2">
                <span class="text-crush-orange mt-0.5 shrink-0">-</span>
                <span>{{ item }}</span>
              </li>
            }
          </ul>
        </div>
      }
    </div>
  `,
})
export default class ChangelogPage implements OnInit {
  releases = [
    {
      version: 'v0.1.0-beta',
      date: '2025-05-25',
      items: [
        'Initial beta release',
        'Windows native container runtime using Job Objects',
        'crush build — auto-detect stack and build OCI images',
        'crush run — run containers with sub-second starts',
        'crush watch — dev mode with auto-rebuild on file changes',
        'crush migrate — Dockerfile to Crushfile conversion',
        'crush compose — docker-compose.yml support',
        'crush secrets — encrypted secret store with AES-256-GCM',
        'crush scan — vulnerability scanning for images',
        'crush sbom — SPDX JSON SBOM generation',
        'DOCKER_HOST socket compatibility layer',
        'Firecracker microVM for Linux builds on Windows',
        'Cross-platform: Windows, macOS, Linux',
      ],
    },
  ];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Changelog — Crush');
    this.meta.updateTag({
      name: 'description',
      content: 'Crush version history and release notes.',
    });
  }
}
