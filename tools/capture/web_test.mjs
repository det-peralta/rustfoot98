// Smoke test of the browser demo: node tools/capture/web_test.mjs  (serves web/ on :8099)
import { createRequire } from 'module';
import { spawn } from 'child_process';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');
const srv = spawn('python3', ['-m', 'http.server', '8099', '--bind', '127.0.0.1'], { cwd: 'web', stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 800));
const errors = [];
let ok = true;
const check = (name, cond, extra = '') => { console.log((cond ? 'PASS ' : 'FAIL ') + name + (extra ? ' — ' + extra : '')); if (!cond) ok = false; };
try {
  const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
  const p = await (await b.newContext({ viewport: { width: 1180, height: 900 } })).newPage();
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  p.on('pageerror', (e) => errors.push(String(e)));
  await p.goto('http://127.0.0.1:8099/', { waitUntil: 'networkidle' });
  await p.waitForSelector('#table tbody tr');
  check('loads year 1998', (await p.textContent('#year')) === '1998');
  check('8 rows in table', (await p.locator('#table tbody tr').count()) === 8);
  check('4 division tabs', (await p.locator('#tabs button').count()) === 4);
  await p.click('#play-round');
  check('round 1 played', (await p.textContent('#round')).startsWith('1 /'));
  check('4 results shown', (await p.locator('#results li').count()) === 4);
  await p.locator('#results li button').first().click();
  check('match detail opens', await p.locator('#match').isVisible());
  await p.screenshot({ path: 'analysis/shots/web_round1.png', fullPage: true });
  await p.click('#tabs button:nth-child(4)');
  check('4th division tab selected', (await p.getAttribute('#tabs button:nth-child(4)', 'aria-selected')) === 'true');
  await p.click('#play-season');
  check('season rolled over', (await p.textContent('#year')) === '1999' && (await p.textContent('#round')).startsWith('0 /'));
  check('season summary visible', await p.locator('#season-card').isVisible());
  const moves = await p.locator('.moves li').count();
  check('16 promotion/relegation lines', moves === 12 + 4 || moves === 16, String(moves));
  await p.screenshot({ path: 'analysis/shots/web_season.png', fullPage: true });
  // determinism: same seed -> same first table
  await p.fill('#seed', '42'); await p.click('#new-world'); await p.click('#play-round');
  const a = await p.textContent('#table tbody');
  await p.click('#new-world'); await p.click('#play-round');
  const c = await p.textContent('#table tbody');
  check('same seed gives the same round', a === c);
  // real teams: upload the shipped .EFT files when they exist locally (they are git-ignored)
  const fs = await import('fs');
  const dir = 'original/elifoot98/EQUIPAS';
  if (fs.existsSync(dir)) {
    const files = fs.readdirSync(dir).sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase())).map((f) => dir + '/' + f);
    await p.setInputFiles('#eft', files);
    await p.waitForFunction(() => document.querySelector('#eft-status').textContent.includes('reais carregadas'), null, { timeout: 15000 });
    const names = await p.locator('#table tbody td.l').allTextContents();
    check('real teams in the table', names.length === 8 && names.every((n) => !/^(Atlético|Sporting|União|Académica|Desportivo|Estrela) /.test(n)), names.join(', '));
    await p.click('#play-season');
    check('real season plays', (await p.textContent('#year')) === '1999');
    await p.screenshot({ path: 'analysis/shots/web_real.png', fullPage: true });
    await p.click('#use-fiction');
    check('back to fiction', (await p.locator('#table tbody td.l').first().textContent()).match(/^(Atlético|Sporting|União|Académica|Desportivo|Estrela) /) !== null);
  }
  check('no console errors', errors.length === 0, errors.join(' | '));
  await b.close();
} finally { srv.kill(); }
process.exit(ok ? 0 : 1);
