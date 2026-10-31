// The phone's on-screen keyboard for the game and the team editor. A touch screen has no keyboard of its own, and a browser only
// shows the on-screen one for a focused text field: a hidden input takes the focus after a tap when the interface's focused
// control takes text (`wantsText`), and gives it back when it no longer does. What is typed is sent on as keys and characters.
// A physical keyboard keeps going through the page's own keydown handler (which cancels the key, so nothing reaches the input).
const SENTINEL = "  "; // kept in the input so the keyboard's backspace has something to delete

export function softKeyboard(canvas, wantsText, sendKey, sendChar) {
  const touch = matchMedia("(pointer: coarse)").matches || "ontouchstart" in window;
  if (!touch) return { check() {} };
  const input = document.createElement("input");
  Object.assign(input, { type: "text", autocomplete: "off", spellcheck: false, value: SENTINEL });
  input.setAttribute("autocorrect", "off");
  input.setAttribute("autocapitalize", "characters");
  input.setAttribute("enterkeyhint", "done");
  input.setAttribute("aria-label", "Teclado");
  // (16px: a smaller font makes iOS zoom the page when the field takes the focus)
  input.style.cssText = "position:fixed;left:0;top:0;width:1px;height:1px;opacity:0;font-size:16px;border:0;padding:0;";
  document.body.appendChild(input);

  // what the game has been sent so far is what the input shows: each change is sent as backspaces and characters, also while a
  // keyboard is composing a word (Android's), so the letters show as they are typed
  let shown = SENTINEL, composing = false;
  const reset = () => { input.value = shown = SENTINEL; input.setSelectionRange(SENTINEL.length, SENTINEL.length); };
  const flush = () => {
    const v = input.value;
    let p = 0;
    while (p < v.length && p < shown.length && v[p] === shown[p]) p++;
    for (let i = p; i < shown.length; i++) sendKey(3); // Backspace
    for (const ch of v.slice(p)) ch === "\n" ? sendKey(0) : sendChar(ch);
    shown = v;
    if (!composing) reset();
  };
  input.addEventListener("compositionstart", () => { composing = true; });
  input.addEventListener("compositionend", () => { composing = false; flush(); });
  input.addEventListener("input", flush);
  // the keys that are not text go to the interface as keys (on Android's keyboards Backspace may come as 229, "Unidentified",
  // and then reaches `input` as a deletion); the page's own keydown handler does not see them again
  const KEYS = { Enter: 0, Escape: 1, Tab: 2, Backspace: 3, ArrowUp: 4, ArrowDown: 5, ArrowLeft: 6, ArrowRight: 7 };
  input.addEventListener("keydown", (e) => {
    let code = KEYS[e.key];
    if (e.key === "Tab" && e.shiftKey) code = 8;
    if (code === undefined || composing) return;
    flush();
    sendKey(code);
    e.preventDefault();
    e.stopPropagation();
  });

  const check = () => {
    const want = wantsText();
    if (want && document.activeElement !== input) { reset(); input.focus({ preventScroll: true }); }
    else if (!want && document.activeElement === input) { input.blur(); canvas.focus({ preventScroll: true }); }
  };
  // a browser opens its keyboard only for a focus given while handling the tap itself
  canvas.addEventListener("pointerup", check);
  return { check };
}
