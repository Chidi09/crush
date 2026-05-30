// Builds a square `icon-src.png` from the real brand logo (static/logo.png)
// for `tauri icon` to consume. The logo is 1536x1024 (3:2), so we decode it
// and centre it on a transparent square canvas — otherwise `tauri icon` would
// squash it. Dependency-free: hand-rolled PNG decode + encode via zlib.
import { inflateSync, deflateSync } from 'node:zlib';
import { readFileSync, writeFileSync } from 'node:fs';

const SRC = new URL('../../static/logo.png', import.meta.url);
const OUT = new URL('icon-src.png', import.meta.url);

// ── Decode an 8-bit RGBA, non-interlaced PNG ──────────────────────────────
function decodePng(buf) {
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error('not a PNG');
  let p = 8, width = 0, height = 0, bitDepth = 0, colorType = 0, interlace = 0;
  const idat = [];
  while (p < buf.length) {
    const len = buf.readUInt32BE(p); p += 4;
    const type = buf.toString('ascii', p, p + 4); p += 4;
    const data = buf.subarray(p, p + len); p += len + 4; // skip CRC
    if (type === 'IHDR') {
      width = data.readUInt32BE(0); height = data.readUInt32BE(4);
      bitDepth = data[8]; colorType = data[9]; interlace = data[12];
    } else if (type === 'IDAT') { idat.push(data); }
    else if (type === 'IEND') break;
  }
  if (bitDepth !== 8 || colorType !== 6 || interlace !== 0) {
    throw new Error(`unsupported PNG (bitDepth=${bitDepth} colorType=${colorType} interlace=${interlace}); expected 8-bit RGBA non-interlaced`);
  }
  const raw = inflateSync(Buffer.concat(idat));
  const ch = 4, stride = width * ch;
  const out = Buffer.alloc(height * stride);
  let prev = Buffer.alloc(stride), pos = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[pos++];
    const cur = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const a = x >= ch ? cur[x - ch] : 0;
      const b = prev[x];
      const c = x >= ch ? prev[x - ch] : 0;
      let v = raw[pos + x];
      if (filter === 1) v = (v + a) & 0xff;
      else if (filter === 2) v = (v + b) & 0xff;
      else if (filter === 3) v = (v + ((a + b) >> 1)) & 0xff;
      else if (filter === 4) {
        const pp = a + b - c;
        const pa = Math.abs(pp - a), pb = Math.abs(pp - b), pc = Math.abs(pp - c);
        v = (v + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c)) & 0xff;
      }
      cur[x] = v;
    }
    pos += stride;
    cur.copy(out, y * stride);
    prev = cur;
  }
  return { width, height, data: out };
}

// ── Encode an 8-bit RGBA PNG ──────────────────────────────────────────────
function crc32(buf) {
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    crc ^= buf[i];
    for (let j = 0; j < 8; j++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
  const t = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}
function encodePng(size, rgba) {
  const stride = size * 4;
  const raw = Buffer.alloc((stride + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (stride + 1)] = 0; // filter: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, y * stride + stride);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; ihdr[9] = 6; // 8-bit RGBA
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

// ── Pad to square, centred, transparent background ────────────────────────
const img = decodePng(readFileSync(SRC));
const SIZE = Math.max(img.width, img.height);
const sq = Buffer.alloc(SIZE * SIZE * 4); // all-zero = transparent
const offX = (SIZE - img.width) >> 1;
const offY = (SIZE - img.height) >> 1;
for (let y = 0; y < img.height; y++) {
  for (let x = 0; x < img.width; x++) {
    const si = (y * img.width + x) * 4;
    const di = ((y + offY) * SIZE + (x + offX)) * 4;
    sq[di] = img.data[si]; sq[di + 1] = img.data[si + 1];
    sq[di + 2] = img.data[si + 2]; sq[di + 3] = img.data[si + 3];
  }
}
const png = encodePng(SIZE, sq);
writeFileSync(OUT, png);
console.log(`wrote ${OUT.pathname} (${png.length} bytes, ${SIZE}x${SIZE}) from ${img.width}x${img.height} logo`);
