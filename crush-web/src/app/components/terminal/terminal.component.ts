import { Component, OnInit, OnDestroy, signal } from '@angular/core';
import { CommonModule } from '@angular/common';

type Mode = 'docker' | 'crush' | 'dockerfile';

interface Line {
  text: string;
  className?: string;
  outputDelay?: number;
}

const DOCKER_SEQUENCE: Line[] = [
  { text: '~/my-api $ docker build -t my-api .', outputDelay: 300 },
  {
    text: '  ↳ [1/3] downloading base node image (120 MB)... [15.2s]',
    className: 'text-crush-textMuted',
    outputDelay: 600,
  },
  {
    text: '  ↳ [2/3] npm install (fetching 342 dependencies)... [22.4s]',
    className: 'text-crush-textMuted',
    outputDelay: 600,
  },
  {
    text: '  ↳ [3/3] copying files & building typescript... [4.1s]',
    className: 'text-crush-textMuted',
    outputDelay: 400,
  },
  {
    text: '✗ built image my-api:latest (41.7s · 842 MB)',
    className: 'text-docker-blue font-semibold',
    outputDelay: 800,
  },
  { text: '~/my-api $ docker run -p 3000:3000 my-api', outputDelay: 400 },
  {
    text: '  ↳ booting hyper-v vm engine... [4.2s]',
    className: 'text-crush-textMuted',
    outputDelay: 500,
  },
  {
    text: '  ↳ mounting wsl2 project workspace... [1.5s]',
    className: 'text-crush-textMuted',
    outputDelay: 500,
  },
  {
    text: '✓ started container on :3000 (total time: 47.4s)',
    className: 'text-docker-blue font-semibold',
    outputDelay: 2000,
  },
];

const CRUSH_SEQUENCE: Line[] = [
  { text: '~/my-api $ crush', outputDelay: 300 },
  {
    text: '  ↳ detected: Node.js 20 · TypeScript · Express',
    className: 'text-crush-textMuted',
    outputDelay: 400,
  },
  {
    text: '  ↳ dependencies layer cached (unchanged)',
    className: 'text-crush-textMuted',
    outputDelay: 300,
  },
  {
    text: '✓ crushed to image my-api:latest (0.9s · 41 MB)',
    className: 'text-crush-green font-semibold',
    outputDelay: 500,
  },
  { text: '  run it now? [Y/n]', className: 'text-crush-orangeLight', outputDelay: 800 },
  {
    text: '✓ running natively on :3000 — started in 0.3s (total: 1.2s!)',
    className: 'text-crush-green font-semibold',
    outputDelay: 3000,
  },
];

const DOCKERFILE_SEQUENCE: Line[] = [
  { text: '~/my-api $ crush', outputDelay: 300 },
  {
    text: '  ↳ detected: Dockerfile (existing project)',
    className: 'text-crush-textMuted',
    outputDelay: 500,
  },
  {
    text: '  ↳ parsing multi-stage build... [0.1s]',
    className: 'text-crush-textMuted',
    outputDelay: 400,
  },
  {
    text: '  ↳ dependencies layer cached (no reinstall needed)',
    className: 'text-crush-textMuted',
    outputDelay: 400,
  },
  {
    text: '✓ crushed dockerfile → my-api:latest (1.1s · 38 MB)',
    className: 'text-crush-green font-semibold',
    outputDelay: 600,
  },
  {
    text: '  export docker-compatible image for VPS? [Y/n]',
    className: 'text-crush-orangeLight',
    outputDelay: 900,
  },
  {
    text: '  ↳ writing Dockerfile.crush · docker-compose.yml...',
    className: 'text-crush-textMuted',
    outputDelay: 500,
  },
  {
    text: '✓ hostable image ready — deploy with: crush deploy --vps',
    className: 'text-crush-green font-semibold',
    outputDelay: 3000,
  },
];

const NEXT_MODE: Record<Mode, Mode> = {
  docker: 'crush',
  crush: 'dockerfile',
  dockerfile: 'docker',
};

const MODE_LABEL: Record<Mode, string> = {
  docker: 'Docker Build (Slow & VM Heavy)',
  crush: 'Crush Build (Native & Sub-Second)',
  dockerfile: 'Dockerfile → Crush (Zero Migration)',
};

const MODE_RUNTIME: Record<Mode, string> = {
  docker: 'docker-daemon',
  crush: 'crush-runtime',
  dockerfile: 'crush-runtime',
};

const MODE_DELAY: Record<Mode, number> = {
  docker: 5000,
  crush: 7000,
  dockerfile: 6000,
};

