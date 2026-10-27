// Host of the pixel-exact classic interface (ui/src/classic) compiled to WebAssembly (ui-web/src/classic_host.rs).
// The skin pack and the teams (with those saved by the team editor) come from pack.js.
import { allTeams, start } from "./pack.js";
const canvas = document.getElementById("c");
const ctx = canvas.getContext("2d");
const msg = document.getElementById("msg");
const SAVE_PREFIX = "elifoot98-classic:";

const say = (html) => { msg.innerHTML = html; msg.style.display = "block"; canvas.style.display = "none"; };

async function wasmInstance() {
  const res = await fetch("rustfoot_ui.wasm");
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  return instance.exports;
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
  for (const t of await allTeams(pack)) put(5, t.bytes);
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

// the original's MessageBeep(0): a short tone (WebAudio; browsers allow it after the first key or click)
let audio = null;
function beep() {
  try {
    audio = audio || new AudioContext();
    const o = audio.createOscillator(), g = audio.createGain();
    o.type = "square";
    o.frequency.value = 880;
    g.gain.value = 0.05;
    o.connect(g).connect(audio.destination);
    o.start();
    o.stop(audio.currentTime + 0.12);
  } catch { /* no audio */ }
}

function requests() {
  for (;;) {
    const r = w.cl_request();
    if (!r) return;
    if (r === 1) { say("Jogo terminado. Recarrega a página para jogar outra vez."); w = null; return; }
    if (r === 4) { beep(); continue; }
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

start(say, run);
