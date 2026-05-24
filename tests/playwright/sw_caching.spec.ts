import { test, expect, Route } from '@playwright/test';
import { BASE } from './helpers';

// Regression: public/sw.js previously had a cache-first catch-all branch that
// matched ALL requests, including cross-origin ones to api.github.com. That
// silently poisoned every subsequent sync GET — Test Connection and Pull from
// GitHub kept returning the first-ever cached response, even after the remote
// state changed. The fix is the early return for cross-origin URLs at the top
// of the fetch handler.
//
// This test deliberately runs with the service worker ALLOWED (the project's
// default config blocks it). Without SW running, the bug can't reproduce.

test.use({ serviceWorkers: 'allow' });

test.describe('Service Worker: cross-origin requests bypass the cache', () => {
  test('two fetches to the same cross-origin URL see fresh server-side state', async ({ page, context }) => {
    const PROBE = 'https://probe.example.test/sw-cache-regression';

    let counter = 0;
    await context.route(PROBE, (route: Route) => {
      counter += 1;
      route.fulfill({
        status: 200,
        contentType: 'text/plain',
        headers: { 'access-control-allow-origin': '*' },
        body: `resp-${counter}`,
      });
    });

    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    // sw.js registers on window.load; with skipWaiting + clients.claim it takes
    // control of this page on its first activation. Wait until that's true.
    await page.waitForFunction(
      () => navigator.serviceWorker?.controller !== null,
      undefined,
      { timeout: 15_000 },
    );

    const first = await page.evaluate(async (url) => (await fetch(url)).text(), PROBE);
    const second = await page.evaluate(async (url) => (await fetch(url)).text(), PROBE);

    // With the OLD cache-first-everything branch, the SW caches resp-1 and
    // returns it for every subsequent fetch — so `second` would equal `first`.
    expect(first).toBe('resp-1');
    expect(second).toBe('resp-2');

    // Belt-and-braces: confirm nothing cross-origin landed in any SW cache.
    const crossOriginCached = await page.evaluate(async (url) => {
      const names = await caches.keys();
      for (const name of names) {
        const c = await caches.open(name);
        if (await c.match(url)) return true;
      }
      return false;
    }, PROBE);
    expect(crossOriginCached).toBe(false);
  });
});
