import { test, expect, Page } from '@playwright/test';
import { BASE, freshPage } from './helpers';

// The optional direct-to-Anthropic path. Every test here mocks
// api.anthropic.com via page.route — no real key is ever used, and CI never
// makes an outbound call.

const FAKE_KEY = 'sk-ant-test-not-a-real-key';

const WORKOUT_JSON = JSON.stringify({
  name: 'API Generated Day',
  rationale: 'Returned by the mocked Claude API.',
  exercises: [
    {
      library_id: 'Barbell_Squat',
      name: 'Barbell Squat',
      target_sets: 3,
      reps_min: 5,
      reps_max: 8,
      rest_seconds: 120,
      notes: '',
    },
  ],
});

/** Mock a 200 from /v1/messages whose single text block is `text`. */
async function mockMessages(
  page: Page,
  text: string,
  stopReason = 'end_turn',
  captured?: { bodies: string[]; headers: Record<string, string>[] },
) {
  await page.route('https://api.anthropic.com/**', async (route) => {
    captured?.bodies.push(route.request().postData() ?? '');
    captured?.headers.push(route.request().headers());
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        content: [{ type: 'text', text }],
        stop_reason: stopReason,
      }),
    });
  });
}

async function mockMessagesError(page: Page, status: number, message: string) {
  await page.route('https://api.anthropic.com/**', async (route) => {
    await route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify({ type: 'error', error: { type: 'x', message } }),
    });
  });
}

async function openOptions(page: Page) {
  await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
  await page.locator('.page-header').locator('text=Options').click();
  await page.waitForSelector('.claude-key-card');
}

async function saveKey(page: Page, key = FAKE_KEY) {
  await openOptions(page);
  await page.locator('.claude-key-input').fill(key);
  await page.locator('.claude-save-btn').click();
  await expect(page.locator('.claude-key-status')).toContainText('Saved');
  // Options' back button lands on History; the Coach Brief is reached from Workout.
  await page.locator('.back-btn').click();
  await page.locator('.nav-btn').filter({ hasText: 'Workout' }).click();
  await page.waitForSelector('.today-card');
}

