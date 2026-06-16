import { writable } from 'svelte/store';

export type OverlayType = 'modal' | 'drawer' | 'contextMenu' | 'tooltip' | 'hoverCard' | 'commandPalette';

export interface Overlay {
  id: string;
  type: OverlayType;
  node: HTMLElement;
  triggerNode?: HTMLElement;
  close: () => void;
}

const { subscribe, update } = writable<Overlay[]>([]);

export const overlayRegistry = {
  subscribe,
  register: (overlay: Overlay) => {
    update(n => [...n, overlay]);
  },
  unregister: (id: string) => {
    update(n => {
      const idx = n.findIndex(o => o.id === id);
      if (idx !== -1) {
        // Return focus
        const o = n[idx];
        if (o.triggerNode) {
          o.triggerNode.focus();
        }
      }
      return n.filter(o => o.id !== id);
    });
  },
  closeTop: () => {
    update(n => {
      if (n.length > 0) {
        const top = n[n.length - 1];
        top.close();
        if (top.triggerNode) {
          top.triggerNode.focus();
        }
        return n.slice(0, -1);
      }
      return n;
    });
  }
};

// Global escape listener for layered close
if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      overlayRegistry.closeTop();
    }
  });
}
