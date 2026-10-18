// Replays the command list produced by the shared UI (ui-web) on a canvas and forwards input to it.
const canvas = document.getElementById("c");
const ctx = canvas.getContext("2d");
const bytes = await (await fetch("elifoot_ui.wasm")).arrayBuffer();
const { instance } = await WebAssembly.instantiate(bytes, {});
const w = instance.exports;
w.ui_new(1998);

const dec = new TextDecoder();
function frame() {
  w.ui_frame();
  const cmds = JSON.parse(dec.decode(new Uint8Array(w.memory.buffer, w.frame_ptr(), w.frame_len())));
  for (const c of cmds) {
    if (c[0] === 0) { ctx.fillStyle = c[5]; ctx.fillRect(c[1], c[2], c[3], c[4]); }
    else if (c[0] === 1) { ctx.strokeStyle = c[5]; ctx.lineWidth = 1; ctx.strokeRect(c[1] + 0.5, c[2] + 0.5, c[3] - 1, c[4] - 1); }
    else {
      ctx.fillStyle = c[4];
      ctx.font = `${c[5]}px system-ui, "Liberation Sans", Arial, sans-serif`;
      ctx.textBaseline = "top";
      ctx.fillText(c[3], c[1], c[2]);
    }
  }
}

const pos = (e) => {
  const r = canvas.getBoundingClientRect();
  return [Math.round((e.clientX - r.left) * canvas.width / r.width), Math.round((e.clientY - r.top) * canvas.height / r.height)];
};
canvas.addEventListener("pointermove", (e) => { const [x, y] = pos(e); w.ui_event(0, x, y); frame(); });
canvas.addEventListener("pointerdown", (e) => { const [x, y] = pos(e); w.ui_event(1, x, y); frame(); });
canvas.addEventListener("pointerup", (e) => { const [x, y] = pos(e); w.ui_event(2, x, y); frame(); });
canvas.addEventListener("wheel", (e) => { e.preventDefault(); w.ui_event(3, Math.sign(-e.deltaY), 0); frame(); }, { passive: false });
const KEYS = { " ": 0, s: 1, S: 1, Tab: 2, 1: 3, 2: 4, 3: 5, 4: 6, ArrowUp: 7, ArrowDown: 8, Backspace: 9, Enter: 10, Escape: 11 };
canvas.addEventListener("keydown", (e) => {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  if (w.ui_wants_text() && e.key.length === 1) { e.preventDefault(); w.ui_event(5, e.key.codePointAt(0), 0); frame(); return; }
  if (e.key in KEYS) { e.preventDefault(); w.ui_event(4, KEYS[e.key], 0); frame(); }
});
frame();
canvas.focus();
