import { test, expect, Page, Route } from '@playwright/test';
import { BASE } from './helpers';

// Exercises the boot-pull effect (app.rs:94-131) and the debounced auto-push
// effect (app.rs:141-196). Each test sets sync config + localStorage *before*
// navigating, then installs route handlers so the boot fetch is intercepted.

const GH_GLOB = 'https://api.github.com/**';

function ghContentsResponse(opts: { sha: string; state: object }) {
  const json = JSON.stringify(opts.state);
  const content = Buffer.from(json, 'utf8').toString('base64');
  return {
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({
      type: 'file', name: 'state.json', content,
      encoding: 'base64', sha: opts.sha,
    }),
  };
}
function ghPutResponse(newSha: string) {
  return {
    status: 200, contentType: 'application/json',
    body: JSON.stringify({ content: { sha: newSha } }),
  };
}

function entry(opts: { id: string; name: string; date: string; weight: number }) {
  return {
    id: opts.id, date: opts.date, created_at: `${opts.date}T10:00:00.000Z`,
    exercise_name: opts.name, exercise_id: `ex-${opts.id}`,
    session_id: null, day_id: null, day_name: null,
    target_sets: 3, reps_min: 5, reps_max: 8,
    sets: [{ set_number: 1, reps: 5, weight: opts.weight, completed: true, completed_date: opts.date }],
    finalized: true,
  };
}

/** Seed localStorage on first nav, then reload so the boot effect sees it. */
async function seedAndReload(page: Page, kv: Record<string, string>) {
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate((entries) => {
    localStorage.clear();
    for (const [k, v] of Object.entries(entries)) localStorage.setItem(k, v);
  }, kv);
  await page.reload();
}

const SYNC_CFG = JSON.stringify({
  token: 'ghp_xxx',
  repo: 'owner/data',
  branch: 'main',
  path: 'state.json',
});

// ── Boot-pull behavior ────────────────────────────────────────────────────────

test.describe('Sync: boot pull', () => {
  test('newer remote timestamp hydrates local history', async ({ page }) => {
    const remoteState = {
      schema_version: 1,
      updated_at: '2026-06-10T12:00:00.000Z',
      plan: null,
      exercise_history: [entry({ id: 'r1', name: 'Remote Press', date: '2026-06-10', weight: 185 })],
      session_drafts: [],
      custom_exercises: [],
    };
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'sha-remote', state: remoteState }));
      } else {
        route.continue();
      }
    });

    // Local is older → should hydrate
    await seedAndReload(page, {
      ws_gh_sync: SYNC_CFG,
      ws_last_push_at: '2026-06-01T00:00:00.000Z',
    });
    await page.waitForSelector('.bottom-nav');

    // Boot pull toast appears, history view shows remote entry
    await expect(page.locator('.toast')).toContainText('Synced from GitHub', { timeout: 10_000 });
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await expect(page.locator('.history-item').first()).toContainText('Remote Press');
  });

  test('older remote timestamp does NOT hydrate (local wins)', async ({ page }) => {
    const remoteState = {
      schema_version: 1,
      updated_at: '2026-06-01T00:00:00.000Z',
      plan: null,
      exercise_history: [entry({ id: 'old', name: 'Stale Remote', date: '2026-06-01', weight: 100 })],
      session_drafts: [],
      custom_exercises: [],
    };
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'sha-old', state: remoteState }));
      } else {
        route.continue();
      }
    });

    const localEntry = entry({ id: 'local', name: 'Local Squat', date: '2026-06-10', weight: 225 });
    await seedAndReload(page, {
      ws_gh_sync: SYNC_CFG,
      ws_last_push_at: '2026-06-10T12:00:00.000Z', // newer than remote
      ws_ex_history: JSON.stringify([localEntry]),
    });
    await page.waitForSelector('.bottom-nav');

    // No hydrate toast within a reasonable window
    await expect(page.locator('.toast')).not.toContainText('Synced from GitHub', { timeout: 3000 });
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await expect(page.locator('.history-item').first()).toContainText('Local Squat');
    await expect(page.locator('.history-item').filter({ hasText: 'Stale Remote' })).toHaveCount(0);
  });

  test('newer remote with empty exercise_history does NOT clobber local history', async ({ page }) => {
    // Regression guard: app.rs:112-120 only hydrates non-empty remote arrays.
    // This protects an offline device from being wiped by an empty remote
    // that happens to have a newer timestamp.
    const remoteState = {
      schema_version: 1,
      updated_at: '2026-06-30T00:00:00.000Z', // newer than local
      plan: null,
      exercise_history: [], // BUT empty
      session_drafts: [],
      custom_exercises: [],
    };
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'sha-empty', state: remoteState }));
      } else {
        route.continue();
      }
    });

    const localEntry = entry({ id: 'local', name: 'Survives', date: '2026-06-10', weight: 200 });
    await seedAndReload(page, {
      ws_gh_sync: SYNC_CFG,
      ws_last_push_at: '2026-06-01T00:00:00.000Z',
      ws_ex_history: JSON.stringify([localEntry]),
    });
    await page.waitForSelector('.bottom-nav');

    // Boot still hydrates the timestamp (so "Synced from GitHub" may appear),
    // but local history must remain intact.
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await expect(page.locator('.history-item').first()).toContainText('Survives');
  });

  test('404 from remote leaves the app on Home with no error toast', async ({ page }) => {
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill({
          status: 404, contentType: 'application/json',
          body: JSON.stringify({ message: 'Not Found' }),
        });
      } else {
        route.continue();
      }
    });
    await seedAndReload(page, { ws_gh_sync: SYNC_CFG });
    await page.waitForSelector('.bottom-nav');
    // Home is the default view; 404 is treated as benign (first-time push)
    await expect(page.locator('.btn.btn-secondary.btn-full').first()).toBeVisible();
    await expect(page.locator('.toast')).not.toContainText('Synced from GitHub', { timeout: 1500 });
  });
});

