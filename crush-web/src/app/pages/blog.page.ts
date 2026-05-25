import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { RouterLink } from '@angular/router';
import { HlmButtonDirective } from '@spartan-ng/ui-button-helm';
import { HlmBadgeDirective } from '../ui/badge';

@Component({
  selector: 'page-blog',
  standalone: true,
  imports: [RouterLink, HlmButtonDirective, HlmBadgeDirective],
  template: `
    <div class="mx-auto max-w-3xl px-4 py-12 sm:px-6 lg:px-8">
      <h1 class="text-3xl font-bold text-white mb-2">Blog</h1>
      <p class="text-lg text-crush-textMuted mb-12">
        Release notes, announcements, and technical deep dives.
      </p>

      @if (posts.length === 0) {
        <div class="rounded-xl border border-crush-border/50 bg-crush-surface/30 p-12 text-center">
          <p class="text-crush-textMuted mb-2">No posts yet.</p>
          <p class="text-sm text-crush-muted">
            Check back soon for launch announcements and technical content.
          </p>
        </div>
      } @else {
        <div class="space-y-8">
          @for (post of posts; track post.slug) {
            <article
              class="rounded-xl border border-crush-border/50 bg-crush-surface/30 p-6 hover:border-crush-orange/30 transition-colors"
            >
              <div class="flex items-center gap-3 mb-3">
                <span
                  hlmBadge
                  variant="outline"
                  class="border-crush-orange/30 bg-crush-orange/10 text-crush-orange hover:bg-crush-orange/20"
                  >{{ post.tag }}</span
                >
                <time class="text-xs text-crush-muted">{{ post.date }}</time>
              </div>
              <h2 class="text-xl font-semibold text-white mb-2">{{ post.title }}</h2>
              <p class="text-sm text-crush-textMuted">{{ post.excerpt }}</p>
            </article>
          }
        </div>
      }
    </div>
  `,
})
export default class BlogPage implements OnInit {
  posts: { slug: string; title: string; excerpt: string; date: string; tag: string }[] = [];

  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit(): void {
    this.title.setTitle('Blog — Crush');
    this.meta.updateTag({
      name: 'description',
      content:
        'Crush blog — release notes, announcements, and technical content about native Windows containers.',
    });
  }
}
