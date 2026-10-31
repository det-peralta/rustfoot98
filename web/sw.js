// Service worker: the game (classic.html), the team editor (editor.html) and the older page (legacy.html) work offline.
// Network first: a page that can reach the server always gets the current files (no stale copy hides an update); what it fetched is
// kept, and answers when the network does not. That includes the skin pack in skin/ (the user's own files, kept in this browser only).
// Bump VERSION when the precached list changes; old caches are removed on activation.
const VERSION = "v5";
const CACHE = `elifoot98-rs-${VERSION}`;
const SHELL = [
  "./", "index.html", "classic.html", "classic.js", "editor.html", "editor.js", "pack.js", "softkb.js", "rustfoot_ui.wasm",
  "legacy.html", "style.css", "app.js", "rustfoot_wasm.wasm", "manifest.webmanifest", "icon.svg", "icon-192.png", "icon-512.png",
];

// the skin pack, when the server has one (skin/index.json lists its files): a first visit fetches it before the worker is in control
async function skinFiles() {
  try {
    const res = await fetch("skin/index.json");
    if (!res.ok) return [];
    const l = await res.json();
    return ["index.json", l.fonts, l.glyphs, l.forms, l.country, l.referee, ...(l.teams || [])].filter(Boolean).map((n) => "skin/" + n);
  } catch {
    return [];
  }
}

self.addEventListener("install", (event) => {
  event.waitUntil((async () => {
    const c = await caches.open(CACHE);
    await c.addAll(SHELL);
    await Promise.all((await skinFiles()).map((f) => c.add(f).catch(() => {})));
    await self.skipWaiting();
  })());
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("elifoot98-rs-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET" || new URL(req.url).origin !== self.location.origin) return;
  event.respondWith(
    caches.open(CACHE).then(async (cache) => {
      try {
        const res = await fetch(req);
        if (res.ok) cache.put(req, res.clone());
        return res;
      } catch {
        return (await cache.match(req, { ignoreSearch: true })) ||
          new Response("Sem ligação e sem cópia guardada.", { status: 503, headers: { "Content-Type": "text/plain; charset=utf-8" } });
      }
    })
  );
});
