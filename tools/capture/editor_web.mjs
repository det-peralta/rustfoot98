// Drives the browser build of the team editor (web/editor.html) in headless Chromium and saves screenshots of the canvas.
// node tools/capture/editor_web.mjs out_prefix 'step;step...'  steps: click:x,y rclick:x,y dbl:x,y key:Name type:text wait:ms reload
// shot (saves out_prefix_N.png) teams (prints the names of the teams the editor saved in this browser)
import { createRequire } from 'module';
import http from 'http';
import fs from 'fs';
import path from 'path';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');
const [prefix = 'analysis/shots/web_editor', script = 'shot'] = process.argv.slice(2);
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
const p = await (await b.newContext({ viewport: { width: 800, height: 640 } })).newPage();
p.on('console', (m) => console.log('console:', m.text().slice(0, 200)));
p.on('pageerror', (e) => console.log('pageerror:', e.message));
await p.goto(`http://127.0.0.1:${port}/editor.html`);
await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 }).catch(() => console.log('no canvas'));
const canvas = () => p.$('canvas');
const at = async (x, y) => { const r = await (await canvas()).boundingBox(); return [r.x + x * r.width / 800, r.y + y * r.height / 600]; };
let n = 0;
for (const step of script.split(';').filter(Boolean)) {
  const [k, v = ''] = step.split(/:(.*)/s);
  if (k === 'click' || k === 'rclick' || k === 'dbl') {
    const [x, y] = await at(...v.split(',').map(Number));
    if (k === 'dbl') await p.mouse.dblclick(x, y);
    else await p.mouse.click(x, y, { button: k === 'rclick' ? 'right' : 'left' });
  } else if (k === 'key') await p.keyboard.press(v);
  else if (k === 'type') await p.keyboard.type(v);
  else if (k === 'wait') await p.waitForTimeout(Number(v));
  else if (k === 'reload') { await p.reload(); await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 }); }
  else if (k === 'teams') {
    console.log('teams:', await p.evaluate(() => import('./pack.js').then((m) => m.editedTeams()).then((t) => t.map((x) => x.name).join(','))));
  } else if (k === 'shot') { await p.waitForTimeout(150); await (await canvas()).screenshot({ path: `${prefix}_${n++}.png` }); }
  await p.waitForTimeout(80);
}
await b.close();
srv.close();
