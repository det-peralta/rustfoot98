// Long-running driver: loads the reference web build in headless Chromium (fresh profile) and executes
// JSON commands appended to analysis/cmd.jsonl. Results go to analysis/cmd.out.
import { createRequire } from 'module';
import fs from 'fs';
const require = createRequire('/home/fgf/.local/share/mise/installs/npm-playwright/latest/node_modules/');
const { chromium } = require('playwright');
const URL = process.env.ELI_URL || 'https://www.elifoot98.com.br/#/game';
const b = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true, args: ['--no-sandbox', '--use-gl=swiftshader'] });
const ctx = await b.newContext({ viewport: { width: 1100, height: 800 } });
const p = await ctx.newPage();
p.on('console', m => fs.appendFileSync('analysis/console.log', m.text().slice(0, 300) + '\n'));
await p.goto(URL, { waitUntil: 'domcontentloaded' });
fs.writeFileSync('analysis/cmd.jsonl', ''); fs.writeFileSync('analysis/cmd.out', 'ready\n');
let done = 0;
const out = s => fs.appendFileSync('analysis/cmd.out', s + '\n');
while (true) {
  const lines = fs.readFileSync('analysis/cmd.jsonl', 'utf8').split('\n').filter(Boolean);
  while (done < lines.length) {
    const c = JSON.parse(lines[done++]);
    try {
      if (c.op === 'shot') { await p.screenshot({ path: c.path }); out('shot ' + c.path); }
      else if (c.op === 'click') { await p.mouse.click(c.x, c.y, { clickCount: c.n || 1 }); out('click'); }
      else if (c.op === 'move') { await p.mouse.move(c.x, c.y); out('move'); }
      else if (c.op === 'key') { await p.keyboard.press(c.key, { delay: 150 }); await p.waitForTimeout(700); out('key ' + c.key); }
      else if (c.op === 'type') { await p.keyboard.type(c.text, { delay: 200 }); out('type'); }
      else if (c.op === 'wait') { await p.waitForTimeout(c.ms); out('waited'); }
      else if (c.op === 'eval') { out('eval ' + JSON.stringify(await p.evaluate(c.js))); }
      else if (c.op === 'quit') { out('bye'); await b.close(); process.exit(0); }
    } catch (e) { out('ERR ' + e.message.slice(0, 200)); }
  }
  await new Promise(r => setTimeout(r, 300));
}
