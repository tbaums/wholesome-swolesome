// One-off icon renderer. Loads public/icon.svg into a headless Chromium
// at a fixed viewport, takes a screenshot, writes the PNG. Run as:
//   node scripts/render-icon.mjs
// Generates: public/icon-192.png, public/icon-512.png, public/apple-touch-icon.png (180×180).
import { chromium } from 'playwright';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, '..');
const svg = readFileSync(join(root, 'public/icon.svg'), 'utf8');

const targets = [
  { name: 'icon-512.png', size: 512 },
  { name: 'icon-192.png', size: 192 },
  { name: 'apple-touch-icon.png', size: 180 },
];

const browser = await chromium.launch();
for (const { name, size } of targets) {
  const page = await browser.newPage({
    viewport: { width: size, height: size },
    deviceScaleFactor: 1,
  });
  const html = `<!doctype html><html><head><style>
    html,body{margin:0;padding:0;background:transparent;width:${size}px;height:${size}px}
    svg{display:block;width:${size}px;height:${size}px}
  </style></head><body>${svg}</body></html>`;
  await page.setContent(html, { waitUntil: 'load' });
  const buf = await page.screenshot({
    omitBackground: true,
    clip: { x: 0, y: 0, width: size, height: size },
  });
  writeFileSync(join(root, 'public', name), buf);
  console.log(`wrote public/${name} (${buf.length} bytes)`);
  await page.close();
}
await browser.close();
