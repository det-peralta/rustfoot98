// Loads the WebAssembly core and drives the UI. The core exposes a plain C ABI and returns the world as JSON.
const ICON = { goal: "⚽", annulled: "⊘", red: "🟥", injury: "✚", sub: "🔁" };
const POS = ["GR", "D", "M", "A"];
const LABEL = { goal: "Golo", annulled: "Golo anulado", red: "Expulsão", injury: "Lesão" };

let wasm, state, division = 0, selectedMatch = null, useReal = false, needTab = true;
const $ = (id) => document.getElementById(id);
const SAVE_KEY = "elifoot98-rs-save-v1";
const store = {
  get() { try { return localStorage.getItem(SAVE_KEY); } catch { return null; } },
  set(v) { try { localStorage.setItem(SAVE_KEY, v); return true; } catch { return false; } },
};

// Offline support: a service worker caches the page and the WebAssembly core. Add ?nosw to the URL while developing to skip it
// (and remove any registered worker), otherwise edits can be hidden behind a stale cache.
async function setupOffline() {
  const out = $("offline-status");
  if (!("serviceWorker" in navigator)) { out.textContent = "Este browser não suporta modo offline."; return; }
  if (location.search.includes("nosw")) {
    for (const r of await navigator.serviceWorker.getRegistrations()) await r.unregister();
    out.textContent = "Modo offline desligado (?nosw).";
    return;
  }
  try {
    await navigator.serviceWorker.register("sw.js");
    await navigator.serviceWorker.ready;
    out.textContent = "Funciona offline e pode ser instalado como aplicação.";
  } catch (e) {
    out.textContent = "Modo offline indisponível (precisa de https ou localhost).";
  }
}

async function boot() {
  setupOffline();
  const res = await fetch("elifoot_wasm.wasm");
  const bytes = await res.arrayBuffer();
  $("wasm-size").textContent = (bytes.byteLength / 1024).toFixed(0) + " KB";
  const { instance } = await WebAssembly.instantiate(bytes, {});
  wasm = instance.exports;
  $("load-now").disabled = !store.get();
  if (store.get()) $("save-status").textContent = "Há um jogo guardado neste browser (botão Carregar).";
  newWorld();
}

function read() {
  const ptr = wasm.world_json_ptr(), len = wasm.world_json_len();
  state = JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len)));
  if (needTab && state.human) { division = state.human.div; needTab = false; }
  render();
}

function saveText() {
  wasm.world_save();
  return new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, wasm.save_ptr(), wasm.save_len()));
}

function saveNow(auto) {
  const ok = store.set(saveText());
  $("load-now").disabled = !store.get();
  $("save-status").textContent = ok ? `${auto ? "Guardado automaticamente" : "Guardado"} às ${new Date().toLocaleTimeString("pt-PT", { hour: "2-digit", minute: "2-digit" })}.` : "Não foi possível guardar (armazenamento do browser bloqueado). Usa Exportar ficheiro.";
}

function loadText(text) {
  const bytes = new TextEncoder().encode(text);
  const ptr = wasm.buf_resize(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  if (!wasm.world_load(bytes.length)) {
    const p = wasm.world_json_ptr(), l = wasm.world_json_len();
    $("save-status").textContent = "Ficheiro inválido: " + JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, p, l))).error;
    return false;
  }
  selectedMatch = null;
  needTab = true;
  useReal = false;
  read();
  $("mode").textContent = "jogo carregado";
  $("save-status").textContent = "Jogo carregado.";
  return true;
}

function importE98(bytes) {
  const ptr = wasm.buf_resize(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const seed = Math.max(1, Number($("seed").value) | 0);
  if (!wasm.world_import_e98(bytes.length, seed)) {
    const p = wasm.world_json_ptr(), l = wasm.world_json_len();
    $("save-status").textContent = "Não foi possível importar: " + JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, p, l))).error;
    return false;
  }
  selectedMatch = null;
  needTab = true;
  useReal = false;
  read();
  $("mode").textContent = "jogo importado do Elifoot 98 original";
  $("save-status").textContent = "Importado do .e98 original (classificações, jogadores, moral; economia, finanças e taça recomeçam).";
  return true;
}

