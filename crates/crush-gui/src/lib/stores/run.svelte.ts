// Owns the lifecycle of the *current project run* at module scope so it survives
// route changes. The dashboard used to hold this as component-local state, but
// the layout wraps every page in `{#key $page.url.pathname}` — so navigating
// away destroyed the dashboard, dropped `activeRunId`, and lost the Tauri event
// listener. The backend process kept running (orphaned), making it look like the
// run was "killed" and leaving no way to view its logs or stop it.
//
// Hoisting the run + its event listener here means a run keeps streaming into a
// persistent buffer no matter which tab you're on; come back to the dashboard
// and the terminal, status, and preview are exactly where you left them.

import * as api from '$lib/tauri';
import type { ProjectInfo, GitInfo } from '$lib/tauri';

export type RunStatus = 'running' | 'exited' | 'failed';
export type Phase = 'build' | 'runtime';
export type LineKind = 'out' | 'err' | 'meta' | 'ok' | 'warn';
export type RunLine = { text: string; kind: LineKind; phase: Phase };

const MAX_LINES = 1500;

class RunStore {
  activeRunId = $state<string | null>(null);
  status = $state<RunStatus>('running');
  port = $state<number | null>(null);
  url = $state<string | null>(null);
  lines = $state<RunLine[]>([]);
  /** Detected stack language (e.g. "flutter", "react-native", "node") — set on Detected. */
  language = $state<string | null>(null);
  /** True for Flutter/React Native runs, which mirror an emulator instead of a port. */
  get isMobile() {
    const l = (this.language ?? '').toLowerCase();
    return l.startsWith('flutter') || l.startsWith('react-native');
  }

  // Context captured at launch — needed for the overview + the deployment record.
  projectPath = $state<string | null>(null);
  project = $state<ProjectInfo | null>(null);
  git = $state<GitInfo | null>(null);

  private startMs = 0;
  private buildLog: string[] = [];
  private runtimeLog: string[] = [];
  private unlisten: (() => void) | null = null;

  /** Start a run and begin streaming its events into this store. */
  async start(projectPath: string, devMode: boolean, ctx: { project: ProjectInfo | null; git: GitInfo | null }) {
    if (this.activeRunId) return; // a run is already owned by the store
    this.reset();
    this.projectPath = projectPath;
    this.project = ctx.project;
    this.git = ctx.git;
    this.startMs = Date.now();
    try {
      const runId = await api.runProject(projectPath, devMode);
      this.activeRunId = runId;
      await this.attach(runId);
      this.saveDeployment();
    } catch (e) {
      console.error('run failed', e);
      this.status = 'failed';
    }
  }

  /** Abort the backend process (explicit user action). */
  stop() {
    if (this.activeRunId) api.abortRun(this.activeRunId).catch(console.error);
  }

  /** Dismiss a finished run from the UI without touching the backend. */
  close() {
    this.unlisten?.();
    this.unlisten = null;
    this.activeRunId = null;
    this.port = null;
    this.url = null;
    this.status = 'running';
    this.lines = [];
    this.language = null;
    this.buildLog = [];
    this.runtimeLog = [];
  }

  private reset() {
    this.unlisten?.();
    this.unlisten = null;
    this.status = 'running';
    this.port = null;
    this.url = null;
    this.lines = [];
    this.language = null;
    this.buildLog = [];
    this.runtimeLog = [];
  }

  private push(text: string, kind: LineKind, phase: Phase = 'build') {
    this.lines = [...this.lines.slice(-(MAX_LINES - 1)), { text, kind, phase }];
    (phase === 'build' ? this.buildLog : this.runtimeLog).push(text);
  }

  // Dev servers (Vite, etc.) may bump to a free port — trust the URL printed to
  // stdout over the detected port.
  private scanPort(line: string) {
    const clean = line.replace(/\x1b\[[0-9;]*[a-zA-Z]/g, '').replace(/\x1b\]8;;.*?\x1b\\/g, '');
    const m = /https?:\/\/(?:localhost|127\.0\.0\.1|0\.0\.0\.0):(\d+)/i.exec(clean);
    if (m) this.port = Number(m[1]);
  }

  // Prefer a docs/swagger URL (the meaningful preview for a backend), else first.
  private pickUrl(urls: [string, string][]): string | null {
    if (!urls || !urls.length) return null;
    const docs = urls.find(([, u]) => /swagger|\/docs|redoc|openapi|\/api-docs/i.test(u));
    return (docs ?? urls[0])[1] ?? null;
  }

