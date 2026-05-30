import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { injectContentFiles } from '@analogjs/content';

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
  selector: 'page-blog-index',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="relative min-h-screen py-16 overflow-hidden">
      <!-- Ambient light gradients background -->
      <div
        class="absolute top-0 left-1/2 -translate-x-1/2 w-[500px] h-[500px] bg-crush-orange/5 blur-[120px] pointer-events-none rounded-full"
      ></div>

      <div class="mx-auto max-w-4xl px-4 sm:px-6 relative">
        <header class="mb-16 text-center select-none">
          <h1
            class="text-4xl font-extrabold text-white tracking-tight sm:text-5xl lg:text-6xl mb-4"
          >
            The <span class="gradient-text">Crush Blog</span>
          </h1>
          <p class="text-lg text-crush-textMuted max-w-2xl mx-auto text-balance">
            Systems programming deep dives, dev tools insights, and release announcements for native
            Windows container-less workflows.
          </p>
        </header>

        @if (posts.length === 0) {
          <div
            class="rounded-2xl border border-crush-border/50 bg-crush-surface/20 p-16 text-center backdrop-blur-sm"
          >
            <p class="text-crush-textMuted mb-2">No posts published yet.</p>
            <p class="text-sm text-crush-muted">
              Check back soon for system design guides and launch logs.
            </p>
          </div>
        } @else {
          <div class="space-y-6">
            @for (post of posts; track post.slug) {
              <a
                [routerLink]="['/blog', post.slug]"
                class="block rounded-2xl border border-crush-border/50 bg-crush-surface/10 p-6 sm:p-8 hover:border-crush-orange/40 hover:bg-crush-surface/20 hover:shadow-xl hover:shadow-crush-orange/[0.02] hover:-translate-y-0.5 transition-all duration-300 group cursor-pointer relative overflow-hidden backdrop-blur-sm"
              >
                <!-- Interactive Corner Glow -->
                <div
                  class="absolute -right-16 -top-16 w-36 h-36 rounded-full bg-crush-orange/2 blur-3xl group-hover:bg-crush-orange/6 transition-all duration-500 pointer-events-none"
                ></div>

                <div class="flex flex-col justify-between h-full relative">
                  <div>
                    <!-- Meta info & tag -->
                    <div class="flex items-center gap-3 mb-4 flex-wrap">
                      <span
                        class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold border border-crush-orange/30 bg-crush-orange/10 text-crush-orange group-hover:bg-crush-orange/20 transition-colors"
                      >
                        {{ post.attributes.tag }}
                      </span>
                      <span class="text-xs text-crush-muted">•</span>
                      <time class="text-xs text-crush-muted">{{ post.attributes.date }}</time>
                      <span class="text-xs text-crush-muted">•</span>
                      <span class="text-xs text-crush-textMuted">{{
                        post.attributes.readingTime
                      }}</span>
                    </div>

                    <!-- Title -->
                    <h2
                      class="text-xl sm:text-2xl font-bold text-white mb-3 group-hover:text-crush-orangeLight transition-colors duration-200"
                    >
                      {{ post.attributes.title }}
                    </h2>

                    <!-- Excerpt -->
                    <p
                      class="text-sm sm:text-base text-crush-textMuted leading-relaxed mb-6 group-hover:text-crush-text transition-colors duration-200"
                    >
                      {{ post.attributes.excerpt }}
                    </p>
                  </div>

                  <!-- Author block & Read more -->
                  <div
                    class="flex items-center justify-between border-t border-crush-border/30 pt-4 flex-wrap gap-4"
                  >
                    <div class="flex items-center gap-2.5">
                      <div
                        class="h-8 w-8 rounded-full border border-crush-border overflow-hidden bg-crush-surface flex items-center justify-center"
                      >
                        @if (post.attributes.authorImage) {
                          <img
                            [src]="post.attributes.authorImage"
                            [alt]="post.attributes.author"
                            class="h-full w-full object-cover"
                          />
                        } @else {
                          <span class="text-xs font-bold text-crush-orange">C</span>
                        }
                      </div>
                      <span class="text-xs font-medium text-white">{{
                        post.attributes.author
                      }}</span>
                    </div>

                    <!-- Read Link -->
                    <span
                      class="inline-flex items-center gap-1 text-xs font-bold text-crush-orange group-hover:text-crush-orangeLight transition-colors duration-200 select-none"
                    >
                      Read Article
                      <svg
                        viewBox="0 0 24 24"
                        class="h-3 w-3 fill-none stroke-current stroke-3 transition-transform group-hover:translate-x-0.5"
                      >
                        <line x1="5" y1="12" x2="19" y2="12" />
                        <polyline points="12 5 19 12 12 19" />
                      </svg>
                    </span>
                  </div>
                </div>
              </a>
            }
          </div>
        }
      </div>
    </div>
  `,
})
export default class BlogIndexPage implements OnInit {
  // Dynamically load all Markdown files under /src/content/blog/
  readonly posts = injectContentFiles<PostAttributes>((contentFile) =>
    contentFile.filename.includes('/src/content/blog/')
  )
    .map((post) => ({ ...post, slug: post.slug.replace('blog/', '') }))
    .sort((a, b) => new Date(b.attributes.date).getTime() - new Date(a.attributes.date).getTime());

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Blog — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Technical articles, announcements, and deep dives on native Windows systems development, Job Objects, and VM-free local containers.',
    });
  }
}