function newWorld() {
  selectedMatch = null;
  needTab = true;
  const seed = Math.max(1, Number($("seed").value) | 0);
  if (useReal) {
    if (!wasm.world_new_from_efts(seed)) { fail(); return; }
    read();
  } else {
    wasm.world_new(seed);
    read();
  }
}

// Reads the error the core published as {"error": "..."} and falls back to the fictional league.
function fail() {
  const ptr = wasm.world_json_ptr(), len = wasm.world_json_len();
  const msg = JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len))).error;
  $("eft-status").textContent = "Não foi possível usar essas equipas: " + msg;
  useReal = false;
  $("use-fiction").hidden = true;
  wasm.world_new(Math.max(1, Number($("seed").value) | 0));
  read();
}

async function loadEfts(files) {
  // the original reads the folder in directory order (alphabetical on the host); equal levels keep that order
  files = [...files].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
  wasm.eft_clear();
  let ok = 0;
  for (const f of files) {
    const bytes = new Uint8Array(await f.arrayBuffer());
    const ptr = wasm.buf_resize(bytes.length);          // may grow memory: take the view afterwards
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
    ok += wasm.eft_add(bytes.length);
  }
  $("eft-status").textContent = `${ok} equipas válidas em ${files.length} ficheiros.`;
  if (ok < 34) { $("eft-status").textContent += " São precisas pelo menos 34."; return; }
  useReal = true;
  $("use-fiction").hidden = false;
  newWorld();
  if (useReal) $("eft-status").textContent = `${ok} equipas reais carregadas: por nível (do mais alto para o mais baixo) as 32 primeiras formam as 4 divisões e as restantes ficam fora da liga (máx. 128).`;
}

function play(fn) {
  selectedMatch = null;
  wasm[fn]();
  read();
  if ($("autosave").checked) saveNow(true);
}

function renderTabs() {
  const nav = $("tabs");
  nav.innerHTML = "";
  state.divisions.forEach((d, i) => {
    const b = document.createElement("button");
    b.textContent = d.name;
    b.setAttribute("role", "tab");
    b.setAttribute("aria-selected", String(i === division));
    b.onclick = () => { division = i; selectedMatch = null; render(); };
    nav.appendChild(b);
  });
}

function renderTable() {
  const d = state.divisions[division];
  $("t-title").textContent = "Classificação — " + d.name;
  const body = $("table").querySelector("tbody");
  body.innerHTML = "";
  d.table.forEach((r, i) => {
    const tr = document.createElement("tr");
    const zone = i < 2 && division > 0 ? "up" : i >= 6 && division < 3 ? "down" : "";
    tr.innerHTML = `<td class="${zone}">${i + 1}</td><td class="l">${esc(r.name)}</td><td>${r.w + r.d + r.l}</td><td>${r.w}</td><td>${r.d}</td><td>${r.l}</td>` +
      `<td>${r.gf}</td><td>${r.ga}</td><td class="pts">${r.pts}</td><td>${r.k.toFixed(2)}</td><td>${r.str.toFixed(1)}</td>`;
    body.appendChild(tr);
  });
  $("base-note").textContent = `Nível-base da divisão: ${d.base} (janela de evolução = base − 2 × posição ± 5). Os dois primeiros sobem, os dois últimos descem.`;
}

