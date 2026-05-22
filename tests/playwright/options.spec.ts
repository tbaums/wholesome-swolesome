import { test, expect, Page, Route } from '@playwright/test';
import { freshPage } from './helpers';

// ── Helpers ───────────────────────────────────────────────────────────────────

const GH_GLOB = 'https://api.github.com/**';

/** A SyncedState payload, base64-encoded and wrapped in GitHub's Contents API
 *  response shape. Mirrors what fetch_state() expects from gloo-net. */
function ghContentsResponse(opts: { sha: string; state: object }) {
  const json = JSON.stringify(opts.state);
  // btoa is browser-only, so encode here in node land
  const content = Buffer.from(json, 'utf8').toString('base64');
  return {
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({
      type: 'file',
      name: 'state.json',
      content,
      encoding: 'base64',
      sha: opts.sha,
    }),
  };
}

/** Successful PUT response after a push — returns the new file sha. */
function ghPutResponse(newSha: string) {
  return {
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({
      content: { sha: newSha },
    }),
  };
}

const EMPTY_STATE = {
  schema_version: 1,
  updated_at: null,
  plan: null,
  exercise_history: [],
  session_drafts: [],
  custom_exercises: [],
};

const HYDRATED_STATE = {
  schema_version: 1,
  updated_at: '2026-06-01T10:00:00.000Z',
  plan: null,
  exercise_history: [
    {
      id: 'remote-1',
      date: '2026-06-01',
      created_at: '2026-06-01T10:00:00.000Z',
      exercise_name: 'Remote Squat',
      exercise_id: 'rs1',
      session_id: null,
      day_id: null,
      day_name: null,
      target_sets: 3,
      reps_min: 5,
      reps_max: 8,
      sets: [{ set_number: 1, reps: 5, weight: 225, completed: true, completed_date: '2026-06-01' }],
      finalized: true,
    },
  ],
  session_drafts: [],
  custom_exercises: [],
};

async function goToOptions(page: Page) {
  await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
  await page.locator('button').filter({ hasText: 'Options' }).click();
  // Form fields rendered by OptionsView
  await page.waitForSelector('input[type="password"]');
}

