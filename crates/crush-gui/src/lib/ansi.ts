// Minimal ANSI SGR → styled segments, so program output (vite, cargo, pip…)
// renders with real colors instead of raw escape codes. Shared by the run
// terminal and the Logs page.

export type AnsiSeg = { text: string; style: string };

const FG: Record<number, string> = {
  30: '#6b6b80', 31: '#fca5a5', 32: '#4ade80', 33: '#eab308', 34: '#60a5fa',
  35: '#c084fc', 36: '#22d3ee', 37: '#e8e8ed',
  90: '#9a9ab0', 91: '#fca5a5', 92: '#86efac', 93: '#fde047', 94: '#93c5fd',
  95: '#d8b4fe', 96: '#67e8f9', 97: '#ffffff',
};

export function parseAnsi(input: string): AnsiSeg[] {
  const segs: AnsiSeg[] = [];
  // strip OSC-8 hyperlinks (ESC ] 8 ;; … ESC \) — keep the visible text only
  const cleaned = input.replace(/\x1b\]8;;.*?(?:\x1b\\|\x07)/g, '');
  // eslint-disable-next-line no-control-regex
  const re = /\x1b\[([0-9;]*)m/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let fg = ''; let bold = false; let dim = false; let italic = false; let underline = false;
  const style = () => {
    const p: string[] = [];
    if (fg) p.push(`color:${fg}`);
    if (bold) p.push('font-weight:700');
    if (dim) p.push('opacity:0.6');
    if (italic) p.push('font-style:italic');
    if (underline) p.push('text-decoration:underline');
    return p.join(';');
  };
  const emit = (t: string) => { if (t) segs.push({ text: t, style: style() }); };
  while ((m = re.exec(cleaned)) !== null) {
    emit(cleaned.slice(last, m.index));
    last = re.lastIndex;
    const codes = m[1] === '' ? [0] : m[1].split(';').map(Number);
    for (let i = 0; i < codes.length; i++) {
      const c = codes[i];
      if (c === 0) { fg = ''; bold = dim = italic = underline = false; }
      else if (c === 1) bold = true;
      else if (c === 2) dim = true;
      else if (c === 22) { bold = false; dim = false; }
      else if (c === 3) italic = true;
      else if (c === 23) italic = false;
      else if (c === 4) underline = true;
      else if (c === 24) underline = false;
      else if (c === 39) fg = '';
      else if (FG[c]) fg = FG[c];
      else if (c === 38 || c === 48) { i += codes[i + 1] === 5 ? 2 : codes[i + 1] === 2 ? 4 : 0; }
    }
  }
  emit(cleaned.slice(last));
  return segs.length ? segs : [{ text: cleaned, style: '' }];
}