@Component({
  selector: 'app-terminal',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div
      class="code-block overflow-hidden transition-all duration-500 shadow-2xl"
      [class.glow-docker-blue]="currentMode() === 'docker'"
      [class.glow-orange]="currentMode() !== 'docker'"
    >
      <!-- Terminal Header -->
      <div
        class="flex items-center justify-between border-b border-crush-border/50 bg-crush-dark/80 px-4 py-3 select-none"
      >
        <div class="flex items-center gap-1.5">
          <span class="w-3 h-3 rounded-full bg-[#ff5f56] block"></span>
          <span class="w-3 h-3 rounded-full bg-[#ffbd2e] block"></span>
          <span class="w-3 h-3 rounded-full bg-[#27c93f] block"></span>
        </div>

        <!-- Mode Badge -->
        <span
          class="text-xs font-mono font-bold uppercase tracking-wider transition-colors duration-500"
          [class.text-docker-blue]="currentMode() === 'docker'"
          [class.text-crush-orangeLight]="currentMode() !== 'docker'"
        >
          {{ modeLabel() }}
        </span>

        <span class="text-xs text-crush-muted font-mono select-none">
          {{ modeRuntime() }}
        </span>
      </div>

      <!-- Terminal Output Screen -->
      <div class="p-5 font-mono text-[13px] leading-relaxed min-h-[280px] bg-crush-black/90">
        @for (line of visibleLines(); track $index) {
          <div class="whitespace-pre-wrap" [class]="line.className ?? 'text-crush-text'">
            {{ line.text }}
            @if ($index === visibleLines().length - 1 && !completed()) {
              <span
                class="inline-block w-2 h-4 bg-current ml-0.5 animate-type-cursor"
                [class.text-docker-blue]="currentMode() === 'docker'"
                [class.text-crush-orange]="currentMode() !== 'docker'"
              ></span>
            }
          </div>
        }
      </div>
    </div>
  `,
})
export class TerminalComponent implements OnInit, OnDestroy {
  visibleLines = signal<Line[]>([]);
  completed = signal(false);
  currentMode = signal<Mode>('docker');

  modeLabel = () => MODE_LABEL[this.currentMode()];
  modeRuntime = () => MODE_RUNTIME[this.currentMode()];

  private timers: ReturnType<typeof setTimeout>[] = [];
  private typingIntervals: ReturnType<typeof setInterval>[] = [];
  private loopTimer: ReturnType<typeof setTimeout> | null = null;

  ngOnInit(): void {
    this.startSequence();
  }

  ngOnDestroy(): void {
    this.clearAllTimers();
  }

  private clearAllTimers(): void {
    this.timers.forEach(clearTimeout);
    this.timers = [];
    this.typingIntervals.forEach(clearInterval);
    this.typingIntervals = [];
    if (this.loopTimer) {
      clearTimeout(this.loopTimer);
      this.loopTimer = null;
    }
  }

  private getSequence(): Line[] {
    const mode = this.currentMode();
    if (mode === 'docker') return DOCKER_SEQUENCE;
    if (mode === 'crush') return CRUSH_SEQUENCE;
    return DOCKERFILE_SEQUENCE;
  }

  private startSequence(): void {
    this.clearAllTimers();
    this.visibleLines.set([]);
    this.completed.set(false);
    this.typeSequence(this.getSequence(), 0);
  }

  private typeSequence(sequence: Line[], index: number): void {
    if (index >= sequence.length) {
      this.completed.set(true);
      this.loopTimer = setTimeout(() => {
        this.currentMode.set(NEXT_MODE[this.currentMode()]);
        this.startSequence();
      }, MODE_DELAY[this.currentMode()]);
      return;
    }

    const currentLine = sequence[index]!;

    if (currentLine.text.startsWith('~')) {
      let currentChar = 0;
      const targetText = currentLine.text;
      const typingLine = { ...currentLine, text: '' };
      this.visibleLines.update((lines) => [...lines, typingLine]);

      const interval = setInterval(() => {
        currentChar++;
        this.visibleLines.update((lines) => {
          const updated = [...lines];
          if (updated.length > 0) {
            updated[updated.length - 1] = {
              ...typingLine,
              text: targetText.substring(0, currentChar),
            };
          }
          return updated;
        });

        if (currentChar === targetText.length) {
          clearInterval(interval);
          const timer = setTimeout(() => {
            this.typeSequence(sequence, index + 1);
          }, currentLine.outputDelay || 300);
          this.timers.push(timer);
        }
      }, 45);
      this.typingIntervals.push(interval);
    } else {
      this.visibleLines.update((lines) => [...lines, currentLine]);
      const timer = setTimeout(() => {
        this.typeSequence(sequence, index + 1);
      }, currentLine.outputDelay || 400);
      this.timers.push(timer);
    }
  }
}
