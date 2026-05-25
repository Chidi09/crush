import { Component, signal, HostListener } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { CommonModule } from '@angular/common';
import { HlmButtonDirective } from '@spartan-ng/ui-button-helm';
import { HlmIconComponent } from '@spartan-ng/ui-icon-helm';

interface SearchItem {
  title: string;
  description: string;
  category: string;
  route: string;
  keywords: string[];
}

const SEARCH_ITEMS: SearchItem[] = [
  {
    title: 'Getting Started Guide',
    description: 'Learn how to install and boot Crush in under a minute.',
    category: 'Getting Started',
    route: '/docs/getting-started',
    keywords: ['install', 'boot', 'curl', 'powershell', 'quickstart', 'getting started']
  },
  {
    title: 'Windows Kernel Job Objects',
    description: 'Deep dive into Windows native isolation and HCS/HNS APIs.',
    category: 'Architecture Guides',
    route: '/docs/windows',
    keywords: ['windows', 'job objects', 'hcs', 'hns', 'kernel', 'nt', 'containment']
  },
  {
    title: 'CLI Reference & Commands',
    description: 'Complete list of all crush commands, flags, and options.',
    category: 'Reference Docs',
    route: '/docs/cli-reference',
    keywords: ['cli', 'commands', 'crush build', 'crush run', 'crush ps', 'subcommand', 'help']
  },
  {
    title: 'The Crushfile Specification',
    description: 'Details on the optional zero-config TOML configuration schema.',
    category: 'Reference Docs',
    route: '/docs/crushfile',
    keywords: ['crushfile', 'toml', 'schema', 'config', 'resources', 'port', 'env']
  },
  {
    title: 'Migrating from Dockerfile',
    description: 'Learn how crush compatively loads and parses existing docker environments.',
    category: 'Architecture Guides',
    route: '/docs/docker-migration',
    keywords: ['docker', 'migration', 'dockerfile', 'docker-compose', 'compose', 'migrate']
  },
  {
    title: 'Built-in Vulnerability Scanner',
    description: 'Scanning containers, modules, and secrets for secure execution.',
    category: 'Security & Compliance',
    route: '/docs/security',
    keywords: ['security', 'scan', 'secrets', 'vulnerability', 'sbom', 'spdx', 'compliance']
  },
  {
    title: 'AnalogJS Installation',
    description: 'Setup information for the documentation web framework.',
    category: 'Developer Settings',
    route: '/docs/installation',
    keywords: ['installation', 'analog', 'client', 'angular', 'spartan', 'dist']
  },
  {
    title: 'SaaS Platform Deployment',
    description: 'Deploying optimized OCI images to AWS, GC, DigitalOcean, or Hetzner.',
    category: 'Architecture Guides',
    route: '/docs/getting-started',
    keywords: ['deploy', 'aws', 'gcp', 'digitalocean', 'hetzner', 'vps', 'oci']
  }
];

