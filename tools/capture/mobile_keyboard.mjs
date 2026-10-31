// The phone keyboard of the game page (web/softkb.js) on an emulated phone: a tap on "Quero começar um jogo do início" gives
// the focus to the file name field, so the hidden input must take the focus (a phone then shows its keyboard); text typed the
// way an on-screen keyboard sends it (input events, an Android-style composition, a deletion) reaches the game; Enter goes on
// to the countries dialog and the input lets the focus go.
//   node tools/capture/mobile_keyboard.mjs [out.png]
import { createRequire } from 'module';
import http from 'http';
import fs from 'fs';
import path from 'path';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');

const out = process.argv[2] || '/tmp/mobile_keyboard.png';
const root = path.resolve('web');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const srv = http.createServer((req, res) => {
  const f = path.join(root, decodeURIComponent(req.url.split('?')[0]).replace(/\/$/, '/index.html'));
  if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream' });
  fs.createReadStream(f).pipe(res);
}).listen(0);
const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
const ctx = await b.newContext({ viewport: { width: 800, height: 600 }, isMobile: true, hasTouch: true, deviceScaleFactor: 1 });
const p = await ctx.newPage();
const errors = [];
p.on('pageerror', (e) => errors.push(e.message));
await p.goto(`http://127.0.0.1:${srv.address().port}/classic.html`);
await p.waitForSelector('canvas', { state: 'visible', timeout: 60000 });
await p.waitForTimeout(500);
const tap = async (x, y) => {
  const r = await p.evaluate(() => { const r = document.getElementById('c').getBoundingClientRect(); return [r.left, r.top, r.width / 800]; });
  await p.touchscreen.tap(r[0] + x * r[2], r[1] + y * r[2]);
  await p.waitForTimeout(150);
};
const focused = () => p.evaluate(() => document.activeElement.tagName + (document.activeElement.ariaLabel ? ':' + document.activeElement.ariaLabel : ''));
const fail = [];
const expect = (what, got, want) => { console.log(what, got); if (got !== want) fail.push(`${what}: ${got}, expected ${want}`); };

await tap(614, 208); // the About box's OK
await p.waitForTimeout(300);
expect('focus on the choice of game', await focused(), 'CANVAS:Elifoot 98');
await tap(196, 183); // "Quero começar um jogo do início"
await p.locator('canvas').screenshot({ path: out.replace(/\.png$/, '_tap.png') });
expect('focus after the tap on a new game', await focused(), 'INPUT:Teclado');
await p.keyboard.insertText('NOVX');
// a deletion as Android's keyboards send it: the input's text one character shorter, no Backspace key
await p.evaluate(() => { const i = document.activeElement; i.value = i.value.slice(0, -1); i.dispatchEvent(new InputEvent('input', { inputType: 'deleteContentBackward' })); });
// a word composed letter by letter (Android): each step replaces the composed text
await p.evaluate(() => {
  const i = document.activeElement, base = i.value;
  i.dispatchEvent(new CompositionEvent('compositionstart'));
  for (const w of ['o', 'oX', 'o']) { i.value = base + w; i.dispatchEvent(new InputEvent('input', { isComposing: true, inputType: 'insertCompositionText' })); }
  i.dispatchEvent(new CompositionEvent('compositionend'));
});
await p.waitForTimeout(300);
await p.locator('canvas').screenshot({ path: out });
await p.keyboard.press('Enter');
await p.waitForTimeout(500);
expect('focus after Enter (the countries dialog)', await focused(), 'CANVAS:Elifoot 98');
await p.locator('canvas').screenshot({ path: out.replace(/\.png$/, '_next.png') });
await b.close();
srv.close();
if (errors.length) fail.push('page errors: ' + errors.join('; '));
console.log(fail.length ? 'FAIL\n' + fail.join('\n') : 'ok: ' + out);
process.exit(fail.length ? 1 : 0);
