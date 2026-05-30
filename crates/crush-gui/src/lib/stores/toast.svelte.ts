// Lightweight global toast system — professional action feedback (copied,
// ejected, deploy started, errors) without alert()/console noise.
export type ToastKind = 'success' | 'error' | 'info';
export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
}

function makeToaster() {
  let items = $state<Toast[]>([]);
  let seq = 0;

  function dismiss(id: number) {
    items = items.filter((t) => t.id !== id);
  }
  function push(text: string, kind: ToastKind = 'info', ms = 3200) {
    const id = ++seq;
    items = [...items, { id, kind, text }];
    if (ms > 0) setTimeout(() => dismiss(id), ms);
    return id;
  }

  return {
    get items() { return items; },
    push,
    dismiss,
  };
}

export const toaster = makeToaster();

/** Fire a toast from anywhere: `toast('Copied', 'success')`. */
export function toast(text: string, kind: ToastKind = 'info', ms?: number) {
  return toaster.push(text, kind, ms);
}
