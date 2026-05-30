import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink, ActivatedRoute } from '@angular/router';
import { Title, Meta } from '@angular/platform-browser';
import { injectContent, MarkdownComponent } from '@analogjs/content';

export interface PostAttributes {
  title: string;
  excerpt: string;
  date: string;
  tag: string;
  author: string;
  authorImage: string;
  readingTime: string;
}

@Component({
  selector: 'page-blog-post',
  standalone: true,
  imports: [CommonModule, RouterLink, MarkdownComponent],
  template: `
    <article class="relative min-h-screen py-16 overflow-hidden">
      <!-- Ambient light gradients background -->
      <div
        class="absolute top-0 left-1/2 -translate-x-1/2 w-[600px] h-[600px] bg-crush-orange/5 blur-[120px] pointer-events-none rounded-full"
      ></div>

      <div class="mx-auto max-w-3xl px-4 sm:px-6 relative">
        <!-- Back Button -->
        <a
          routerLink="/blog"
          class="inline-flex items-center gap-2 text-sm text-crush-textMuted hover:text-white mb-10 transition-colors group select-none cursor-pointer"
        >
          <svg
            viewBox="0 0 24 24"
            class="h-4 w-4 fill-none stroke-current stroke-2.5 transition-transform group-hover:-translate-x-1"
          >
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
          Back to Blog
        </a>

        @if (post$ | async; as post) {
          <!-- Header -->
          <header class="mb-10">
            <div class="flex items-center gap-3 mb-6">
              <span
                class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold border border-crush-orange/30 bg-crush-orange/10 text-crush-orange"
              >
                {{ post.attributes.tag }}
              </span>
              <span class="text-xs text-crush-muted">•</span>
              <span class="text-xs text-crush-textMuted">{{ post.attributes.readingTime }}</span>
            </div>

            <h1
              class="text-3xl font-extrabold text-white tracking-tight sm:text-4xl lg:text-5xl leading-tight mb-8"
            >
              {{ post.attributes.title }}
            </h1>

            <!-- Author & Metadata Block -->
            <div
              class="flex items-center justify-between border-y border-crush-border/30 py-4 flex-wrap gap-4"
            >
              <div class="flex items-center gap-3">
                <div
                  class="h-10 w-10 rounded-full border border-crush-border overflow-hidden bg-crush-surface/50 flex items-center justify-center"
                >
                  @if (post.attributes.authorImage) {
                    <img
                      [src]="post.attributes.authorImage"
                      [alt]="post.attributes.author"
                      class="h-full w-full object-cover"
                    />
                  } @else {
                    <span class="text-sm font-bold text-crush-orange">C</span>
                  }
                </div>
                <div>
                  <div class="text-sm font-semibold text-white">{{ post.attributes.author }}</div>
                  <div class="text-xs text-crush-muted">{{ post.attributes.date }}</div>
                </div>
              </div>

              <!-- Share Buttons -->
              <div class="flex items-center gap-2">
                <button
                  (click)="shareOnTwitter(post.attributes.title)"
                  class="p-2 rounded-lg border border-crush-border/50 bg-crush-surface/30 text-crush-textMuted hover:text-white hover:border-crush-orange/30 transition-colors duration-200"
                  title="Share on Twitter"
                >
                  <svg viewBox="0 0 24 24" class="h-4 w-4 fill-current">
                    <path
                      d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"
                    />
                  </svg>
                </button>
                <button
                  (click)="copyLink()"
                  class="p-2 rounded-lg border border-crush-border/50 bg-crush-surface/30 text-crush-textMuted hover:text-white hover:border-crush-orange/30 transition-colors duration-200 relative flex items-center justify-center"
                  title="Copy link"
                >
                  @if (copied) {
                    <svg viewBox="0 0 24 24" class="h-4 w-4 fill-none stroke-emerald-400 stroke-2">
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                  } @else {
                    <svg viewBox="0 0 24 24" class="h-4 w-4 fill-none stroke-current stroke-2">
                      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                    </svg>
                  }
                </button>
              </div>
            </div>
          </header>

          <!-- Glassmorphism Article Container -->
          <div
            class="prose prose-invert max-w-none rounded-2xl border border-crush-border/50 bg-crush-surface/10 p-6 sm:p-10 shadow-2xl backdrop-blur-sm relative mb-12"
          >
            <analog-markdown
              [content]="post.content"
              class="block markdown-body text-crush-text leading-relaxed font-sans text-base sm:text-lg"
            ></analog-markdown>
          </div>
        }
      </div>
    </article>
  `,
  styles: [
    `
      :host ::ng-deep .markdown-body {
        color: #94a3b8; /* text-slate-400 */
        font-size: 1.0625rem;
        line-height: 1.8;
      }
      :host ::ng-deep .markdown-body h2 {
        color: #ffffff;
        font-weight: 700;
        font-size: 1.5rem;
        margin-top: 2rem;
        margin-bottom: 1rem;
      }
      :host ::ng-deep .markdown-body h3 {
        color: #ffffff;
        font-weight: 700;
        font-size: 1.25rem;
        margin-top: 1.5rem;
        margin-bottom: 0.75rem;
      }
      :host ::ng-deep .markdown-body p {
        margin-bottom: 1.25rem;
      }
      :host ::ng-deep .markdown-body code {
        color: #f97316; /* orange-500 */
        background: rgba(249, 115, 22, 0.1);
        padding: 0.2rem 0.4rem;
        border-radius: 0.25rem;
        font-family: 'JetBrains Mono', monospace;
        font-size: 0.875em;
      }
      :host ::ng-deep .markdown-body pre {
        background: #0f172a; /* slate-900 */
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 0.75rem;
        padding: 1.25rem;
        margin: 1.5rem 0;
        overflow-x: auto;
      }
      :host ::ng-deep .markdown-body pre code {
        color: #f8fafc;
        background: none;
        padding: 0;
        font-size: 0.875rem;
      }
      :host ::ng-deep .markdown-body ul,
      :host ::ng-deep .markdown-body ol {
        margin-left: 1.5rem;
        margin-bottom: 1.25rem;
        list-style-type: disc;
      }
      :host ::ng-deep .markdown-body li {
        margin-bottom: 0.5rem;
      }
      :host ::ng-deep .markdown-body blockquote {
        border-left: 4px solid #f97316;
        background: rgba(249, 115, 22, 0.05);
        padding: 0.75rem 1.25rem;
        margin: 1.5rem 0;
        border-radius: 0 0.5rem 0.5rem 0;
        color: #cbd5e1;
      }
    `,
  ],
})
export default class BlogPostPage implements OnInit {
  private route = inject(ActivatedRoute);
  private title = inject(Title);
  private meta = inject(Meta);

