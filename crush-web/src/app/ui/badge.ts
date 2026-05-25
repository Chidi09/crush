import { Directive, input } from '@angular/core';

@Directive({
  selector: '[hlmBadge]',
  standalone: true,
  host: {
    class:
      'inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
  },
})
export class HlmBadgeDirective {
  readonly variant = input<'default' | 'secondary' | 'destructive' | 'outline'>('default');
}
