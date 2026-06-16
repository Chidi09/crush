// Promise-based global confirm / prompt — replaces the webview's native
// confirm()/prompt() ("localhost says…") dialogs with a branded in-app modal.
//
// Usage:
//   import { confirmAction, promptInput } from '$lib/stores/confirm.svelte.ts';
//   if (!await confirmAction({ message: 'Delete this?', danger: true })) return;
//   const name = await promptInput({ title: 'New folder', placeholder: 'images' });

export interface ConfirmOptions {
  title?: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  /** Style the confirm button as destructive (red). */
  danger?: boolean;
}

export interface PromptOptions extends Omit<ConfirmOptions, 'message'> {
  message?: string;
  placeholder?: string;
  defaultValue?: string;
}

interface DialogState {
  mode: 'confirm' | 'prompt';
  title: string;
  message: string;
  confirmText: string;
  cancelText: string;
  danger: boolean;
  placeholder: string;
  resolve: (value: boolean | string | null) => void;
}

function makeConfirm() {
  let current = $state<DialogState | null>(null);
  let input = $state('');

  function confirm(opts: ConfirmOptions): Promise<boolean> {
    return new Promise((resolve) => {
      current = {
        mode: 'confirm',
        title: opts.title ?? 'Are you sure?',
        message: opts.message,
        confirmText: opts.confirmText ?? 'Confirm',
        cancelText: opts.cancelText ?? 'Cancel',
        danger: opts.danger ?? false,
        placeholder: '',
        resolve: resolve as (v: boolean | string | null) => void,
      };
    });
  }

  function prompt(opts: PromptOptions): Promise<string | null> {
    return new Promise((resolve) => {
      input = opts.defaultValue ?? '';
      current = {
        mode: 'prompt',
        title: opts.title ?? 'Enter a value',
        message: opts.message ?? '',
        confirmText: opts.confirmText ?? 'OK',
        cancelText: opts.cancelText ?? 'Cancel',
        danger: opts.danger ?? false,
        placeholder: opts.placeholder ?? '',
        resolve: resolve as (v: boolean | string | null) => void,
      };
    });
  }

  function accept() {
    const d = current;
    if (!d) return;
    current = null;
    d.resolve(d.mode === 'prompt' ? input : true);
  }

  function cancel() {
    const d = current;
    if (!d) return;
    current = null;
    d.resolve(d.mode === 'prompt' ? null : false);
  }

  return {
    get current() { return current; },
    get input() { return input; },
    set input(v: string) { input = v; },
    confirm,
    prompt,
    accept,
    cancel,
  };
}

export const confirmStore = makeConfirm();

/** Ask the user to confirm an action. Resolves true if confirmed. */
export function confirmAction(opts: ConfirmOptions): Promise<boolean> {
  return confirmStore.confirm(opts);
}

/** Ask the user for a text value. Resolves the string, or null if cancelled. */
export function promptInput(opts: PromptOptions): Promise<string | null> {
  return confirmStore.prompt(opts);
}
