// The skin pack shared by the game (classic.js) and the team editor (editor.js): fonts, glyphs, forms, COUNTRY.TXE, REFEREE.TXE and
// the team files, built from the user's own copy of the game by tools/ui/webpack.py. It is read from ./skin/ or, failing that, from a
// folder the user picks (kept in IndexedDB). Teams saved by the editor live in their own IndexedDB store and replace the pack's
// files of the same name.

const DB = "elifoot98-classic-pack";
const idb = () => new Promise((ok, no) => {
  const r = indexedDB.open(DB, 2);
  r.onupgradeneeded = () => {
    if (!r.result.objectStoreNames.contains("files")) r.result.createObjectStore("files");
    if (!r.result.objectStoreNames.contains("teams")) r.result.createObjectStore("teams");
  };
  r.onsuccess = () => ok(r.result);
  r.onerror = () => no(r.error);
});

async function storeGet(store, key) {
  try {
    const db = await idb();
    return await new Promise((ok) => {
      const q = db.transaction(store).objectStore(store).get(key);
      q.onsuccess = () => ok(q.result || null);
      q.onerror = () => ok(null);
    });
  } catch { return null; }
}

async function storePut(store, key, value) {
  try {
    const db = await idb();
    await new Promise((ok) => {
      const t = db.transaction(store, "readwrite");
      t.objectStore(store).put(value, key);
      t.oncomplete = ok; t.onerror = ok;
    });
  } catch { /* the store is only a convenience */ }
}

export const dbGet = () => storeGet("files", "pack");
export const dbPut = (pack) => storePut("files", "pack", pack);

/// The teams the editor saved: [{name, bytes}].
export async function editedTeams() {
  try {
    const db = await idb();
    return await new Promise((ok) => {
      const out = [];
      const q = db.transaction("teams").objectStore("teams").openCursor();
      q.onsuccess = () => {
        const c = q.result;
        if (!c) { ok(out); return; }
        out.push({ name: c.key, bytes: c.value });
        c.continue();
      };
      q.onerror = () => ok(out);
    });
  } catch { return []; }
}

export const saveTeam = (name, bytes) => storePut("teams", name.toUpperCase(), bytes);

/// The pack's teams with the edited ones in place of those of the same name (and added when new): [{name, bytes}].
export async function allTeams(pack) {
  const teams = (pack.teams || []).map((t, i) => (t instanceof Uint8Array ? { name: `T${i}.EFT`, bytes: t } : t));
  for (const e of await editedTeams()) {
    const i = teams.findIndex((t) => t.name.toUpperCase() === e.name);
    if (i >= 0) teams[i] = e; else teams.push(e);
  }
  return teams;
}

export async function fetchPack() {
  const idx = await fetch("skin/index.json");
  if (!idx.ok) return null;
  const list = await idx.json();
  const get = async (n) => new Uint8Array(await (await fetch("skin/" + n)).arrayBuffer());
  const pack = { fonts: await get(list.fonts), glyphs: await get(list.glyphs), forms: await get(list.forms),
                 country: await get(list.country), referee: await get(list.referee), teams: [] };
  for (const t of list.teams || []) pack.teams.push({ name: t.toUpperCase(), bytes: await get(t) });
  return pack;
}

export async function packFromFiles(files) {
  const by = {};
  const teams = [];
  for (const f of files) {
    const n = f.name.toLowerCase();
    const bytes = new Uint8Array(await f.arrayBuffer());
    if (n === "fonts.bin") by.fonts = bytes; else if (n === "glyphs.bin") by.glyphs = bytes;
    else if (n === "forms.json") by.forms = bytes; else if (n === "country.txe") by.country = bytes;
    else if (n === "referee.txe") by.referee = bytes; else if (n.endsWith(".eft")) teams.push({ name: f.name.toUpperCase(), bytes });
  }
  if (!(by.fonts && by.glyphs && by.forms && by.country && by.referee)) return null;
  return { ...by, teams };
}

/// Shows the request for the pack in `say` and calls `run(pack)` once the user picked a valid folder.
export function askForPack(say, run) {
  say(`<h2>Elifoot 98</h2>
    <p>Para jogar com o aspecto exacto do original é preciso o <b>pacote de aspecto</b>, feito a partir da tua própria cópia do jogo:
    <code>python tools/ui/skin.py</code> e <code>python tools/ui/webpack.py</code> (cria <code>web/skin/</code>).</p>
    <p>Ou escolhe aqui a pasta <code>web/skin</code> (os ficheiros ficam só neste navegador):</p>
    <label class="btn"><input id="pick" type="file" webkitdirectory multiple hidden>Escolher pasta…</label>`);
  document.getElementById("pick").addEventListener("change", async (e) => {
    const pack = await packFromFiles([...e.target.files]);
    if (!pack) { say("Pasta inválida: faltam <code>fonts.bin</code>, <code>glyphs.bin</code>, <code>forms.json</code>, <code>COUNTRY.TXE</code> ou <code>REFEREE.TXE</code>."); setTimeout(() => askForPack(say, run), 3000); return; }
    await dbPut(pack);
    run(pack);
  });
}

/// Loads the pack (served, else stored) and runs `run(pack)`, or asks for it.
export async function start(say, run) {
  try {
    const pack = (await fetchPack().catch(() => null)) || (await dbGet());
    if (pack) await run(pack); else askForPack(say, run);
  } catch (e) { say("Erro: " + e); }
}
