import { Component, OnInit } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { DocsSidebarComponent } from '../../components/docs-sidebar/docs-sidebar.component';

@Component({
  selector: 'page-branch-previews',
  standalone: true,
  imports: [DocsSidebarComponent],
  template: `
    <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
      <div class="flex flex-col md:flex-row gap-12">
        <app-docs-sidebar />
        <article class="flex-1 min-w-0">
          <!-- Page Header -->
          <div class="border-b border-crush-border/30 pb-6 mb-10 select-none">
            <span class="text-xs font-bold uppercase tracking-wider text-crush-orange"
              >Feature</span
            >
            <h1 class="text-3xl font-extrabold text-white tracking-tight mt-1 mb-2">
              Branch Previews
            </h1>
            <p class="text-base text-crush-textMuted">
              Test other Git branches in seconds without stashing changes or switching directories,
              driven by native Git worktrees.
            </p>
          </div>

          <!-- Section 1: Introduction -->
          <section class="mb-12">
            <h2 class="text-xl font-bold text-white mb-4">Git Worktree Previews</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              When working on a complex feature, getting asked to review a colleague's PR or switch
              branches usually means stashing active changes, running <code>git checkout</code>,
              re-building packages, and breaking your local database state.
            </p>
            <p class="text-sm text-crush-textMuted leading-relaxed mt-4">
              Crush solves this by leveraging native <strong>Git Worktrees</strong>. It creates a
              lightweight, isolated copy of your repository for the target branch in a temporary
              system directory, allowing you to run, build, and test a separate branch completely in
              parallel with your primary coding environment.
            </p>
          </section>

          <!-- Section 2: How to trigger -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">Previewing a Branch</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed mb-4">
              Inside the Crush GUI or via CLI, you can select any local or remote git branch and
              click "Preview".
            </p>

            <div
              class="rounded-xl border border-crush-border/40 bg-crush-black/60 overflow-hidden mb-4"
            >
              <div
                class="flex items-center justify-between px-4 py-2 border-b border-crush-border/30 bg-crush-surface/30"
              >
                <span class="text-[10px] text-crush-textMuted font-mono">Terminal</span>
                <span class="text-[9px] text-crush-textMuted uppercase font-semibold">preview</span>
              </div>
              <div class="p-4 font-mono text-sm overflow-x-auto text-crush-text">
                <code>crush preview feat/new-login-flow</code>
              </div>
            </div>

            <p class="text-sm text-crush-textMuted leading-relaxed">
              This command automatically checkouts the branch, provisions isolated ports for
              Postgres/Redis databases (preventing conflict with your main app), and starts the
              runtime engine, presenting you with a preview URL instantly.
            </p>
          </section>

          <!-- Section 3: Cleanup -->
          <section class="mb-12">
            <h2 class="text-lg font-bold text-white mb-4">Zero-Clutter Cleanups</h2>
            <p class="text-sm text-crush-textMuted leading-relaxed">
              Once you are finished reviewing the branch, simply hit "Stop Preview" or run
              <code>crush preview --clean</code>. Crush shuts down the isolated database processes
              and fully deletes the temporary git worktree folder, keeping your developer
              workstation perfectly clean.
            </p>
          </section>
        </article>
      </div>
    </div>
  `,
})
export class BranchPreviewsPageComponent implements OnInit {
  constructor(
    private title: Title,
    private meta: Meta
  ) {}

  ngOnInit() {
    this.title.setTitle('Git Branch Previews - Crush Docs');
    this.meta.updateTag({
      name: 'description',
      content:
        'Instantly test Git branches in parallel isolated environments using native Git worktrees.',
    });
  }
}