function renderResults() {
  const list = $("results");
  list.innerHTML = "";
  const matches = state.lastRound.filter((m) => m.div === division);
  if (!matches.length) {
    list.innerHTML = `<li class="empty">Ainda não foi jogada nenhuma jornada.</li>`;
    $("match").hidden = true;
    return;
  }
  matches.forEach((m, i) => {
    const li = document.createElement("li");
    if (isMine(m)) li.className = "mine";
    const b = document.createElement("button");
    b.setAttribute("aria-pressed", String(i === selectedMatch));
    b.innerHTML = `<span>${esc(m.home)}</span><span class="score">${m.hg} – ${m.ag}</span><span class="away">${esc(m.away)}</span>`;
    b.onclick = () => { selectedMatch = i; renderResults(); };
    li.appendChild(b);
    list.appendChild(li);
  });
  const box = $("match");
  if (selectedMatch == null) { box.hidden = true; return; }
  const m = matches[selectedMatch];
  box.hidden = false;
  const tl = timeline(m);
  const evs = tl.length ? tl.map((e) => `<div class="ev ${e.away ? "away" : ""}"><span class="m">${e.m}'</span><span>${e.html}</span><span></span></div>`).join("") : `<p class="empty">Sem acontecimentos.</p>`;
  const gate = m.att ? `<p class="note">Público: ${m.att.toLocaleString("pt-PT")} · Receita: ${money(m.gate)}</p>` : "";
  box.innerHTML = `<h3>${esc(m.home)} ${m.hg} – ${m.ag} ${esc(m.away)}</h3>${gate}${evs}`;
}

const isMine = (m) => state.human && m.div === state.human.div && (m.homeIdx === state.human.idx || m.awayIdx === state.human.idx) &&
  (m.home === state.human.club || m.away === state.human.club);

// events and substitutions of a match in time order
function timeline(m) {
  const evs = m.events.map((e) => ({ m: e.m, html: `${ICON[e.kind]} ${esc(e.player)} <small>(${esc(e.side ? m.away : m.home)})</small>`, away: !!e.side }));
  const subs = (m.subs || []).map((s) => ({ m: s.m, html: `${ICON.sub} ${esc(s.in)} por ${esc(s.out)} <small>(${esc(s.side ? m.away : m.home)}${s.auto ? ", automática" : ""})</small>`, away: !!s.side }));
  return [...evs, ...subs].sort((a, b) => a.m - b.m);
}

function subsHtml(m) {
  if (!m.subs || !m.subs.length) return "";
  return "<ul class=\"mm\">" + m.subs.map((s) => `<li>${s.m}' ${ICON.sub} ${esc(s.in)} por ${esc(s.out)} <small>(${esc(s.side ? m.away : m.home)}${s.auto ? ", automática" : ""})</small></li>`).join("") + "</ul>";
}

// ---- the user's club -------------------------------------------------------------------------------------------
function fmt(f) { return f.join("-"); }