  private fmtMB(b: number): string {
    return b < 1_000_000 ? `${(b / 1000).toFixed(0)} KB` : `${(b / 1_000_000).toFixed(0)} MB`;
  }

  private async attach(runId: string) {
    this.unlisten = await api.onRunEvent(runId, (event) => {
      const e = event as any;
      switch (e.kind) {
        case 'detected':
          this.language = e.language ?? null;
          this.push(`↳ detected ${e.language}${e.framework ? ` · ${e.framework}` : ''} · :${e.port}${e.is_monorepo ? ` · monorepo (${e.dep_count} svc)` : ''}`, 'meta'); break;
        case 'warm-run': this.push('warm run — launching', 'meta'); break;
        case 'deps-fresh': this.push('dependencies fresh — node_modules up to date', 'meta'); break;
        case 'dep-started': this.push(`✓ ${e.name} started${e.native ? ' (native)' : ` · ${e.image}`}`, 'ok'); break;
        case 'dep-failed': this.push(`✗ ${e.name} failed: ${e.error}`, 'err'); break;
        case 'image-fresh': this.push('image fresh — skipping pack', 'meta'); break;
        case 'image-packed': this.push(`crushed to image${e.size_bytes ? ` (${this.fmtMB(e.size_bytes)})` : ''}`, 'ok'); break;
        case 'build-started': this.push(`build: ${e.command ?? ''}`, 'meta'); break;
        case 'build-output': this.scanPort(e.line); this.push(e.line, e.stream === 'stderr' ? 'err' : 'out'); break;
        case 'build-finished': this.push(`build finished${e.duration_ms ? ` in ${(e.duration_ms / 1000).toFixed(1)}s` : ''}`, 'meta'); break;
        case 'spawning': this.push(`spawning${e.command ? `: ${e.command}` : ''}${e.port ? ` on :${e.port}` : ''}`, 'meta', 'runtime'); break;
        case 'app-output': this.scanPort(e.line); this.push(e.line, e.stream === 'stderr' ? 'err' : 'out', 'runtime'); break;
        case 'port-bound': {
          if (e.port) this.port = e.port;
          const best = this.pickUrl(e.urls ?? []);
          if (best) this.url = best;
          const urls = (e.urls ?? []).map((u: [string, string]) => u[1]).join('  ');
          this.push(`✓ ready on :${e.port}${urls ? ` — ${urls}` : ''}`, 'ok', 'runtime'); break;
        }
        case 'warning': this.push(`! ${e.message ?? e.text ?? ''}`, 'warn', 'runtime'); break;
        case 'aborted':
          this.status = 'exited';
          this.push('run aborted', 'meta', 'runtime');
          this.saveDeployment(); break;
        case 'exited':
          this.status = e.code === 0 ? 'exited' : 'failed';
          this.push(`process exited (code ${e.code})`, e.code === 0 ? 'meta' : 'err', 'runtime');
          this.saveDeployment(); break;
        default: break;
      }
    });
  }

  private saveDeployment() {
    if (!this.activeRunId || !this.projectPath) return;
    const name = this.project?.name ?? baseName(this.projectPath);
    const ended = this.status === 'running' ? null : Date.now();
    const recordStatus: 'running' | 'ready' | 'failed' =
      this.status === 'running' ? 'running' : this.status === 'failed' ? 'failed' : 'ready';
    api.saveDeployment({
      id: this.activeRunId,
      project: name,
      project_path: this.projectPath,
      created_ms: this.startMs,
      ended_ms: ended,
      duration_ms: ended ? ended - this.startMs : 0,
      status: recordStatus,
      port: this.port,
      runtime: this.project?.runtime ?? null,
      framework: this.project?.framework ?? null,
      build_log: this.buildLog.join('\n'),
      runtime_log: this.runtimeLog.join('\n'),
      has_screenshot: false,
      branch: this.git?.branch ?? undefined,
      commit_short: this.git?.head?.short ?? undefined,
      commit_message: this.git?.head?.message ?? undefined,
    }).catch(console.error);
  }
}

function baseName(p: string): string { return p.split(/[\\/]/).filter(Boolean).pop() ?? p; }

export const run = new RunStore();
