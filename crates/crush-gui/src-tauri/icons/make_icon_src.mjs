// Generates a square 1024x1024 PNG of the Crush brand diamond
// (orange #e05540 on near-black #09090b) for `tauri icon` to consume.
// Dependency-free: hand-rolls a valid RGBA PNG via zlib.
import { deflateSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';

const SIZE = 1024;
const bg = [0x09, 0x09, 0x0b];
const fg = [0xe0, 0x55, 0x40];

const cx = SIZE / 2;
const cy = SIZE / 2;
const r = SIZE * 0.40; // half-diagonal of the diamond

// L1 (diamond) distance, with 3x3 supersampling for smooth edges.
function coverage(px, py) {
  let hits = 0;
  for (let sx = 0; sx < 3; sx++) {
    for (let sy = 0; sy < 3; sy++) {
      const x = px + (sx + 0.5) / 3;
      const y = py + (sy + 0.5) / 3;
      if (Math.abs(x - cx) + Math.abs(y - cy) <= r) hits++;
    }
  }
  return hits / 9;
}

const raw = Buffer.alloc((SIZE * 4 + 1) * SIZE);
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0; // filter: none
  for (let x = 0; x < SIZE; x++) {
    const c = coverage(x, y);
    const idx = y * (SIZE * 4 + 1) + 1 + x * 4;
    raw[idx] = Math.round(bg[0] + (fg[0] - bg[0]) * c);
    raw[idx + 1] = Math.round(bg[1] + (fg[1] - bg[1]) * c);
    raw[idx + 2] = Math.round(bg[2] + (fg[2] - bg[2]) * c);
    raw[idx + 3] = 255;
  }
}

function crc32(buf) {
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    crc ^= buf[i];
    for (let j = 0; j < 8; j++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw)),
  chunk('IEND', Buffer.alloc(0)),
]);

const out = new URL('icon-src.png', import.meta.url);
writeFileSync(out, png);
console.log(`wrote ${out.pathname} (${png.length} bytes, ${SIZE}x${SIZE})`);
