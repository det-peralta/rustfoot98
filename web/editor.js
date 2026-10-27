// Host of the team editor (ui/src/editor, the original's EDITEQ.EXE) compiled to WebAssembly (ui-web/src/editor_host.rs).
// The teams are the pack's (pack.js) with the ones saved here, which are kept in this browser and used by new games.
import { allTeams, offline, saveTeam, start } from "./pack.js";

const canvas = document.getElementById("c");
const ctx = canvas.getContext("2d");
const msg = document.getElementById("msg");
const bar = document.getElementById("bar");
const say = (html) => { msg.innerHTML = html; msg.style.display = "block"; canvas.style.display = "none"; bar.style.display = "none"; };

let w;
function put(kind, bytes) {
  const p = w.ed_alloc(bytes.length);
  new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes);
  return w.ed_put(kind);
}
const enc = (s) => new TextEncoder().encode(s);
const out = () => new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.ed_out_ptr(), w.ed_out_len()));
const file = () => new Uint8Array(w.memory.buffer, w.ed_file_ptr(), w.ed_file_len()).slice();

async function run(pack) {
  const res = await fetch("rustfoot_ui.wasm");
  w = (await WebAssembly.instantiate(await res.arrayBuffer(), {})).instance.exports;
  put(0, pack.fonts); put(1, pack.glyphs); put(2, pack.forms); put(3, pack.country);
  put(6, new Uint8Array(0));
  for (const t of await allTeams(pack)) { put(4, enc(t.name)); put(5, t.bytes); }
  if (!w.ed_start()) { say("Erro ao iniciar: " + out()); return; }
  msg.style.display = "none";
  canvas.style.display = "block";
  bar.style.display = "flex";
  fit();
  canvas.focus();
  requestAnimationFrame(loop);
}

function fit() {
  const s = Math.min(innerWidth / 800, (innerHeight - 40) / 600);
  const k = s >= 1 ? Math.floor(s * 4) / 4 : s;
  canvas.style.width = Math.floor(800 * k) + "px";
  canvas.style.height = Math.floor(600 * k) + "px";
}
addEventListener("resize", fit);

function pos(e) {
  const r = canvas.getBoundingClientRect();
  return [Math.floor((e.clientX - r.left) * 800 / r.width), Math.floor((e.clientY - r.top) * 600 / r.height)];
}
canvas.addEventListener("pointermove", (e) => { if (w) { const [x, y] = pos(e); w.ed_event(0, x, y); } });
canvas.addEventListener("pointerdown", (e) => {
  if (!w || e.button > 2) return;
  canvas.focus();
  const [x, y] = pos(e);
  w.ed_event(0, x, y);
  w.ed_event(e.button === 2 ? 2 : 1, x, y);
  e.preventDefault();
});
canvas.addEventListener("dblclick", (e) => { if (w) { const [x, y] = pos(e); w.ed_event(5, x, y); } });
canvas.addEventListener("contextmenu", (e) => e.preventDefault());

const KEYS = { Enter: 0, Escape: 1, Tab: 2, Backspace: 3, ArrowUp: 4, ArrowDown: 5, ArrowLeft: 6, ArrowRight: 7, Insert: 9, Delete: 10 };
addEventListener("keydown", (e) => {
  if (!w || e.metaKey || e.altKey || document.activeElement !== canvas) return;
  if (e.ctrlKey) {
    // Ctrl+N: Nova equipa
    if (e.key === "n" || e.key === "N") { w.ed_event(4, 14, 0); e.preventDefault(); }
    return;
  }
  let code = KEYS[e.key];
  if (e.key === "Tab" && e.shiftKey) code = 8;
  if (/^F([1-9]|1[0-2])$/.test(e.key)) code = 100 + Number(e.key.slice(1));
  if (code !== undefined) { w.ed_event(3, code, 0); e.preventDefault(); }
  else if (e.key.length === 1) { w.ed_event(4, e.key.codePointAt(0), 0); e.preventDefault(); }
});

function download(name, bytes) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

document.getElementById("export").addEventListener("click", () => {
  if (w && w.ed_export()) download(out(), file());
  canvas.focus();
});
document.getElementById("import").addEventListener("change", async (e) => {
  for (const f of e.target.files) {
    const bytes = new Uint8Array(await f.arrayBuffer());
    put(4, enc(f.name));
    const p = w.ed_alloc(bytes.length);
    new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes);
    if (w.ed_import()) await saveTeam(f.name, bytes);
  }
  e.target.value = "";
  canvas.focus();
});

function requests() {
  for (;;) {
    const r = w.ed_request();
    if (!r) return;
    if (r === 1) { say('Editor terminado. <a href="editor.html" style="color:#fff">Abrir outra vez</a> ou <a href="classic.html" style="color:#fff">jogar</a>.'); w = null; return; }
    if (r === 2) saveTeam(out(), file());
  }
}

let last = 0;
function loop(now) {
  if (!w) return;
  w.ed_tick(Math.round(Math.min(now - (last || now), 250)));
  last = now;
  requests();
  if (w && w.ed_frame()) {
    ctx.putImageData(new ImageData(new Uint8ClampedArray(w.memory.buffer, w.ed_fb_ptr(), 800 * 600 * 4), 800, 600), 0, 0);
  }
  requestAnimationFrame(loop);
}

offline();
start(say, run);
