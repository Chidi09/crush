import { Injectable } from '@angular/core';

@Injectable({ providedIn: 'root' })
export class CopyService {
  private timeout: ReturnType<typeof setTimeout> | null = null;

  async copy(text: string): Promise<boolean> {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand('copy');
        return true;
      } catch {
        return false;
      } finally {
        document.body.removeChild(textarea);
      }
    }
  }

  showCopied(element: HTMLElement, text = 'Copied!'): void {
    const original = element.textContent;
    element.textContent = text;
    element.classList.add('text-crush-green');
    if (this.timeout) clearTimeout(this.timeout);
    this.timeout = setTimeout(() => {
      element.textContent = original;
      element.classList.remove('text-crush-green');
    }, 2000);
  }
}