// ── Debounced auto-push behavior ──────────────────────────────────────────────

test.describe('Sync: debounced auto-push', () => {
  test('a data change triggers a PUT after the debounce window', async ({ page }) => {
    let putCount = 0;
    let lastPutBody: string | null = null;
    await page.route(GH_GLOB, (route: Route) => {
      const req = route.request();
      if (req.method() === 'GET') {
        // Return empty state so boot_done flips without hydration
        route.fulfill(ghContentsResponse({ sha: 'boot-sha', state: {
          schema_version: 1, updated_at: null, plan: null,
          exercise_history: [], session_drafts: [], custom_exercises: [],
        } }));
      } else if (req.method() === 'PUT') {
        putCount++;
        lastPutBody = req.postData();
        route.fulfill(ghPutResponse(`after-${putCount}`));
      } else {
        route.continue();
      }
    });

    await seedAndReload(page, { ws_gh_sync: SYNC_CFG });
    await page.waitForSelector('.bottom-nav');

    // Wait for boot pull to complete so boot_done is true
    await page.waitForTimeout(500);

    // Trigger a data change: create a custom exercise
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.ex-card');
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Sync Test Exercise');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();

    // Auto-push fires 2s after the change
    await expect.poll(() => putCount, { timeout: 6000 }).toBeGreaterThanOrEqual(1);
    expect(lastPutBody).not.toBeNull();
    const parsed = JSON.parse(lastPutBody!);
    const decoded = Buffer.from(parsed.content, 'base64').toString('utf8');
    expect(decoded).toContain('Sync Test Exercise');
  });

  test('PUT 409 conflict triggers refetch + retry', async ({ page }) => {
    let getCount = 0;
    let putCount = 0;
    await page.route(GH_GLOB, (route: Route) => {
      const req = route.request();
      if (req.method() === 'GET') {
        getCount++;
        // First GET (boot) returns sha A; second GET (after 409) returns sha B
        const sha = getCount === 1 ? 'sha-A' : 'sha-B';
        route.fulfill(ghContentsResponse({ sha, state: {
          schema_version: 1, updated_at: null, plan: null,
          exercise_history: [], session_drafts: [], custom_exercises: [],
        } }));
      } else if (req.method() === 'PUT') {
        putCount++;
        if (putCount === 1) {
          route.fulfill({
            status: 409, contentType: 'application/json',
            body: JSON.stringify({ message: 'sha mismatch' }),
          });
        } else {
          route.fulfill(ghPutResponse('final-sha'));
        }
      } else {
        route.continue();
      }
    });

    await seedAndReload(page, { ws_gh_sync: SYNC_CFG });
    await page.waitForSelector('.bottom-nav');
    await page.waitForTimeout(500);

    // Trigger a data change
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.ex-card');
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Conflict Retry');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();

    // Conflict-recovery path: PUT (409) → GET (refetch sha) → PUT (200)
    await expect.poll(() => putCount, { timeout: 8000 }).toBeGreaterThanOrEqual(2);
    expect(getCount).toBeGreaterThanOrEqual(2); // boot GET + recovery GET
  });
});
