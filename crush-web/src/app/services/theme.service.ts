import { Injectable, inject, PLATFORM_ID, signal, effect } from '@angular/core';
import { isPlatformBrowser } from '@angular/common';

@Injectable({
  providedIn: 'root',
})
export class ThemeService {
  private platformId = inject(PLATFORM_ID);

  // Theme state signal: true for dark mode, false for light mode
  isDark = signal<boolean>(true); // default to dark

  constructor() {
    if (isPlatformBrowser(this.platformId)) {
      // 1. Read stored preference from localStorage
      const storedTheme = localStorage.getItem('theme');

      if (storedTheme) {
        this.isDark.set(storedTheme === 'dark');
      } else {
        // 2. Fallback to system preferences
        const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        this.isDark.set(systemPrefersDark);
      }

      // 3. Reactively update DOM and local storage when isDark changes
      effect(() => {
        const dark = this.isDark();
        const root = window.document.documentElement;

        if (dark) {
          root.classList.add('dark');
          localStorage.setItem('theme', 'dark');
        } else {
          root.classList.remove('dark');
          localStorage.setItem('theme', 'light');
        }
      });
    }
  }

  toggleTheme(): void {
    this.isDark.set(!this.isDark());
  }
}
