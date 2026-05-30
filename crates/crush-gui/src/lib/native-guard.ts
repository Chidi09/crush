// Make the Tauri webview feel like a native app instead of a browser tab.
//
// A Tauri window is a WebView2 surface, so by default it honours browser
// behaviour: the right-click context menu, F5 / Ctrl+R reload, and the
// Ctrl+Shift+I devtools shortcut. In production we suppress those so the app
// can't be "refreshed" like a web page. They stay enabled in dev (vite serve)
// so hot-reload and the inspector still work.
export function installNativeGuard(): void {
  if (!import.meta.env.PROD) return;

  const swallow = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
  };

  // No browser context menu.
  window.addEventListener('contextmenu', swallow);

  // No reload / devtools accelerators.
  window.addEventListener(
    'keydown',
    (e: KeyboardEvent) => {
      const k = e.key.toLowerCase();
      const reload = k === 'f5' || ((e.ctrlKey || e.metaKey) && k === 'r');
      const devtools =
        k === 'f12' ||
        ((e.ctrlKey || e.metaKey) && e.shiftKey && (k === 'i' || k === 'j' || k === 'c'));
      if (reload || devtools) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    { capture: true },
  );
}