@Component({
  selector: 'app-nav',
  standalone: true,
  imports: [RouterLink, RouterLinkActive, CommonModule, HlmButtonDirective, HlmIconComponent],
  styles: [
    `
      :host {
        display: block;
        position: sticky;
        top: 0;
        z-index: 50;
      }
    `,
  ],
  template: `
    <nav
      class="w-full border-b border-crush-border/30 bg-crush-dark/70 backdrop-blur-md transition-all duration-300 font-sans"
    >
      <div class="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6 lg:px-8">
        <!-- Left Section: Logo and Desktop Menu -->
        <div class="flex items-center gap-6 md:gap-8">
          <!-- Logo Brand -->
          <a
            routerLink="/"
            class="flex items-center group transition-transform duration-200 active:scale-95 select-none"
          >
            <img
              src="/logo.png"
              alt=""
              class="h-14 w-auto transition-all duration-300"
              style="mix-blend-mode: screen; filter: hue-rotate(-18deg) saturate(0.88) drop-shadow(0 0 8px rgba(224,85,64,0.3));"
            />
            <span
              class="text-[17px] font-semibold text-crush-orange leading-none tracking-tight -ml-3"
              style="font-family: 'Geist', sans-serif;"
              >Crush</span
            >
          </a>

          <!-- Desktop Links (Premium style) -->
          <div class="hidden lg:flex items-center gap-1 select-none">
            <a
              routerLink="/docs"
              routerLinkActive="bg-crush-surface/50 text-white font-semibold"
              [routerLinkActiveOptions]="{ exact: true }"
              class="px-3.5 py-1.5 rounded-lg text-xs font-semibold tracking-wider text-white hover:bg-crush-surface/30 transition-all duration-200"
            >
              Docs
            </a>
            <a
              routerLink="/docs/installation"
              routerLinkActive="bg-crush-surface/50 text-white font-semibold"
              class="px-3.5 py-1.5 rounded-lg text-xs font-semibold tracking-wider text-white hover:bg-crush-surface/30 transition-all duration-200"
            >
              Install
            </a>
            <a
              routerLink="/blog"
              routerLinkActive="bg-crush-surface/50 text-white font-semibold"
              class="px-3.5 py-1.5 rounded-lg text-xs font-semibold tracking-wider text-white hover:bg-crush-surface/30 transition-all duration-200"
            >
              Blog
            </a>
          </div>
        </div>

        <!-- Right Section: Interactive Actions (SpartanUI style) -->
        <div class="flex items-center gap-3 ml-auto">
          <!-- Premium Search Dialog Button Wrapper -->
          <div class="relative">
            <button
              (click)="searchOpen.set(!searchOpen()); searchQuery.set('')"
              class="hidden md:flex items-center justify-between gap-8 pl-3.5 pr-2.5 py-1.5 rounded-lg border border-crush-border/40 bg-crush-dark/40 hover:border-crush-orange/40 hover:bg-crush-surface/30 text-white transition-all duration-300 relative h-9 w-40 lg:w-56 select-none outline-none z-50"
            >
              <span class="text-xs font-medium tracking-wide">Search docs...</span>
              <div
                class="flex items-center gap-0.5 px-1.5 py-0.5 rounded border border-crush-border/60 bg-crush-surface/50 text-[10px] font-mono select-none"
              >
                <span>Ctrl</span>
                <span class="text-[9px] font-sans">K</span>
              </div>
            </button>

            <!-- Search Dropdown Box (Appearing right below the button) -->
            @if (searchOpen()) {
              <!-- Invisible backdrop to catch clicks outside the dropdown -->
              <div 
                class="fixed inset-0 z-40 bg-transparent cursor-default"
                (click)="searchOpen.set(false)"
              ></div>

              <div 
                class="absolute right-0 top-full mt-2 w-72 lg:w-80 rounded-2xl border border-crush-border/40 bg-[#0a0a0f]/95 shadow-2xl p-2 flex flex-col max-h-[70vh] overflow-hidden select-none border-t-crush-border/60 z-50 animate-fade-slide-up"
                (click)="$event.stopPropagation()"
              >
                <!-- Search Header -->
                <div class="flex items-center gap-2 px-2 py-2 border-b border-crush-border/30">
                  <svg viewBox="0 0 24 24" class="h-3.5 w-3.5 fill-none stroke-current stroke-2 text-crush-textMuted"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
                  <input 
                    #searchInput
                    type="text" 
                    class="w-full bg-transparent text-xs text-white placeholder-crush-textMuted outline-none" 
                    placeholder="Search docs..." 
                    [value]="searchQuery()"
                    (input)="searchQuery.set(searchInput.value)"
                    (keydown.escape)="searchOpen.set(false)"
                    autofocus
                  />
                  <button 
                    (click)="searchOpen.set(false)"
                    class="text-[9px] font-mono border border-crush-border/60 bg-crush-surface/50 text-crush-textMuted px-1 py-0.5 rounded hover:text-white"
                  >
                    ESC
                  </button>
                </div>

                <!-- Search Results -->
                <div class="flex-1 overflow-y-auto p-1 space-y-3 scrollbar-thin max-h-[320px]">
                  @if (filteredItems().length === 0) {
                    <div class="py-6 text-center text-[11px] text-crush-textMuted select-none">
                      No results found for "<span class="text-white font-semibold">{{ searchQuery() }}</span>"
                    </div>
                  } @else {
                    @for (group of getGroupedResults(); track group.category) {
                      <div class="space-y-1">
                        <!-- Category label -->
                        <div class="px-2 py-1 text-[8px] font-bold uppercase tracking-widest text-crush-orangeLight font-sans">
                          {{ group.category }}
                        </div>
                        <!-- Items list -->
                        @for (item of group.items; track item.title) {
                          <a
                            [routerLink]="item.route"
                            (click)="searchOpen.set(false)"
                            class="flex items-center gap-2 px-2 py-1.5 rounded-lg border border-transparent bg-transparent hover:border-crush-border/20 hover:bg-crush-surface/20 text-left group transition-all duration-150"
                          >
                            <div class="h-5 w-5 rounded border border-crush-border/40 bg-crush-dark/40 flex items-center justify-center text-crush-textMuted group-hover:text-crush-orange group-hover:border-crush-orange/40 transition-colors">
                              <svg viewBox="0 0 24 24" class="h-3 w-3 fill-none stroke-current stroke-2.5"><polyline points="9 18 15 12 9 6"/></svg>
                            </div>
                            <div class="flex flex-col">
                              <span class="text-[11px] font-semibold text-white group-hover:text-crush-orangeLight transition-colors leading-tight">
                                {{ item.title }}
                              </span>
                              <span class="text-[9px] text-crush-textMuted leading-tight mt-0.5">
                                {{ item.description }}
                              </span>
                            </div>
                          </a>
                        }
                      </div>
                    }
                  }
                </div>
              </div>
            }
          </div>

          <!-- Divider -->
          <span class="hidden md:inline-block w-px h-4 bg-crush-border/40"></span>

          <!-- Twitter / X Icon Link -->
          <a
            href="https://x.com/crushcontainer"
            target="_blank"
            rel="noopener"
            class="hidden sm:inline-flex items-center justify-center h-8 w-8 rounded-lg text-crush-textMuted hover:text-white hover:bg-crush-surface/40 transition-all duration-200"
          >
            <svg viewBox="0 0 24 24" class="h-4 w-4 fill-current">
              <path
                d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"
              />
            </svg>
          </a>

          <!-- Divider -->
          <span class="hidden sm:inline-block w-px h-4 bg-crush-border/40"></span>

          <!-- GitHub Stars Count Capsule -->
          <a
            href="https://github.com/Chidi09/crush"
            target="_blank"
            rel="noopener"
            class="hidden sm:inline-flex items-center gap-2 px-3 py-1.5 rounded-lg border border-crush-border/40 hover:border-crush-orange/40 bg-crush-dark/40 hover:bg-crush-surface/40 transition-all duration-200 group/git"
          >
            <svg
              role="img"
              viewBox="0 0 24 24"
              class="h-4 w-4 fill-current text-crush-textMuted group-hover/git:text-white transition-colors duration-200"
            >
              <path
                d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"
              />
            </svg>
            <span
              class="text-xs font-semibold text-crush-textMuted group-hover/git:text-white transition-colors duration-200"
              >1.8k</span
            >
          </a>

          <!-- Divider -->
          <span class="hidden sm:inline-block w-px h-4 bg-crush-border/40"></span>

          <!-- Theme Toggle -->
          <button hlmBtn variant="ghost" size="sm" class="hidden sm:inline-flex">
            <hlm-icon name="lucideContrast" size="19px" />
            <span class="sr-only">Toggle theme</span>
          </button>

          <!-- Get Started Call to Action -->
          <a
            routerLink="/docs/getting-started"
            class="inline-flex items-center justify-center px-4 py-1.5 rounded-lg text-xs font-bold text-white bg-crush-orange hover:bg-crush-orangeLight shadow-lg shadow-crush-orange/15 transition-all duration-200 select-none outline-none"
          >
            Get Started
          </a>

          <!-- Mobile Menu Trigger -->
          <button
            class="lg:hidden flex items-center justify-center p-2 rounded-lg border border-crush-border/40 bg-crush-dark/40 text-crush-textMuted hover:text-white hover:border-crush-orange/50 transition-all duration-200 outline-none"
            (click)="mobileOpen.set(!mobileOpen())"
          >
            @if (!mobileOpen()) {
              <svg
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-none stroke-current stroke-2.5 animate-fade-in"
              >
                <line x1="3" y1="12" x2="21" y2="12" />
                <line x1="3" y1="6" x2="21" y2="6" />
                <line x1="3" y1="18" x2="21" y2="18" />
              </svg>
            } @else {
              <svg
                viewBox="0 0 24 24"
                class="h-5 w-5 fill-none stroke-current stroke-2.5 animate-fade-in"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            }
          </button>
        </div>
      </div>

      <!-- Mobile Dropdown Menu -->
      @if (mobileOpen()) {
        <div
          class="lg:hidden border-t border-crush-border/30 bg-crush-dark/95 backdrop-blur-lg px-6 py-6 space-y-4 animate-fade-slide-up shadow-2xl"
        >
          <a
            routerLink="/docs"
            class="block text-sm font-semibold uppercase tracking-wider text-crush-textMuted hover:text-white py-2"
            (click)="mobileOpen.set(false)"
          >
            Docs
          </a>
          <a
            routerLink="/docs/installation"
            class="block text-sm font-semibold uppercase tracking-wider text-crush-textMuted hover:text-white py-2"
            (click)="mobileOpen.set(false)"
          >
            Install
          </a>
          <a
            routerLink="/blog"
            class="block text-sm font-semibold uppercase tracking-wider text-crush-textMuted hover:text-white py-2"
            (click)="mobileOpen.set(false)"
          >
            Blog
          </a>
          <a
            href="https://github.com/Chidi09/crush"
            target="_blank"
            class="block text-sm font-semibold uppercase tracking-wider text-crush-textMuted hover:text-white py-2 inline-flex items-center gap-2"
            (click)="mobileOpen.set(false)"
          >
            <svg role="img" viewBox="0 0 24 24" class="h-4 w-4 fill-current">
              <path
                d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"
              />
            </svg>
            GitHub
          </a>
          <a
            routerLink="/docs/getting-started"
            class="block w-full text-center px-4 py-2.5 rounded-lg text-sm font-bold text-white bg-crush-orange hover:bg-crush-orangeLight transition-colors duration-200 select-none outline-none"
            (click)="mobileOpen.set(false)"
          >
            Get Started
          </a>
        </div>
      }
    </nav>
  `,
})
export class NavComponent {
  mobileOpen = signal(false);
  searchOpen = signal(false);
  searchQuery = signal('');

  @HostListener('window:keydown.control.k', ['$event'])
  @HostListener('window:keydown.meta.k', ['$event'])
  handleKeyboardEvent(event: KeyboardEvent) {
    event.preventDefault();
    this.searchOpen.set(!this.searchOpen());
    if (this.searchOpen()) {
      this.searchQuery.set('');
    }
  }

  filteredItems = (): SearchItem[] => {
    const query = this.searchQuery().toLowerCase().trim();
    if (!query) {
      return SEARCH_ITEMS;
    }
    return SEARCH_ITEMS.filter(item => 
      item.title.toLowerCase().includes(query) ||
      item.description.toLowerCase().includes(query) ||
      item.category.toLowerCase().includes(query) ||
      item.keywords.some(kw => kw.includes(query))
    );
  };

  getGroupedResults() {
    const items = this.filteredItems();
    const groups: { category: string; items: SearchItem[] }[] = [];
    
    items.forEach(item => {
      let group = groups.find(g => g.category === item.category);
      if (!group) {
        group = { category: item.category, items: [] };
        groups.push(group);
      }
      group.items.push(item);
    });
    
    return groups;
  }
}
