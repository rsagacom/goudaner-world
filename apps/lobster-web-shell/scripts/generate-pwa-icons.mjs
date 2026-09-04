// generate-pwa-icons.mjs — 用 16×16 像素画生成 PWA 图标（纯 Node，零依赖）。
// 画风对齐项目"像素风夜城 dark-on-dark"规范：深棕底 + 龙虾橙红主体。
// 运行：node scripts/generate-pwa-icons.mjs（从 apps/lobster-web-shell 目录）。
// 输出：assets/icons/icon-{192,512}.png 与 icon-maskable-512.png（确定性字节）。
// maskable 版安全区：画面整体缩到 80% 内缩留白，避免 Android 圆形裁切伤到主体。

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// 16×16 调色板：. = 背景，o = 深底纹，b = 身体，d = 身体暗部，c = 钳/高光，
// e = 眼白，k = 眼仁，w = 触须/尾扇高光
const PALETTE = {
  ".": [26, 18, 14, 255], // #1a120e 深棕底
  o: [43, 38, 34, 255], // 底部暗纹
  b: [217, 95, 43, 255], // #d95f2b 龙虾橙红
  d: [158, 58, 27, 255], // 暗部
  c: [240, 158, 96, 255], // 钳与高光
  e: [232, 222, 210, 255], // 眼白（对齐 UI 米白文字色）
  k: [22, 16, 12, 255], // 眼仁
  w: [232, 222, 210, 180], // 触须
};

const ART = [
  "................",
  ".....w....w.....",
  "..c..w....w..c..",
  ".ccc.w....w.ccc.",
  ".cccc..bb..cccc.",
  ".ccd..bbbb..dcc.",
  ".cc...bbbb...cc.",
  ".c..bbbbbbbb..c.",
  "....bbbbbbbb....",
  "....bbkbbkbb....",
  "....dbbbbbbd....",
  "...dbbbbbbbbd...",
  "...db.dddd.bd...",
  "...d..dddd..d...",
  "......dddd......",
  ".......oo.......",
];

function pixelGrid() {
  return ART.map((row) => [...row].map((ch) => PALETTE[ch] ?? PALETTE["."]));
}

function nearestNeighbor(grid, scale, { contentScale = 0.88 } = {}) {
  const base = grid.length;
  const size = base * scale;
  const canvas = new Uint8Array(size * size * 4);
  // 背景填充
  for (let i = 0; i < size * size; i++) {
    canvas.set(PALETTE["."], i * 4);
  }
  const drawScale = Math.max(1, Math.round(scale * contentScale));
  const drawSize = base * drawScale;
  const offset = Math.floor((size - drawSize) / 2);
  for (let y = 0; y < drawSize; y++) {
    for (let x = 0; x < drawSize; x++) {
      const source = grid[Math.min(base - 1, Math.floor(y / drawScale))][Math.min(base - 1, Math.floor(x / drawScale))];
      const target = (y + offset) * size + (x + offset);
      canvas.set(source, target * 4);
    }
  }
  return { canvas, size };
}

function crc32(buffer) {
  let c = ~0;
  for (const byte of buffer) {
    c ^= byte;
    for (let bit = 0; bit < 8; bit++) {
      c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
    }
  }
  return ~c >>> 0;
}

function pngChunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

function encodePng({ canvas, size }) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0; // filter none
    Buffer.from(canvas.buffer, y * size * 4, size * 4).copy(raw, y * (size * 4 + 1) + 1);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", deflateSync(raw, { level: 9 })),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

function pngSize(buffer) {
  return { width: buffer.readUInt32BE(16), height: buffer.readUInt32BE(20) };
}

const grid = pixelGrid();
const outDir = path.join(ROOT, "assets", "icons");
mkdirSync(outDir, { recursive: true });

const targets = [
  { file: "icon-192.png", scale: 12, contentScale: 0.88 },
  { file: "icon-512.png", scale: 32, contentScale: 0.88 },
  // maskable 安全区：内容缩到 80%，避免 Android 圆形裁切伤到主体
  { file: "icon-maskable-512.png", scale: 32, contentScale: 0.8 },
];

for (const target of targets) {
  const png = encodePng(nearestNeighbor(grid, target.scale, { contentScale: target.contentScale }));
  const { width, height } = pngSize(png);
  if (width !== height) throw new Error(`non-square icon: ${target.file}`);
  writeFileSync(path.join(outDir, target.file), png);
  console.log(`${target.file}: ${width}x${height}, ${png.length} bytes`);
}
