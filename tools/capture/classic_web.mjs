// Drives the browser build of the classic interface in headless Chromium and saves screenshots of the canvas.
// node tools/capture/classic_web.mjs out_prefix 'step;step...'  steps: click:x,y key:Name type:text wait:ms reload shot info:NAME (opens one of the information dialogs
// through the menu bar, saves a shot and reports whether the screen changed; NAME is a key of INFO below)
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
await p.goto(`http://127.0.0.1:${port}/classic.html`);
await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 }).catch(() => console.log('no canvas'));
// the menu bar (top-level x) and the entries of the information dialogs (y of the entry in the drop-down: 110 + 18 per entry, 7 per rule)
const TOP = { equipa: 265, jogador: 340, campeonato: 410, treinador: 497 };
const INFO = {
  standings: ['campeonato', 110], scorers: ['campeonato', 128], calendar: ['campeonato', 146], winners: ['campeonato', 171],
  alltime: ['campeonato', 189], history: ['equipa', 128], transfers: ['jogador', 171], acquisitions: ['jogador', 189],
  coachhistory: ['treinador', 110], manager: ['treinador', 128], ranking: ['treinador', 146], profile: ['treinador', 171],
};
let n = 0;
for (const step of script.split(';')) {
  const [k, a = ''] = step.split(':');
  if (k === 'click') { const [x, y] = a.split(',').map(Number); await p.mouse.click(x, y); }
  else if (k === 'move') { const [x, y] = a.split(',').map(Number); await p.mouse.move(x, y); }
  else if (k === 'key') await p.keyboard.press(a);
  else if (k === 'type') await p.keyboard.type(a);
  else if (k === 'reload') { await p.reload(); await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 }); }
  else if (k === 'info') {
    const [top, y] = INFO[a];
    const before = await p.screenshot();
    await p.mouse.click(TOP[top], 90);
    await p.mouse.click(TOP[top] + 60, y);
    await p.waitForTimeout(200);
    const after = await p.screenshot({ path: `${prefix}${n++}.png` });
    console.log(`info:${a}`, Buffer.compare(before, after) === 0 ? 'FAILED (the screen did not change)' : 'opened');
  }
  else if (k === 'wait') await p.waitForTimeout(Number(a));
  else if (k === 'shot') await p.screenshot({ path: `${prefix}${n++}.png` });
}
await b.close(); srv.close();
