import { writeFileSync } from 'fs';

function createPNG(size) {
  // Minimal valid RGBA PNG using Node's built-in zlib
  const zlib = await import('zlib');
  const { deflateSync } = zlib;
  
  const width = size, height = size;
  const rawData = Buffer.alloc((width * 4 + 1) * height);
  
  for (let y = 0; y < height; y++) {
    rawData[y * (width * 4 + 1)] = 0; // filter byte
    for (let x = 0; x < width; x++) {
      const idx = y * (width * 4 + 1) + 1 + x * 4;
      rawData[idx] = 224;     // R
      rawData[idx + 1] = 85;  // G
      rawData[idx + 2] = 64;  // B
      rawData[idx + 3] = 255; // A
    }
  }
  
  const deflated = deflateSync(rawData);
  
  // Build PNG manually
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
  
  // IHDR
  const ihdrData = Buffer.alloc(13);
  ihdrData.writeUInt32BE(width, 0);
  ihdrData.writeUInt32BE(height, 4);
  ihdrData[8] = 8;  // bit depth
  ihdrData[9] = 6;  // color type: RGBA
  ihdrData[10] = 0; // compression
  ihdrData[11] = 0; // filter
  ihdrData[12] = 0; // interlace
  
  const crc32 = (buf) => {
    let crc = 0xFFFFFFFF;
    for (let i = 0; i < buf.length; i++) {
      crc ^= buf[i];
      for (let j = 0; j < 8; j++) {
        crc = (crc >>> 1) ^ (crc & 1 ? 0xEDB88320 : 0);
      }
    }
    return (crc ^ 0xFFFFFFFF) >>> 0;
  };
  
  function chunk(type, data) {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const typeB = Buffer.from(type, 'ascii');
    const crcBuf = Buffer.alloc(4);
    const crcData = Buffer.concat([typeB, data]);
    crcBuf.writeUInt32BE(crc32(crcData));
    return Buffer.concat([len, typeB, data, crcBuf]);
  }
  
  // IEND
  const iend = chunk('IEND', Buffer.alloc(0));
  
  return Buffer.concat([sig, chunk('IHDR', ihdrData), chunk('IDAT', deflated), iend]);
}

// Run synchronously since top-level await in a module script needs node --experimental-modules
import { createRequire } from 'module';
const require = createRequire(import.meta.url);

const sizes = {
  '32x32.png': 32,
  '128x128.png': 128,
  'icon.ico': 32,
  'icon.icns': 128,
};

for (const [name, size] of Object.entries(sizes)) {
  const png = createPNG(size);
  writeFileSync(new URL(name, import.meta.url).pathname, png);
  console.log(`Created ${name} (${png.length} bytes)`);
}
