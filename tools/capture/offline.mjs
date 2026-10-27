// Checks offline use of the browser build: a first visit registers the service worker (sw.js); then, with the network off, the game,
// the team editor and the root page still load and draw their canvas. node tools/capture/offline.mjs
import { createRequire } from 'module';
import http from 'http';
import fs from 'fs';
import path from 'path';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');
const root = path.resolve('web');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.webmanifest': 'application/manifest+json', '.svg': 'image/svg+xml', '.png': 'image/png' };
const srv = http.createServer((req, res) => {
  const f = path.join(root, decodeURIComponent(req.url.split('?')[0]).replace(/\/$/, '/index.html'));
  if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream' });
  fs.createReadStream(f).pipe(res);
}).listen(0);
const base = `http://localhost:${srv.address().port}/`;
const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
const ctx = await b.newContext({ viewport: { width: 800, height: 600 } });
const p = await ctx.newPage();
const errors = [];
p.on('pageerror', (e) => errors.push(e.message));
let ok = true;
const check = async (what, url) => {
  await p.goto(base + url);
  const drawn = await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 }).then(() => true).catch(() => false);
  console.log(what, drawn ? 'canvas shown' : 'NO CANVAS: ' + (await p.evaluate(() => document.body.innerText.slice(0, 200))));
  ok &&= drawn;
};
await check('online  classic', 'classic.html');
await p.evaluate(() => navigator.serviceWorker.ready);
console.log('controlled after ready:', await p.evaluate(() => !!navigator.serviceWorker.controller));
const cached = await p.evaluate(async () => { const k = await caches.keys(); const c = await caches.open(k[0]); return [k, (await c.keys()).length]; });
console.log('caches', JSON.stringify(cached));
await ctx.setOffline(true);
await check('offline classic', 'classic.html');
await check('offline editor ', 'editor.html');
await check('offline root   ', '');
console.log('errors', errors.length ? errors : 'none');
await b.close(); srv.close();
process.exit(ok && !errors.length ? 0 : 1);
