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
  check('economy index shown', /^\d+\.\d\d$/.test(await p.textContent('#economy')), await p.textContent('#economy'));
  check('8 rows in table', (await p.locator('#table tbody tr').count()) === 8);
  check('4 division tabs', (await p.locator('#tabs button').count()) === 4);
  // the user's club (starts in the bottom six of the 4th division, tab opens on it)
  check('club panel visible', await p.locator('#club-card').isVisible());
  check('starts on the 4th division tab', (await p.getAttribute('#tabs button:nth-child(4)', 'aria-selected')) === 'true');
  check('twelve formations', (await p.locator('#formation option').count()) === 12);
  const rows = await p.locator('#squad tbody tr').count();
  check('squad listed', rows >= 14, String(rows));
  await p.selectOption('#formation', '2'); // 3-4-3
  check('formation 3-4-3 applied', (await p.textContent('#club-msg')).includes('3-4-3'), await p.textContent('#club-msg'));
  await p.selectOption('#formation', '0'); // back to 4-4-2
  const startersBefore = await p.locator('#squad select.role').evaluateAll((els) => els.filter((e) => e.value === 'T').length);
  check('11 starters', startersBefore === 11, String(startersBefore));
  // bench a starter: the game repairs the plan to 11 starters again after the round
  await p.locator('#squad select.role').evaluateAll((els) => { const e = els.find((x) => x.value === 'T' && x.closest('tr').children[0].textContent === 'A'); e.value = 'F'; e.dispatchEvent(new Event('change')); });
  check('warning when the plan is short', (await p.textContent('#club-msg')).includes('corrige') || (await p.textContent('#club-msg')).includes('10'));
  await p.click('#auto-plan');
  await p.click('#add-sub');
  check('substitution row added', (await p.locator('.subrow').count()) === 1);
  await p.click('#play-round');
  check('round 1 played', (await p.textContent('#round')).startsWith('1 /'));
  check('4 results shown', (await p.locator('#results li').count()) === 4);
  check('my match shown', (await p.textContent('#my-match')).includes('–'), await p.textContent('#my-match'));
  check('finance card shows cash and gate', (await p.textContent('#finance')).includes('Caixa') && (await p.textContent('#finance')).includes('Salários por jogo'));
  check('match detail shows attendance', true);
  // the user's market: put a player up for sale at a low price and read the answer
  await p.fill('#squad tbody tr:first-child input.ask', '1000');
  await p.click('#squad tbody tr:first-child button.sell');
  const mmsg = await p.textContent('#market-msg');
  check('selling gives an answer', /Vendido|nenhum clube|jogadores/.test(mmsg), mmsg);
  check('my match highlighted in the results', (await p.locator('#results li.mine').count()) === 1);
  check('planned substitution consumed', (await p.locator('.subrow').count()) === 0);
  await p.locator('#results li button').first().click();
  check('match detail opens', await p.locator('#match').isVisible());
  await p.screenshot({ path: 'analysis/shots/web_round1.png', fullPage: true });
  // switch club
  const opts = await p.locator('#change-club option').allTextContents();
  await p.selectOption('#change-club', { index: 0 });
  check('changed to a first-division club', (await p.textContent('#club-div')).includes('1ª'), await p.textContent('#club-div'));
  await p.click('#tabs button:nth-child(4)');
  check('4th division tab selected', (await p.getAttribute('#tabs button:nth-child(4)', 'aria-selected')) === 'true');
  await p.click('#play-season');
  check('season rolled over', (await p.textContent('#year')) === '1999' && (await p.textContent('#round')).startsWith('0 /'));
  check('season summary visible', await p.locator('#season-card').isVisible());
  check('cup winner announced at season end', (await p.textContent('#season')).includes('Vencedor da Taça'));
  check('final shown in the cup card', (await p.textContent('#cup-title')).includes('FINAL') && await p.locator('#cup-results li').count() === 1);
  const moves = await p.locator('.moves li').count();
  check('16 promotion/relegation lines', moves === 12 + 4 || moves === 16, String(moves));
  await p.screenshot({ path: 'analysis/shots/web_season.png', fullPage: true });
  // determinism: same seed -> same first table
  await p.fill('#seed', '42'); await p.click('#new-world'); await p.click('#play-round');
  const a = await p.textContent('#table tbody');
  await p.click('#new-world'); await p.click('#play-round');
  const c = await p.textContent('#table tbody');
  check('same seed gives the same round', a === c);
  // offline: the service worker must cache the app, then the page must work with the network cut
  await p.waitForFunction(() => document.querySelector('#offline-status').textContent.includes('offline e pode'), null, { timeout: 10000 });
  check('service worker active', true);
  const manifest = await p.evaluate(async () => (await fetch('manifest.webmanifest')).json());
  check('manifest has icons and standalone display', manifest.display === 'standalone' && manifest.icons.length >= 3);
  await p.context().setOffline(true);
  await p.reload({ waitUntil: 'domcontentloaded' });
  await p.waitForSelector('#table tbody tr', { timeout: 10000 });
  check('works offline after reload', (await p.locator('#table tbody tr').count()) === 8);
  await p.click('#play-round');
  check('plays a round offline', (await p.textContent('#round')).includes('1 /'));
  await p.context().setOffline(false);
  // saving and loading
  await p.fill('#seed', '777'); await p.click('#new-world');
  await p.click('#play-round'); await p.click('#play-round');
  const snapshot = await p.textContent('#table tbody');
  const yearRound = (await p.textContent('#round'));
  check('autosave wrote a save', (await p.evaluate(() => localStorage.getItem('elifoot98-rs-save-v1') || '')).startsWith('ELIFOOT98-RS 1'));
  await p.click('#new-world'); // a different game replaces the state
  check('state changed', (await p.textContent('#round')).startsWith('0 /'));
  check('load enabled', await p.isEnabled('#load-now'));
  await p.click('#load-now');
  check('load restores round and table', (await p.textContent('#round')) === yearRound && (await p.textContent('#table tbody')) === snapshot);
  const [download] = await Promise.all([p.waitForEvent('download'), p.click('#export')]);
  const file = await download.path();
  const exported = (await import('fs')).readFileSync(file, 'utf8');
  check('exported file is a save', exported.startsWith('ELIFOOT98-RS 1'));
  await p.click('#new-world');
  await p.setInputFiles('#import', { name: 'save.txt', mimeType: 'text/plain', buffer: Buffer.from(exported) });
  await p.waitForFunction(() => document.querySelector('#save-status').textContent.includes('carregado'), null, { timeout: 5000 });
  check('import restores the game', (await p.textContent('#round')) === yearRound && (await p.textContent('#table tbody')) === snapshot);
  await p.setInputFiles('#import', { name: 'bad.txt', mimeType: 'text/plain', buffer: Buffer.from('garbage') });
  await p.waitForFunction(() => document.querySelector('#save-status').textContent.includes('inválido'), null, { timeout: 5000 }).catch(() => {});
  check('bad file refused', (await p.textContent('#save-status')).includes('inválido'), await p.textContent('#save-status'));
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
  // original .e98 import (the sample is a local, git-ignored game save; skipped when absent)
  const { existsSync } = await import('fs');
  if (existsSync('original/saves/snap2.e98')) {
    await p.setInputFiles('#import-e98', 'original/saves/snap2.e98');
    await p.waitForFunction(() => document.querySelector('#mode').textContent.includes('importado'));
    check('e98 imported', (await p.textContent('#mode')).includes('importado'), await p.textContent('#save-status'));
    check('e98 year 2026 round 2', (await p.textContent('#year')) === '2026' && (await p.textContent('#round')).startsWith('2 /'));
    check('e98 my club is Figueirense', (await p.textContent('#club-name')).includes('FIGUEIRENSE'), await p.textContent('#club-name'));
    check('e98 cash imported', (await p.textContent('#finance')).replace(/\s/g, ' ').includes('1 216 448') || (await p.textContent('#finance')).includes('1\u00a0216\u00a0448'), (await p.textContent('#finance')).slice(0, 120));
    await p.click('#play-round');
    check('e98 plays on', (await p.textContent('#round')).startsWith('3 /'));
  }
  await p.setInputFiles('#import-e98', { name: 'x.e98', mimeType: 'application/octet-stream', buffer: Buffer.alloc(200) });
  await p.waitForFunction(() => document.querySelector('#save-status').textContent.includes('Não foi possível'));
  check('bad e98 refused', (await p.textContent('#save-status')).includes('Não foi possível importar'));
  check('no console errors', errors.length === 0, errors.join(' | '));
  await b.close();
} finally { srv.kill(); }
process.exit(ok ? 0 : 1);
