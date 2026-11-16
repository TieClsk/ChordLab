// Serves the repo with cross-origin isolation headers (needed for SharedArrayBuffer)
// and runs test.html in headless Chromium.
//   node experiments/browser-worklet/run.mjs <repo root> <build dir with engine_wasm.wasm + native.f32> <chromium-launcher dir>
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const [root, buildDir, launcherDir] = process.argv.slice(2);
const require = createRequire(path.join(launcherDir, 'package.json'));
const { chromium } = require('playwright-core');
const sparticuz = require('@sparticuz/chromium').default ?? require('@sparticuz/chromium');

const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.f32': 'application/octet-stream' };
const server = http.createServer((req, res) => {
  const u = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  const file = u.startsWith('/build/') ? path.join(buildDir, u.slice(7)) : path.join(root, u);
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { 'Content-Type': types[path.extname(file)] || 'application/octet-stream', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' });
    res.end(data);
  });
}).listen(0);
const port = server.address().port;

// Drop the Lambda-oriented flags that weaken isolation (they disable crossOriginIsolated).
const args = [...sparticuz.args.filter((a) => !/site-isolation|single-process|no-zygote|disable-web-security|disable-features|enable-features=SharedArrayBuffer/.test(a)), '--autoplay-policy=no-user-gesture-required', ...(process.env.FAKE_AUDIO ? ['--disable-audio-output'] : [])];
console.log('chromium args', args.join(' '));
const browser = await chromium.launch({ executablePath: await sparticuz.executablePath(), args, headless: true });
const page = await browser.newPage();
page.on('console', (m) => console.log('[page]', m.text()));
page.on('pageerror', (e) => console.log('[pageerror]', e.message));
console.log('browser', browser.version());
await page.goto(`http://localhost:${port}/experiments/browser-worklet/test.html`);
const results = await page.waitForFunction(() => window.__results, null, { timeout: 60000 }).then((h) => h.jsonValue());
console.log(JSON.stringify(results, null, 1));
await browser.close();
server.close();
