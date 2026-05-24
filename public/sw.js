const CACHE = "swolesome-v10";

const PRECACHE = [
  "./",
  "./index.html",
];

self.addEventListener("install", (e) => {
  e.waitUntil(
    caches.open(CACHE).then((c) => c.addAll(PRECACHE))
  );
  self.skipWaiting();
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))
    )
  );
  self.clients.claim();
});

self.addEventListener("fetch", (e) => {
  const { request } = e;
  const url = new URL(request.url);

  // Never intercept cross-origin requests (sync to api.github.com etc).
  // The catch-all cache-first branch below would otherwise poison sync.
  if (url.origin !== self.location.origin) return;

  // Navigation: network-first, cache fallback for offline.
  if (request.mode === "navigate") {
    e.respondWith(
      fetch(request).catch(() => caches.match("./index.html"))
    );
    return;
  }

  // /data/* (exercise library + images): network-first, bypass the HTTP cache
  // so a stale entry from a broken earlier load can't poison us. Cache the
  // fresh response for offline; fall back to SW cache only when network fails.
  if (url.pathname.startsWith("/data/")) {
    e.respondWith(
      fetch(request, { cache: "reload" })
        .then((response) => {
          if (response.ok && response.headers.get("content-type")?.includes("json") ||
              response.ok && url.pathname.match(/\.(jpg|png|gif|webp)$/i)) {
            const clone = response.clone();
            caches.open(CACHE).then((c) => c.put(request, clone));
          }
          return response;
        })
        .catch(() => caches.match(request))
    );
    return;
  }

  // Everything else (wasm, js, css, html): cache-first.
  e.respondWith(
    caches.match(request).then((cached) => {
      if (cached) return cached;
      return fetch(request).then((response) => {
        if (response.ok) {
          const clone = response.clone();
          caches.open(CACHE).then((c) => c.put(request, clone));
        }
        return response;
      });
    })
  );
});
