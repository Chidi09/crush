// Global error guard — keeps failures professional.
//
// A Tauri webview renders `alert()` as a native "localhost says…" dialog and
// lets uncaught errors / rejected promises surface as raw browser noise. None
// of that reads as a finished native app. This routes everything through the
// in-app Toaster instead, so the user only ever sees branded, dismissible toasts.
import { toast } from '$lib/stores/toast.svelte.ts';

function clean(msg: string): string {
  return msg
    .replace(/^(Uncaught\s+)?Error:\s*/i, '')
    .replace(/^invoke error:\s*/i, '')
    .trim() || 'Something went wrong';
}

// Failures/denials → error toast; everything else (e.g. "Copied", "Saved") → info.
function kindFor(msg: string): 'error' | 'info' {
  return /\b(fail|failed|error|denied|unable|cannot|can't|could not|couldn't|invalid|not found|timed out|refused)\b/i.test(msg)
    ? 'error'
    : 'info';
}

let installed = false;

export function installErrorGuard(): void {
  if (installed) return;
  installed = true;

  // Uncaught runtime errors. Ignore resource-load errors (e.g. an <img> favicon
  // that 404s) — those are handled inline with onerror fallbacks.
  window.addEventListener('error', (e: ErrorEvent) => {
    if (e.target && e.target !== window && (e.target as HTMLElement).tagName) return;
    toast(clean(e.message || 'Something went wrong'), 'error');
    if (import.meta.env.PROD) e.preventDefault();
  });

  // Rejected promises (most invoke() failures land here if not caught).
  window.addEventListener('unhandledrejection', (e: PromiseRejectionEvent) => {
    const r: any = e.reason;
    const msg = typeof r === 'string' ? r : (r?.message ?? (r != null ? String(r) : 'Operation failed'));
    toast(clean(msg), 'error');
    if (import.meta.env.PROD) e.preventDefault();
  });

  // Replace the webview's native alert() with a toast. This upgrades every
  // existing alert(...) call across the app without touching each call site.
  window.alert = (message?: unknown) => {
    const msg = clean(String(message ?? ''));
    toast(msg, kindFor(msg));
  };
}