function sendPlan(starters, bench, subs) {
  const bytes = [starters.length, ...starters, bench.length, ...bench, subs.length, ...subs.flat()];
  const ptr = wasm.buf_resize(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  wasm.human_plan(bytes.length);
  read();
}

function planFromDom() {
  const roles = [...document.querySelectorAll("#squad select.role")];
  const pick = (v) => roles.filter((r) => r.value === v).map((r) => Number(r.dataset.i));
  return { starters: pick("T"), bench: pick("B") };
}

function subsFromDom() {
  return [...document.querySelectorAll(".subrow")].map((row) => {
    const [minute, out, inn] = row.querySelectorAll("input, select");
    return [Math.min(90, Math.max(1, Number(minute.value) | 0)), Number(out.value), Number(inn.value)];
  });
}

const money = (v) => `${v < 0 ? "−" : ""}${Math.abs(v).toLocaleString("pt-PT")}`;

function renderFinance(h) {
  const f = h.finance;
  if (!f) { $("finance").innerHTML = ""; return; }
  $("finance").innerHTML = `<h3>Finanças</h3><dl>` +
    `<div><dt>Caixa</dt><dd>${money(f.cash)}</dd></div>` +
    `<div><dt>Bilhetes (época)</dt><dd>${money(f.gate)}</dd></div>` +
    `<div><dt>Salários (época)</dt><dd>${money(f.wages)}</dd></div>` +
    `<div><dt>Salários por jogo</dt><dd>${money(-f.wageBill)}</dd></div>` +
    `<div><dt>Preço do bilhete</dt><dd>${f.price}</dd></div>` +
    `<div><dt>Estádio</dt><dd>${f.capacity.toLocaleString("pt-PT")} lugares (nível ${f.stadium})</dd></div>` +
    `<div><dt>Empréstimo</dt><dd>${money(f.loan)}</dd></div></dl>` +
    `<p class="note">Valores em reais (escala ${f.scale}). Receitas de bilheteira, salários, caixa inicial e estádio seguem o original; transferências ainda não existem.</p>`;
}

function renderClub() {
  const card = $("club-card"), h = state.human;
  if (!h) { card.hidden = true; return; }
  card.hidden = false;
  renderFinance(h);
  $("club-name").textContent = h.club;
  $("club-div").textContent = state.divisions[h.div].name;
  const sel = $("formation");
  if (!sel.options.length) h.formations.forEach((f, k) => sel.add(new Option(fmt(f), k)));
  sel.value = h.formation;
  const change = $("change-club");
  change.innerHTML = "";
  state.divisions.forEach((d, di) => d.table.forEach((r) => {
    // table rows are sorted by points; the club index is not known here, so the option carries the name only
    change.add(new Option(`${di + 1}ª · ${r.name}`, `${di}|${r.name}`));
  }));
  change.value = `${h.div}|${h.club}`;

  const starters = new Set(h.plan.starters), bench = new Set(h.plan.bench);
  const body = $("squad").querySelector("tbody");
  body.innerHTML = "";
  [...h.squad].sort((a, b) => a.pos - b.pos || b.str - a.str).forEach((p) => {
    const unavailable = p.ban > 0 || p.inj > 0;
    const state_ = p.inj > 0 ? `<span class="badge bad">Lesionado ${p.inj} j</span>` : p.ban > 0 ? `<span class="badge bad">Suspenso ${p.ban} j</span>` : "";
    const role = starters.has(p.i) ? "T" : bench.has(p.i) ? "B" : "F";
    const tr = document.createElement("tr");
    tr.className = unavailable ? "out" : role === "T" ? "start" : "";
    tr.innerHTML = `<td class="l">${POS[p.pos]}</td><td class="l">${esc(p.name)}</td><td>${p.str}</td><td>${p.star ? "★" : ""}</td><td class="l">${state_}</td>` +
      `<td class="l"><select class="role" data-i="${p.i}" ${unavailable ? "disabled" : ""} aria-label="Papel de ${esc(p.name)}">` +
      `<option value="T">Titular</option><option value="B">Banco</option><option value="F">Fora</option></select></td>`;
    tr.querySelector("select").value = unavailable ? "F" : role;
    tr.querySelector("select").onchange = () => { const p2 = planFromDom(); sendPlan(p2.starters, p2.bench, subsFromDom()); };
    body.appendChild(tr);
  });
  const counts = [0, 0, 0, 0];
  h.plan.starters.forEach((i) => counts[h.squad[i].pos]++);
  const msg = [];
  msg.push(`Em campo: ${counts[0]} GR · ${counts[1]}-${counts[2]}-${counts[3]}.`);
  if (h.plan.starters.length !== 11 || counts[0] !== 1) msg.push("A equipa tem de ter 11 titulares e 1 guarda-redes; o jogo corrige o que faltar.");
  $("club-msg").textContent = msg.join(" ");

  const subsBox = $("subs");
  subsBox.innerHTML = "";
  const opt = (list) => list.map((i) => `<option value="${i}">${esc(h.squad[i].name)}</option>`).join("");
  h.plan.subs.forEach(([minute, out, inn], k) => {
    const row = document.createElement("div");
    row.className = "subrow";
    row.innerHTML = `<input type="number" min="1" max="90" value="${minute}" aria-label="Minuto"><select aria-label="Sai">${opt(h.plan.starters)}</select><select aria-label="Entra">${opt(h.plan.bench)}</select><button aria-label="Remover">✕</button>`;
    const [mi, so, si] = row.querySelectorAll("input, select");
    so.value = out; si.value = inn;
    const update = () => sendPlan(h.plan.starters, h.plan.bench, subsFromDom());
    mi.onchange = so.onchange = si.onchange = update;
    row.querySelector("button").onclick = () => { row.remove(); update(); };
    subsBox.appendChild(row);
  });
  $("add-sub").disabled = h.plan.subs.length >= 3 || !h.plan.bench.length;

  const mine = state.lastRound.find(isMine);
  const box = $("my-match");
  if (!mine) { box.className = "empty"; box.textContent = "Ainda não jogaste."; return; }
  box.className = "";
  const evs = timeline(mine).map((e) => `<li>${e.m}' ${e.html}</li>`).join("");
  box.innerHTML = `<p><b>${esc(mine.home)} ${mine.hg} – ${mine.ag} ${esc(mine.away)}</b></p><ul class="mm">${evs || "<li class=\"empty\">Sem acontecimentos.</li>"}</ul>`;
}

let selectedCup = null;
function renderCup() {
  const card = $("cup-card"), c = state.lastCup;
  if (!c) { card.hidden = true; return; }
  card.hidden = false;
  $("cup-title").textContent = `Taça — ${c.name}`;
  const mine = (m) => state.human && (m.home === state.human.club || m.away === state.human.club);
  const list = $("cup-results");
  list.innerHTML = "";
  if (selectedCup != null && selectedCup >= c.matches.length) selectedCup = null;
  c.matches.forEach((m, i) => {
    const li = document.createElement("li");
    if (mine(m)) li.className = "mine";
    const b = document.createElement("button");
    b.setAttribute("aria-pressed", String(i === selectedCup));
    const how = m.shootout ? " (g.p.)" : m.extraTime ? " (a.p.)" : "";
    b.innerHTML = `<span>${esc(m.home)}</span><span class="score">${m.hg} – ${m.ag}${how}</span><span class="away">${esc(m.away)}</span>`;
    b.onclick = () => { selectedCup = i; renderCup(); };
    li.appendChild(b);
    list.appendChild(li);
  });
  const box = $("cup-match");
  if (selectedCup == null) box.hidden = true;
  else {
    const m = c.matches[selectedCup];
    box.hidden = false;
    const tl = timeline(m);
    const evs = tl.length ? tl.map((e) => `<div class="ev ${e.away ? "away" : ""}"><span class="m">${e.m}'</span><span>${e.html}</span><span></span></div>`).join("") : `<p class="empty">Sem acontecimentos.</p>`;
    box.innerHTML = `<h3>${esc(m.home)} ${m.hg} – ${m.ag} ${esc(m.away)}</h3>${evs}<p class="note">Passa: <b>${esc(m.winner)}</b>${m.shootout ? " (grandes penalidades)" : m.extraTime ? " (após prolongamento)" : ""}</p>`;
  }
  $("cup-note").textContent = c.left === 1 ? "Final disputada." : `Ainda em prova: ${c.left} clubes.`;
}

function renderTransfers() {
  const card = $("transfers-card"), list = state.transfers || [];
  if (!list.length) { card.hidden = true; return; }
  card.hidden = false;
  $("transfers").innerHTML = list.map((t) =>
    `<li><span class="muted">${t.year} · J${t.round}</span> <b>${esc(t.player)}</b> <small>(${POS[t.pos]}, ${t.str})</small> ${esc(t.from)} → ${esc(t.to)} <span class="fee">${money(t.fee)}</span></li>`).join("");
}

function renderSeason() {
  const card = $("season-card"), s = state.lastSeason;
  if (!s) { card.hidden = true; return; }
  card.hidden = false;
  const champ = s.tables[0][0];
  const names = state.divisions.map((d) => d.name).concat(["Fora da liga"]);
  const ups = s.moves.filter((m) => m.to < m.from).map((m) => `<li><span class="up-t">▲</span> ${esc(m.name)} → ${names[m.to]}</li>`).join("");
  const downs = s.moves.filter((m) => m.to > m.from).map((m) => `<li><span class="down-t">▼</span> ${esc(m.name)} → ${names[m.to]}</li>`).join("");
  $("s-title").textContent = `Fim de época ${s.year}`;
  $("season").innerHTML = (s.cupWinner ? `<p><b>Vencedor da Taça:</b> ${esc(s.cupWinner)}.</p>` : "") + `<p><b>Campeão da 1ª Divisão:</b> ${esc(champ.name)} — ${champ.pts} pontos (${champ.w}V ${champ.d}E ${champ.l}D, ${champ.gf}–${champ.ga}).</p>` +
    `<div class="moves"><div><h3>Subidas</h3><ul>${ups}</ul></div><div><h3>Descidas</h3><ul>${downs}</ul></div></div>`;
}

function render() {
  $("mode").textContent = useReal ? "equipas e jogadores reais dos teus ficheiros .EFT" : "clubes e jogadores fictícios";
  $("year").textContent = state.year;
  $("round").textContent = `${state.round} / ${state.rounds}`;
  $("economy").textContent = state.economy.toFixed(2);
  renderTabs();
  renderTable();
  renderResults();
  renderClub();
  renderCup();
  renderTransfers();
  renderSeason();
}

// The tables are sorted by points while the core addresses clubs by position in the division list. The JSON carries the
// position of each club in `idx` for the user's club and in `homeIdx`/`awayIdx` for matches; for an arbitrary club we ask
// the core by name instead.
function indexInList(di, name, fallback) {
  const enc = new TextEncoder().encode(name);
  const ptr = wasm.buf_resize(enc.length);
  new Uint8Array(wasm.memory.buffer, ptr, enc.length).set(enc);
  const i = wasm.club_index(di, enc.length);
  return i >= 0 ? i : fallback;
}

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

$("add-sub").onclick = () => {
  const h = state.human;
  if (!h || h.plan.subs.length >= 3 || !h.plan.bench.length) return;
  const outfield = h.plan.starters.filter((i) => h.squad[i].pos !== 0);
  const subs = subsFromDom();
  subs.push([60, outfield[0] ?? h.plan.starters[0], h.plan.bench.find((i) => h.squad[i].pos !== 0) ?? h.plan.bench[0]]);
  sendPlan(h.plan.starters, h.plan.bench, subs);
};
$("formation").onchange = (e) => { if (!wasm.human_formation(Number(e.target.value))) $("club-msg").textContent = "Essa formação não pode ser montada com o plantel disponível."; read(); };
$("auto-plan").onclick = () => { wasm.human_formation(Number($("formation").value)); read(); };
$("change-club").onchange = (e) => {
  const [di, name] = e.target.value.split("|");
  // the table is sorted by points, so find the club index by asking the squad order of the division list
  const idx = state.divisions[Number(di)].table.findIndex((r) => r.name === name);
  wasm.human_set(Number(di), indexInList(Number(di), name, idx));
  division = Number(di);
  read();
};
$("save-now").onclick = () => saveNow(false);
$("load-now").onclick = () => { const t = store.get(); if (t) loadText(t); };
$("export").onclick = () => {
  const blob = new Blob([saveText()], { type: "text/plain" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `elifoot98-${state.year}-j${state.round}.txt`;
  a.click();
  URL.revokeObjectURL(a.href);
};
$("import").onchange = async (e) => { const f = e.target.files[0]; if (f) loadText(await f.text()); e.target.value = ""; };
$("import-e98").onchange = async (e) => { const f = e.target.files[0]; if (f) importE98(new Uint8Array(await f.arrayBuffer())); e.target.value = ""; };
$("play-round").onclick = () => play("world_play_round");
$("play-season").onclick = () => play("world_play_season");
$("new-world").onclick = newWorld;
$("eft").onchange = (e) => e.target.files.length && loadEfts([...e.target.files]);
$("use-fiction").onclick = () => { useReal = false; $("use-fiction").hidden = true; $("eft").value = ""; $("eft-status").textContent = "Equipas fictícias."; newWorld(); };
boot().catch((e) => { document.body.insertAdjacentHTML("afterbegin", `<p style="padding:16px;color:#b3261e">Falha ao carregar o WebAssembly: ${esc(e.message)}</p>`); });
