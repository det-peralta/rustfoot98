// Host of the pixel-exact classic interface (ui/src/classic) compiled to WebAssembly (ui-web/src/classic_host.rs).
// The skin pack (fonts, glyphs, forms, team files: built from the user's own copy of the game by tools/ui/webpack.py) is read from
// ./skin/ or, failing that, from a folder the user picks (kept in IndexedDB).
const canvas = document.getElementById("c");
const ctx = canvas.getContext("2d");
const msg = document.getElementById("msg");
const SAVE_PREFIX = "elifoot98-classic:";

const say = (html) => { msg.innerHTML = html; msg.style.display = "block"; canvas.style.display = "none"; };

async function wasmInstance() {
  const res = await fetch("elifoot_ui.wasm");
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  return instance.exports;
}

// ---- the pack: name -> bytes ----
const DB = "elifoot98-classic-pack";
const idb = () => new Promise((ok, no) => {
  const r = indexedDB.open(DB, 1);
  r.onupgradeneeded = () => r.result.createObjectStore("files");
  r.onsuccess = () => ok(r.result);
  r.onerror = () => no(r.error);
});
async function dbGet() {
  try {
    const db = await idb();
    return await new Promise((ok) => {
      const q = db.transaction("files").objectStore("files").get("pack");
      q.onsuccess = () => ok(q.result || null);
      q.onerror = () => ok(null);
    });
  } catch { return null; }
}
async function dbPut(pack) {
  try {
    const db = await idb();
    await new Promise((ok) => {
      const t = db.transaction("files", "readwrite");
      t.objectStore("files").put(pack, "pack");
      t.oncomplete = ok; t.onerror = ok;
    });
  } catch { /* the pack is only a convenience */ }
}

async function fetchPack() {
  const idx = await fetch("skin/index.json");
  if (!idx.ok) return null;
  const list = await idx.json();
  const get = async (n) => new Uint8Array(await (await fetch("skin/" + n)).arrayBuffer());
  const pack = { fonts: await get(list.fonts), glyphs: await get(list.glyphs), forms: await get(list.forms),
                 country: await get(list.country), referee: await get(list.referee), teams: [] };
  for (const t of list.teams || []) pack.teams.push(await get(t));
  return pack;
}

async function packFromFiles(files) {
  const by = {};
  const teams = [];
  for (const f of files) {
    const n = f.name.toLowerCase();
    const bytes = new Uint8Array(await f.arrayBuffer());
    if (n === "fonts.bin") by.fonts = bytes; else if (n === "glyphs.bin") by.glyphs = bytes;
    else if (n === "forms.json") by.forms = bytes; else if (n === "country.txe") by.country = bytes;
    else if (n === "referee.txe") by.referee = bytes; else if (n.endsWith(".eft")) teams.push(bytes);
  }
  if (!(by.fonts && by.glyphs && by.forms && by.country && by.referee)) return null;
  return { ...by, teams };
}

function askForPack() {
  say(`<h2>Elifoot 98</h2>
    <p>Para jogar com o aspecto exacto do original é preciso o <b>pacote de aspecto</b>, feito a partir da tua própria cópia do jogo:
    <code>python tools/ui/skin.py</code> e <code>python tools/ui/webpack.py</code> (cria <code>web/skin/</code>).</p>
    <p>Ou escolhe aqui a pasta <code>web/skin</code> (os ficheiros ficam só neste navegador):</p>
    <label class="btn"><input id="pick" type="file" webkitdirectory multiple hidden>Escolher pasta…</label>`);
  document.getElementById("pick").addEventListener("change", async (e) => {
    const pack = await packFromFiles([...e.target.files]);
    if (!pack) { say("Pasta inválida: faltam <code>fonts.bin</code>, <code>glyphs.bin</code>, <code>forms.json</code>, <code>COUNTRY.TXE</code> ou <code>REFEREE.TXE</code>."); setTimeout(askForPack, 3000); return; }
    await dbPut(pack);
    run(pack);
  });
}

