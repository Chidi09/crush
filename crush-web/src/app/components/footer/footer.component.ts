import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { HlmIconComponent } from '@spartan-ng/ui-icon-helm';

@Component({
  selector: 'app-footer',
  standalone: true,
  imports: [RouterLink, HlmIconComponent],
  template: `
    <footer class="border-t border-crush-border/50 bg-crush-black">
      <div class="mx-auto max-w-7xl px-4 py-12 sm:px-6 lg:px-8">
        <div class="grid grid-cols-2 gap-8 md:grid-cols-4">
          <div>
            <h4 class="text-sm font-semibold text-white">Product</h4>
            <ul class="mt-4 space-y-3">
              <li>
                <a
                  routerLink="/docs"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Docs</a
                >
              </li>
              <li>
                <a
                  routerLink="/docs/getting-started"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Getting Started</a
                >
              </li>
              <li>
                <a
                  routerLink="/docs/installation"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Install</a
                >
              </li>
              <li>
                <a
                  routerLink="/changelog"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Changelog</a
                >
              </li>
            </ul>
          </div>
          <div>
            <h4 class="text-sm font-semibold text-white">Documentation</h4>
            <ul class="mt-4 space-y-3">
              <li>
                <a
                  routerLink="/docs/cli-reference"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >CLI Reference</a
                >
              </li>
              <li>
                <a
                  routerLink="/docs/crushfile"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Crushfile</a
                >
              </li>
              <li>
                <a
                  routerLink="/docs/docker-migration"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Docker Migration</a
                >
              </li>
              <li>
                <a
                  routerLink="/docs/windows"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Windows Guide</a
                >
              </li>
              <li>
                <a
                  routerLink="/docs/security"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Security</a
                >
              </li>
            </ul>
          </div>
          <div>
            <h4 class="text-sm font-semibold text-white">Community</h4>
            <ul class="mt-4 space-y-3">
              <li>
                <a
                  href="https://github.com/crushcontainer/crush"
                  target="_blank"
                  rel="noopener"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors inline-flex items-center gap-1"
                >
                  <hlm-icon name="lucideGithub" size="xs" /> GitHub
                </a>
              </li>
              <li>
                <a
                  routerLink="/blog"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Blog</a
                >
              </li>
            </ul>
          </div>
          <div>
            <h4 class="text-sm font-semibold text-white">Legal</h4>
            <ul class="mt-4 space-y-3">
              <li>
                <a
                  href="https://github.com/crushcontainer/crush/blob/main/LICENSE"
                  target="_blank"
                  rel="noopener"
                  class="text-sm text-crush-textMuted hover:text-crush-orange transition-colors"
                  >Apache-2.0</a
                >
              </li>
            </ul>
          </div>
        </div>
        <div class="mt-12 border-t border-crush-border/50 pt-8 flex items-center justify-between">
          <p class="text-sm text-crush-muted">&copy; {{ year }} Crush. Apache-2.0 License.</p>
          <div class="flex items-center gap-4 text-sm text-crush-muted">
            <span>Made for developers who ship on Windows</span>
          </div>
        </div>
      </div>
    </footer>
  `,
})
export class FooterComponent {
  year = new Date().getFullYear();
}
