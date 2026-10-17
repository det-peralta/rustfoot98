// Service worker: precaches the app shell and the WebAssembly core so the game works offline.
// Bump VERSION when the files change; old caches are removed on activation.
const VERSION = "v1";
const CACHE = `elifoot98-rs-${VERSION}`;
const SHELL = ["./", "index.html", "style.css", "app.js", "elifoot_wasm.wasm", "manifest.webmanifest", "icon.svg", "icon-192.png", "icon-512.png"];

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("elifoot98-rs-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

// Stale-while-revalidate for same-origin GETs: answer from the cache at once, refresh it in the background.
self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET" || new URL(req.url).origin !== self.location.origin) return;
  event.respondWith(
    caches.open(CACHE).then(async (cache) => {
      const cached = await cache.match(req, { ignoreSearch: true });
      const network = fetch(req).then((res) => { if (res.ok) cache.put(req, res.clone()); return res; }).catch(() => null);
      return cached || (await network) || new Response("Sem ligação e sem cópia guardada.", { status: 503, headers: { "Content-Type": "text/plain; charset=utf-8" } });
    })
  );
});