async function fillSyncForm(page: Page, repo = 'owner/data-repo') {
  await page.locator('input[type="password"]').fill('ghp_test_token_xxxxxxxxxxxx');
  // Repo is the first text input on the page
  await page.locator('input[type="text"]').first().fill(repo);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe('Options view — sync configuration', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('navigating from History opens Options with default form values', async ({ page }) => {
    await goToOptions(page);
    // Default branch / path are pre-filled from OptionsView
    await expect(page.locator('input[type="text"]').nth(1)).toHaveValue('main');
    await expect(page.locator('input[type="text"]').nth(2)).toHaveValue('state.json');
    // Default repo placeholder name is filled
    await expect(page.locator('input[type="text"]').first()).toHaveValue('tbaums/wholesome-swolesome-data');
  });

  test('Test connection: success path renders the sha confirmation', async ({ page }) => {
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'abc1234deadbeef', state: EMPTY_STATE }));
      } else {
        route.continue();
      }
    });
    await goToOptions(page);
    await fillSyncForm(page);
    await page.locator('button').filter({ hasText: 'Test connection' }).click();
    await expect(page.locator('text=/Connected.*sha abc1234/')).toBeVisible({ timeout: 5000 });
  });

  test('Test connection: failure renders the error string', async ({ page }) => {
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill({
          status: 401,
          contentType: 'application/json',
          body: JSON.stringify({ message: 'Bad credentials' }),
        });
      } else {
        route.continue();
      }
    });
    await goToOptions(page);
    await fillSyncForm(page);
    await page.locator('button').filter({ hasText: 'Test connection' }).click();
    await expect(page.locator('text=/✗.*401/')).toBeVisible({ timeout: 5000 });
  });

  test('Pull: empty remote shows the "push first" toast', async ({ page }) => {
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'empty', state: EMPTY_STATE }));
      } else {
        route.continue();
      }
    });
    await goToOptions(page);
    await fillSyncForm(page);
    await page.locator('button').filter({ hasText: 'Pull from GitHub' }).click();
    await expect(page.locator('.toast')).toContainText('push first');
  });

  test('Pull: hydrates history from remote and shows success toast', async ({ page }) => {
    await page.route(GH_GLOB, (route: Route) => {
      if (route.request().method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'hydrated', state: HYDRATED_STATE }));
      } else {
        route.continue();
      }
    });
    await goToOptions(page);
    await fillSyncForm(page);
    await page.locator('button').filter({ hasText: 'Pull from GitHub' }).click();
    await expect(page.locator('.toast')).toContainText('Pulled from GitHub');
    // Navigate to History and confirm the remote entry is now visible locally
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await expect(page.locator('.history-item').first()).toContainText('Remote Squat');
  });

  test('Push: success toast and PUT carries base64 content', async ({ page }) => {
    let putBody: string | null = null;
    await page.route(GH_GLOB, (route: Route) => {
      const req = route.request();
      if (req.method() === 'GET') {
        route.fulfill(ghContentsResponse({ sha: 'pre-push', state: EMPTY_STATE }));
      } else if (req.method() === 'PUT') {
        putBody = req.postData();
        route.fulfill(ghPutResponse('after-push'));
      } else {
        route.continue();
      }
    });
    await goToOptions(page);
    await fillSyncForm(page);
    // Need a known sha first so push doesn't fight optimistic concurrency
    await page.locator('button').filter({ hasText: 'Test connection' }).click();
    await expect(page.locator('text=/Connected/')).toBeVisible();
    await page.locator('button').filter({ hasText: 'Push to GitHub' }).click();
    await expect(page.locator('.toast')).toContainText('Pushed to GitHub');
    expect(putBody).not.toBeNull();
    const parsed = JSON.parse(putBody!);
    expect(parsed).toHaveProperty('content');
    expect(parsed).toHaveProperty('branch', 'main');
    expect(parsed).toHaveProperty('sha', 'pre-push'); // optimistic concurrency token
    const decoded = Buffer.from(parsed.content, 'base64').toString('utf8');
    expect(decoded).toMatch(/"schema_version":\s*1/);
  });

  test('Push: 409 conflict surfaces the "try pulling first" toast', async ({ page }) => {
    await page.route(GH_GLOB, (route: Route) => {
      const req = route.request();
      if (req.method() === 'PUT') {
        route.fulfill({
          status: 409,
          contentType: 'application/json',
          body: JSON.stringify({ message: 'sha mismatch' }),
        });
      } else {
        route.continue();
      }
    });
    await goToOptions(page);
    await fillSyncForm(page);
    await page.locator('button').filter({ hasText: 'Push to GitHub' }).click();
    await expect(page.locator('.toast')).toContainText('Conflict');
  });

  test('Clear local data wipes localStorage and shows toast', async ({ page }) => {
    // Seed something to clear
    await page.evaluate(() => {
      localStorage.setItem('ws_ex_history', '[]');
      localStorage.setItem('ws_plan', '{}');
    });
    await goToOptions(page);
    await page.locator('button').filter({ hasText: 'Clear local data' }).click();
    await expect(page.locator('.toast')).toContainText('Local data cleared');
    const remaining = await page.evaluate(() => Object.keys(localStorage));
    expect(remaining).toHaveLength(0);
  });

  test('Help expander toggles open / closed', async ({ page }) => {
    await goToOptions(page);
    // Steps from the help body aren't visible by default
    await expect(page.locator('text=Create the data repo')).not.toBeVisible();
    // The ? button is the help toggle in the Sync (GitHub) card
    await page.locator('button.btn.btn-ghost.btn-sm').filter({ hasText: '?' }).click();
    await expect(page.locator('text=Create the data repo')).toBeVisible();
    await page.locator('button.btn.btn-ghost.btn-sm').filter({ hasText: '✕' }).click();
    await expect(page.locator('text=Create the data repo')).not.toBeVisible();
  });
});