  copied = false;

  readonly post$ = injectContent<PostAttributes>({
    param: 'slug',
    subdirectory: 'blog',
  });

  ngOnInit(): void {
    // Set dynamic metadata once post data is loaded
    this.post$.subscribe((post) => {
      if (post && post.attributes) {
        const titleStr = `${post.attributes.title} — Crush`;
        this.title.setTitle(titleStr);
        this.meta.updateTag({ name: 'description', content: post.attributes.excerpt });

        // OpenGraph
        this.meta.updateTag({ property: 'og:title', content: titleStr });
        this.meta.updateTag({ property: 'og:description', content: post.attributes.excerpt });

        // Twitter
        this.meta.updateTag({ name: 'twitter:title', content: titleStr });
        this.meta.updateTag({ name: 'twitter:description', content: post.attributes.excerpt });
      }
    });
  }

  shareOnTwitter(title: string): void {
    const url = encodeURIComponent(window.location.href);
    const text = encodeURIComponent(`Check out "${title}" on Crush: `);
    window.open(`https://twitter.com/intent/tweet?url=${url}&text=${text}`, '_blank');
  }

  copyLink(): void {
    navigator.clipboard.writeText(window.location.href).then(() => {
      this.copied = true;
      setTimeout(() => (this.copied = false), 2000);
    });
  }
}