test.describe('Claude API key (optional)', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('the card carries the agreed copy and all three buttons', async ({ page }) => {
    await openOptions(page);
    const card = page.locator('.claude-key-card');
    await expect(card).toContainText('Claude API key (optional)');
    await expect(card).toContainText(
      'Paste your own Anthropic API key to generate workouts and read screenshots without copy/paste. Stored only on this device; never synced.',
    );
    await expect(card.locator('.claude-key-input')).toHaveAttribute('type', 'password');
    await expect(card.locator('.claude-save-btn')).toHaveText('Save');
    await expect(card.locator('.claude-test-btn')).toHaveText('Test key');
    await expect(card.locator('.claude-clear-btn')).toHaveText('Clear');
  });

  test('save / reload / clear round-trips through its own localStorage entry', async ({ page }) => {
    await openOptions(page);
    await page.locator('.claude-key-input').fill(FAKE_KEY);
    await page.locator('.claude-save-btn').click();

    const stored = await page.evaluate(() => localStorage.getItem('ws_claude_api'));
    expect(stored).toContain(FAKE_KEY);
    // Never inside the sync config.
    const sync = await page.evaluate(() => localStorage.getItem('ws_gh_sync'));
    expect(sync ?? '').not.toContain(FAKE_KEY);

    await page.goto(BASE);
    await openOptions(page);
    await expect(page.locator('.claude-key-input')).toHaveValue(FAKE_KEY);

    await page.locator('.claude-clear-btn').click();
    await expect(page.locator('.claude-key-input')).toHaveValue('');
    expect(await page.evaluate(() => localStorage.getItem('ws_claude_api'))).toBeNull();
  });

  test('Test key with an empty field validates inline and makes no request', async ({ page }) => {
    let calls = 0;
    await page.route('https://api.anthropic.com/**', async (route) => {
      calls++;
      await route.fulfill({ status: 200, body: '{}' });
    });
    await openOptions(page);
    await page.locator('.claude-key-input').fill('');
    await page.locator('.claude-test-btn').click();
    await expect(page.locator('.claude-key-status')).toContainText('Paste a key first');
    expect(calls).toBe(0);
  });

  test('Test key reports the API error rather than a bare failure', async ({ page }) => {
    await mockMessagesError(page, 401, 'invalid x-api-key');
    await openOptions(page);
    await page.locator('.claude-key-input').fill(FAKE_KEY);
    await page.locator('.claude-test-btn').click();
    await expect(page.locator('.claude-key-status')).toContainText('401');
  });

  test('with no key saved the Coach Brief is the copy/paste flow, unchanged', async ({ page }) => {
    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.coach-packet-pre');
    await expect(page.locator('.claude-generate-card')).toHaveCount(0);
    await expect(page.locator('button', { hasText: 'Copy brief' })).toBeVisible();
    await expect(page.locator('button', { hasText: 'Import workout' })).toBeVisible();
  });

  test('generate → the returned workout is scheduled, same as a pasted one', async ({ page }) => {
    const captured = { bodies: [] as string[], headers: [] as Record<string, string>[] };
    await mockMessages(page, WORKOUT_JSON, 'end_turn', captured);
    await saveKey(page);

    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-generate-btn').click();

    await expect(page.locator('text=✓ Imported')).toBeVisible({ timeout: 10_000 });
    await page.locator('.back-btn').click();
    await expect(page.locator('.today-card')).toContainText('API Generated Day');

    // The request carries the brief, the pinned model and an explicit effort,
    // and no `thinking` block.
    const body = JSON.parse(captured.bodies[0]);
    expect(body.model).toBe('claude-opus-5-5');
    expect(body.output_config.effort).toBe('medium');
    expect(body.thinking).toBeUndefined();
    expect(body.messages[0].content[0].text).toContain('Wholesome Swolesome — Coach Brief');
    expect(captured.headers[0]['x-api-key']).toBe(FAKE_KEY);
    expect(captured.headers[0]['anthropic-version']).toBe('2023-06-01');
    expect(captured.headers[0]['anthropic-dangerous-direct-browser-access']).toBe('true');
  });

  test('a refusal imports nothing and says so', async ({ page }) => {
    await mockMessages(page, WORKOUT_JSON, 'refusal');
    await saveKey(page);
    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-generate-btn').click();

    await expect(page.locator('text=declined').first()).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('text=✓ Imported')).toHaveCount(0);
    await page.locator('.back-btn').click();
    await expect(page.locator('.today-card')).not.toContainText('API Generated Day');
  });

  test('a max_tokens response imports nothing and says so', async ({ page }) => {
    await mockMessages(page, WORKOUT_JSON, 'max_tokens');
    await saveKey(page);
    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-generate-btn').click();

    await expect(page.locator('text=cut off').first()).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('text=✓ Imported')).toHaveCount(0);
  });

  test('401 surfaces as an error and the copy/paste flow stays usable', async ({ page }) => {
    await mockMessagesError(page, 401, 'invalid x-api-key');
    await saveKey(page);
    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-generate-btn').click();

    await expect(page.locator('text=401').first()).toBeVisible({ timeout: 10_000 });
    // Fallback intact: paste the same JSON by hand and it imports.
    await expect(page.locator('button', { hasText: 'Copy brief' })).toBeVisible();
    await page.locator('textarea').last().fill(WORKOUT_JSON);
    await page.locator('button', { hasText: 'Import workout' }).click();
    await expect(page.locator('text=✓ Imported')).toBeVisible();
  });

  test('a response that fails the parser errors exactly like a bad paste', async ({ page }) => {
    await mockMessages(page, 'not json at all');
    await saveKey(page);
    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-generate-btn').click();
    await expect(page.locator('text=✗').first()).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('text=✓ Imported')).toHaveCount(0);
  });

  test('the key never appears in a sync PUT body', async ({ page }) => {
    const puts: string[] = [];
    await page.route('https://api.github.com/**', async (route) => {
      if (route.request().method() === 'PUT') {
        puts.push(route.request().postData() ?? '');
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ content: { sha: 'deadbeef' } }),
        });
        return;
      }
      await route.fulfill({ status: 404, body: '{}' });
    });
    await mockMessages(page, WORKOUT_JSON);

    await openOptions(page);
    await page.locator('.claude-key-input').fill(FAKE_KEY);
    await page.locator('.claude-save-btn').click();
    // Configure sync and push.
    await page.locator('.card').filter({ hasText: 'Sync (GitHub)' }).locator('input[type="password"]').fill('github_pat_fake');
    await page.locator('button', { hasText: 'Push to GitHub' }).click();
    await expect.poll(() => puts.length, { timeout: 10_000 }).toBeGreaterThan(0);

    for (const body of puts) {
      expect(body).not.toContain(FAKE_KEY);
      // The PUT content is base64; decode it and check there too.
      const content = JSON.parse(body).content as string;
      const decoded = Buffer.from(content, 'base64').toString('utf8');
      expect(decoded).not.toContain(FAKE_KEY);
      expect(decoded).not.toContain('api_key');
    }
  });
});
