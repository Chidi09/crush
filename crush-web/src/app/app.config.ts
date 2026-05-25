import { ApplicationConfig, provideZoneChangeDetection } from '@angular/core';
import { provideFileRouter } from '@analogjs/router';
import { provideContent, withMarkdownRenderer } from '@analogjs/content';
import { withPrismHighlighter } from '@analogjs/content/prism-highlighter';
import { provideHttpClient, withFetch } from '@angular/common/http';
import { provideIcons } from '@spartan-ng/ui-icon-helm';
import {
  lucideMenu,
  lucideX,
  lucideExternalLink,
  lucideChevronRight,
  lucideCopy,
  lucideCheck,
  lucideZap,
  lucideSquareTerminal,
  lucideBrain,
  lucideArrowRight,
  lucideBookOpen,
  lucidePackage,
  lucideShield,
  lucideTerminal,
  lucideContrast,
  lucideSun,
  lucideMoon,
} from '@ng-icons/lucide';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideFileRouter(),
    provideHttpClient(withFetch()),
    provideContent(withMarkdownRenderer(), withPrismHighlighter()),
    provideIcons({
      lucideMenu,
      lucideX,
      // Github was removed from lucide; ExternalLink is used for GitHub links
      lucideGithub: lucideExternalLink,
      lucideChevronRight,
      lucideCopy,
      lucideCheck,
      lucideZap,
      lucideSquareTerminal,
      lucideBrain,
      lucideArrowRight,
      lucideExternalLink,
      lucideBookOpen,
      lucidePackage,
      lucideShield,
      lucideTerminal,
      lucideContrast,
      lucideSun,
      lucideMoon,
    }),
  ],
};
