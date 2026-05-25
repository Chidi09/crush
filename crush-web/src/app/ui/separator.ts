import { Directive } from '@angular/core';

@Directive({
  selector: '[hlmSeparator]',
  standalone: true,
  host: { class: 'block h-px w-full bg-border', role: 'separator' },
})
export class HlmSeparatorDirective {}
