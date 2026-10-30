// Smoke test of the browser build: starts a game and plays rounds like a blind user (Enter, a click on the forced-substitution list) with a fake clock.
// node tools/capture/classic_smoke.mjs out_prefix x [rounds]
import { createRequire } from 'module';
import http from 'http';
import fs from 'fs';
import path from 'path';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');
const [prefix = 'analysis/shots/web_classic', script = 'shot'] = process.argv.slice(2);
const root = path.resolve('web');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const srv = http.createServer((req, res) => {
  const f = path.join(root, decodeURIComponent(req.url.split('?')[0]).replace(/\/$/, '/index.html'));
  if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream' });
  fs.createReadStream(f).pipe(res);
}).listen(0);
const port = srv.address().port;
const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
const p = await (await b.newContext({ viewport: { width: 800, height: 600 } })).newPage();
p.on('console', (m) => console.log('console:', m.text().slice(0, 200)));
p.on('pageerror', (e) => console.log('pageerror:', e.message));
await p.clock.install();
await p.goto(`http://127.0.0.1:${port}/classic.html`);
await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 });
const run = async (ms) => { await p.clock.runFor(ms); };
await run(1500); await p.keyboard.press('Enter'); await run(400);
await p.mouse.click(456, 224); await p.keyboard.type('test'); await p.keyboard.press('Tab'); await p.keyboard.press('Tab'); await p.keyboard.press('Enter'); await run(1500);
await p.keyboard.type('joga1'); await p.keyboard.press('Enter'); await run(500); await p.keyboard.press('Enter'); await run(5000);
await p.keyboard.press('Enter'); await run(1500);
const rounds = Number(process.argv[4] || 4); let shot = 0;
for (let r = 0; r < rounds; r++) {
  // A (Automático): a formation the squad can play (Jogar waits for one, as in the original)
  await p.keyboard.press('a'); await p.keyboard.press('Enter');
  for (let i = 0; i < 400; i++) {
    await run(2000);
    await p.mouse.click(300, 312); await p.mouse.click(550, 360); await p.keyboard.press('Enter');
    if (i % 12 == 0) await p.screenshot({ path: `${prefix}_r${r}_${shot++}.png` });
  }
}
await p.screenshot({ path: `${prefix}_end.png` });
await b.close(); srv.close();
