// Plays whole seasons of the browser build (web/classic.html) in headless Chromium with a fake clock, driven by the game's state
// (cl_status: year, round, screen, top dialog), then checks the autosave, a reload that loads the game back, and Sair.
// node tools/capture/classic_season.mjs out_prefix [seasons] [humans]
import { createRequire } from 'module';
import http from 'http';
import fs from 'fs';
import path from 'path';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');
const [prefix = '/tmp/web_season', seasonsArg = '1', humansArg = '1'] = process.argv.slice(2);
const seasons = Number(seasonsArg), humans = Number(humansArg);
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
const errors = [];
p.on('console', (m) => { if (m.type() === 'error') errors.push(m.text().slice(0, 300)); });
p.on('pageerror', (e) => errors.push('pageerror: ' + e.message));
await p.clock.install();
const run = async (ms) => { await p.clock.runFor(ms); };
// the module keeps its wasm instance private: reach it through a second instance-less call of the exports the page uses
const status = async () => p.evaluate(() => window.__status ? window.__status() : '');
await p.addInitScript(() => {
  const orig = WebAssembly.instantiate;
  WebAssembly.instantiate = async (...a) => {
    const r = await orig(...a);
    const w = r.instance.exports;
    if (w.cl_status) {
      window.__status = () => { w.cl_status(); return new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.cl_out_ptr(), w.cl_out_len())); };
    }
    return r;
  };
});
await p.goto(`http://127.0.0.1:${port}/classic.html`);
await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 });
const shot = (n) => p.screenshot({ path: `${prefix}_${n}.png` });
// About -> new game "TESTE" -> the players' names -> the dialogs of a new game
await run(1500); await p.keyboard.press('Enter'); await run(400);
console.log('select', await status());
await p.mouse.click(456, 224); await p.keyboard.type('teste'); await run(200);
console.log('typed', await status()); await shot('s1');
await p.keyboard.press('Enter'); await run(1500);
let named = false;
let st = '', last = '', same = 0, steps = 0;
const start = (await status()).split(' ');
console.log('start', start.join(' '));
const target = Number(start[0]) + seasons;
while (steps < 200000) {
  steps++;
  st = await status();
  const [year, round, started, screen, ...top] = st.split(' ');
  const dlg = top.join(' ');
  if (Number(year) >= target && round === '0' && screen === 'main' && dlg === '-') break;
  if (st === last) same++; else { same = 0; last = st; }
  // (a cup day can show many shoot-outs in a row, each 13 s or more, and the status line does not change between them)
  if (same > (st.endsWith('Shootout') ? 4000 : 400)) { console.log('STUCK', st); await shot('stuck'); break; }
  if (screen === 'round' && dlg === '-') { await run(500); continue; }
  // a new game: every country (Todas), then OK
  if (dlg === 'SelectCountries') { await p.mouse.click(507, 130); await run(200); await p.mouse.click(600, 130); await run(800); continue; }
  if (dlg === 'CoachName') {
    // the names once; Enter in a name moves to the button, which draws the clubs and then becomes OK
    if (!named) {
      for (let i = 1; i <= humans; i++) { await p.keyboard.type('joga' + i); if (i < humans) await p.keyboard.press('Tab'); }
      named = true;
    }
    await p.keyboard.press('Enter'); await run(800); continue;
  }
  if (started === 'false' && dlg === '-') { await run(500); continue; }
  // the cup draw runs by itself (1.5 s per tie while a human is still in the pot)
  if (dlg === 'CupDraw') { await run(3000); await p.keyboard.press('Enter'); continue; }
  // a human's penalty: a shooter first (the list's first row), then Chutar / the goalkeeper's side
  if (dlg === 'Penalty') { await p.mouse.click(300, 279); await run(200); await p.keyboard.press('Enter'); await run(600); continue; }
  if (dlg === 'ForcedSubstitution') { await p.mouse.click(300, 312); await p.keyboard.press('Enter'); await run(300); continue; }
  if (dlg === 'Auction') { await p.keyboard.press('Escape'); await run(300); continue; }
  if (dlg !== '-') { await p.keyboard.press('Enter'); await run(300); continue; }
  // a main window: automatic line-up, then play
  await p.keyboard.press('a'); await p.keyboard.press('Enter'); await run(300);
  if (steps % 50 === 0) console.log(steps, st);
}
console.log('end', await status(), 'steps', steps);
await shot('end');
const saves = await p.evaluate(() => Object.keys(localStorage).filter((k) => k.startsWith('elifoot98-classic:')));
console.log('saves', saves);
// reload, load the saved game, check where it is
await p.reload(); await p.waitForSelector('canvas', { state: 'visible', timeout: 20000 });
await run(1500); await p.keyboard.press('Enter'); await run(600);
await p.mouse.click(198, 278); await run(200); await p.mouse.click(390, 307); await run(200); await p.keyboard.press('Enter'); await run(3000);
console.log('after load', await status());
await shot('loaded');
console.log('errors', errors.length ? errors : 'none');
await b.close(); srv.close();