// ---- the game ----
let w, mem;
function put(kind, bytes) {
  const p = w.cl_alloc(bytes.length);
  new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes);
  return w.cl_put(kind);
}
function out() { return new Uint8Array(w.memory.buffer, w.cl_out_ptr(), w.cl_out_len()).slice(); }
const text = (b) => new TextDecoder().decode(b);
const savedNames = () => Object.keys(localStorage).filter((k) => k.startsWith(SAVE_PREFIX)).map((k) => k.slice(SAVE_PREFIX.length));
// one line per game: its name, a tab and the first line of its text (the header that describes it)
const pushSaves = () => put(7, new TextEncoder().encode(savedNames().map((n) => {
  let head = "";
  try { head = (localStorage.getItem(SAVE_PREFIX + n) || "").split("\n", 1)[0]; } catch { /* storage blocked */ }
  return n + "\t" + head;
}).join("\n")));
const pushClock = () => { const d = new Date(); w.cl_clock(d.getFullYear(), d.getMonth() + 1, d.getDate(), d.getHours(), d.getMinutes()); };

async function run(pack) {
  w = await wasmInstance();
  put(0, pack.fonts); put(1, pack.glyphs); put(2, pack.forms); put(3, pack.country); put(4, pack.referee);
  put(6, new Uint8Array(0));
  for (const t of pack.teams) put(5, t);
  pushSaves();
  const seed = (Date.now() & 0x7fffffff) || 1;
  if (!w.cl_start(seed)) { say("Erro ao iniciar: " + text(out())); return; }
  pushSaves();
  pushClock();
  msg.style.display = "none";
  canvas.style.display = "block";
  fit();
  canvas.focus();
  loop(performance.now());
}

function fit() {
  const s = Math.min(innerWidth / 800, innerHeight / 600);
  const k = s >= 1 ? Math.floor(s * 4) / 4 : s;
  canvas.style.width = Math.floor(800 * k) + "px";
  canvas.style.height = Math.floor(600 * k) + "px";
}
addEventListener("resize", fit);

function pos(e) {
  const r = canvas.getBoundingClientRect();
  return [Math.floor((e.clientX - r.left) * 800 / r.width), Math.floor((e.clientY - r.top) * 600 / r.height)];
}
canvas.addEventListener("pointermove", (e) => { if (w) { const [x, y] = pos(e); w.cl_event(0, x, y); } });
canvas.addEventListener("pointerdown", (e) => { if (w && e.button === 0) { canvas.focus(); const [x, y] = pos(e); w.cl_event(0, x, y); w.cl_event(1, x, y); } e.preventDefault(); });
canvas.addEventListener("pointerup", (e) => { if (w && e.button === 0) { const [x, y] = pos(e); w.cl_event(2, x, y); } });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());

const KEYS = { Enter: 0, Escape: 1, Tab: 2, Backspace: 3, ArrowUp: 4, ArrowDown: 5, ArrowLeft: 6, ArrowRight: 7 };
addEventListener("keydown", (e) => {
  if (!w || e.ctrlKey || e.metaKey || e.altKey) return;
  let code = KEYS[e.key];
  if (e.key === "Tab" && e.shiftKey) code = 8;
  if (/^F([1-9]|1[0-2])$/.test(e.key)) code = 100 + Number(e.key.slice(1));
  if (code !== undefined) { w.cl_event(3, code, 0); e.preventDefault(); }
  else if (e.key.length === 1) { w.cl_event(4, e.key.codePointAt(0), 0); e.preventDefault(); }
});

function requests() {
  for (;;) {
    const r = w.cl_request();
    if (!r) return;
    if (r === 1) { say("Jogo terminado. Recarrega a página para jogar outra vez."); w = null; return; }
    const name = text(out());
    if (r === 2) {
      pushClock();
      w.cl_save();
      try { localStorage.setItem(SAVE_PREFIX + name, text(out())); } catch { /* storage full or blocked */ }
      pushSaves();
    } else if (r === 3) {
      // an unreadable game comes back as an empty text: the interface tells the player
      let t = null;
      try { t = localStorage.getItem(SAVE_PREFIX + name); } catch { /* storage blocked */ }
      const b = new TextEncoder().encode(t || "");
      const p = w.cl_alloc(b.length);
      new Uint8Array(w.memory.buffer, p, b.length).set(b);
      w.cl_load();
    }
  }
}

let last = 0;
function loop(now) {
  if (!w) return;
  const dt = Math.min(now - last, 250);
  last = now;
  w.cl_tick(Math.round(dt));
  requests();
  if (w && w.cl_frame()) {
    const img = new ImageData(new Uint8ClampedArray(w.memory.buffer, w.cl_fb_ptr(), 800 * 600 * 4), 800, 600);
    ctx.putImageData(img, 0, 0);
  }
  requestAnimationFrame(loop);
}

(async () => {
  try {
    const pack = (await fetchPack().catch(() => null)) || (await dbGet());
    if (pack) await run(pack); else askForPack();
  } catch (e) { say("Erro: " + e); }
})();
