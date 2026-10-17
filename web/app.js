// Loads the WebAssembly core and drives the UI. The core exposes a plain C ABI and returns the world as JSON.
const ICON = { goal: "⚽", annulled: "⊘", red: "🟥", injury: "✚" };
const LABEL = { goal: "Golo", annulled: "Golo anulado", red: "Expulsão", injury: "Lesão" };

let wasm, state, division = 0, selectedMatch = null, useReal = false;
const $ = (id) => document.getElementById(id);

async function boot() {
  const res = await fetch("elifoot_wasm.wasm");
  const bytes = await res.arrayBuffer();
  $("wasm-size").textContent = (bytes.byteLength / 1024).toFixed(0) + " KB";
  const { instance } = await WebAssembly.instantiate(bytes, {});
  wasm = instance.exports;
  newWorld();
}

function read() {
  const ptr = wasm.world_json_ptr(), len = wasm.world_json_len();
  state = JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len)));
  render();
}

function newWorld() {
  selectedMatch = null;
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
  const evs = m.events.length
    ? m.events.map((e) => `<div class="ev ${e.side ? "away" : ""}"><span class="m">${e.m}'</span><span title="${LABEL[e.kind]}">${ICON[e.kind]}</span>` +
        `<span>${esc(e.player)} <small>(${esc(e.side ? m.away : m.home)})</small></span></div>`).join("")
    : `<p class="empty">Sem acontecimentos.</p>`;
  box.innerHTML = `<h3>${esc(m.home)} ${m.hg} – ${m.ag} ${esc(m.away)}</h3>${evs}`;
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
  $("season").innerHTML = `<p><b>Campeão da 1ª Divisão:</b> ${esc(champ.name)} — ${champ.pts} pontos (${champ.w}V ${champ.d}E ${champ.l}D, ${champ.gf}–${champ.ga}).</p>` +
    `<div class="moves"><div><h3>Subidas</h3><ul>${ups}</ul></div><div><h3>Descidas</h3><ul>${downs}</ul></div></div>`;
}

function render() {
  $("mode").textContent = useReal ? "equipas e jogadores reais dos teus ficheiros .EFT" : "clubes e jogadores fictícios";
  $("year").textContent = state.year;
  $("round").textContent = `${state.round} / ${state.rounds}`;
  renderTabs();
  renderTable();
  renderResults();
  renderSeason();
}

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

$("play-round").onclick = () => play("world_play_round");
$("play-season").onclick = () => play("world_play_season");
$("new-world").onclick = newWorld;
$("eft").onchange = (e) => e.target.files.length && loadEfts([...e.target.files]);
$("use-fiction").onclick = () => { useReal = false; $("use-fiction").hidden = true; $("eft").value = ""; $("eft-status").textContent = "Equipas fictícias."; newWorld(); };
boot().catch((e) => { document.body.insertAdjacentHTML("afterbegin", `<p style="padding:16px;color:#b3261e">Falha ao carregar o WebAssembly: ${esc(e.message)}</p>`); });
