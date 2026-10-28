import { test, expect, Page } from '@playwright/test';
import { BASE, enableDateMock, freshPage, localDateStr, openExercise } from './helpers';

// The optional direct-to-Anthropic path. Every test here mocks
// api.anthropic.com via page.route — no real key is ever used, and CI never
// makes an outbound call.

const FAKE_KEY = 'sk-ant-test-not-a-real-key';

/** A 1×1 PNG. `read_image_file` never decodes the image — it checks the media
 *  type and the byte count — but a real PNG keeps the fixture honest. */
const PNG_B64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==';
const PNG_BYTES = Buffer.from(PNG_B64, 'base64');

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
    await page.waitForSelector('.brief-copy-btn');
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

  test('an attached VO2 max screenshot rides along with the brief as a base64 image block', async ({ page }) => {
    const captured = { bodies: [] as string[], headers: [] as Record<string, string>[] };
    await mockMessages(page, WORKOUT_JSON, 'end_turn', captured);
    await saveKey(page);

    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-vitals-file').setInputFiles({
      name: 'vo2.png',
      mimeType: 'image/png',
      buffer: PNG_BYTES,
    });
    // The file is read to base64 before the call, so the card confirms it first.
    await expect(page.locator('.claude-generate-card')).toContainText('Attached: vo2.png');

    await page.locator('.claude-generate-btn').click();
    await expect(page.locator('text=✓ Imported')).toBeVisible({ timeout: 10_000 });

    // Image block first, then the brief — one request, not two.
    expect(captured.bodies).toHaveLength(1);
    const content = JSON.parse(captured.bodies[0]).messages[0].content;
    expect(content[0].type).toBe('image');
    expect(content[0].source).toMatchObject({
      type: 'base64',
      media_type: 'image/png',
      data: PNG_B64,
    });
    expect(content[1].type).toBe('text');
    expect(content[1].text).toContain('Wholesome Swolesome — Coach Brief');
  });

  test('a non-image picked for vitals is refused locally and nothing is attached', async ({ page }) => {
    let calls = 0;
    await page.route('https://api.anthropic.com/**', async (route) => {
      calls++;
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    });
    await saveKey(page);

    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.claude-generate-card');
    await page.locator('.claude-vitals-file').setInputFiles({
      name: 'notes.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('not a screenshot'),
    });

    await expect(page.locator('text=/isn.t a usable image/')).toBeVisible();
    await expect(page.locator('.claude-generate-card')).not.toContainText('Attached:');
    // Reading the file never reaches the network.
    expect(calls).toBe(0);
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

// ── Screenshot → zone minutes (AC3) ──────────────────────────────────────────
// The picked-file chain only exists in a browser: the <input type="file">
// change event, `input.files()`, `File::array_buffer()`, then the base64 image
// block. The Rust tests stop at `ImagePart::from_bytes`, so these drive the
// whole thing through the session's cardio import card.

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);

/** What the prompt asks Claude for: a fenced block wrapping `cardio_actuals`. */
const CARDIO_ACTUALS = [
  '```json',
  JSON.stringify({
    cardio_actuals: {
      exercise_id: 'Jogging_Treadmill',
      zones: [
        { zone: 2, minutes: 18 },
        { zone: 3, minutes: 9 },
      ],
      estimated_rpe: 5,
    },
  }),
  '```',
].join('\n');

function cardioWorkout(date: string) {
  return {
    id: 'w-cardio',
    date,
    name: 'Cardio Day',
    rationale: 'Easy aerobic',
    source: 'Coach',
    exercises: [
      {
        library_id: 'Jogging_Treadmill',
        name: 'Jogging, Treadmill',
        target_sets: 1,
        reps_min: 30,
        reps_max: 30,
        rest_seconds: 0,
        notes: null,
      },
    ],
    created_at: '2026-06-09T20:00:00.000Z',
  };
}

/** Seed a saved key plus a cardio workout for today, then start the session.
 *  The key is seeded rather than typed — the UI round-trip has its own test
 *  above, and the import card reads the key when the session view mounts. */
async function startCardioSession(page: Page) {
  await page.evaluate(
    (seed) => {
      localStorage.setItem('ws_scheduled_workouts', seed.workouts);
      localStorage.setItem('ws_claude_api', seed.claude);
    },
    {
      workouts: JSON.stringify([cardioWorkout(TODAY)]),
      claude: JSON.stringify({ api_key: FAKE_KEY }),
    },
  );
  await page.reload();
  await page.waitForSelector('.bottom-nav');
  await page.locator('button').filter({ hasText: 'Start workout' }).click();
  await page.waitForSelector('.ex-card');
  await expect(page.locator('.ci-file-input')).toBeVisible();
}

async function lastSet(page: Page) {
  const session = await page.evaluate(() =>
    JSON.parse(localStorage.getItem('ws_active_session') ?? 'null'),
  );
  const sets = session.exercise_logs[0].sets;
  return sets[sets.length - 1];
}

test.describe('Claude API: Apple Health screenshot → zone minutes', () => {
  test.beforeEach(async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
  });

  test('picking a screenshot writes the zone minutes onto the set', async ({ page }) => {
    const captured = { bodies: [] as string[], headers: [] as Record<string, string>[] };
    await mockMessages(page, CARDIO_ACTUALS, 'end_turn', captured);
    await startCardioSession(page);

    const card = page.locator('.cardio-import-card');
    await card.locator('.ci-file-input').setInputFiles({
      name: 'apple-health.png',
      mimeType: 'image/png',
      buffer: PNG_BYTES,
    });

    await expect(card).toContainText('Wrote zone actuals', { timeout: 10_000 });

    // The request carried the screenshot ahead of the cardio prompt, with the
    // session's library_id baked in.
    expect(captured.bodies).toHaveLength(1);
    const content = JSON.parse(captured.bodies[0]).messages[0].content;
    expect(content[0].type).toBe('image');
    expect(content[0].source).toMatchObject({
      type: 'base64',
      media_type: 'image/png',
      data: PNG_B64,
    });
    expect(content[1].text).toContain('Apple Health');
    expect(content[1].text).toContain('Jogging_Treadmill');

    // The minutes land in the zone grid (Z1…Z5, so Z2 is index 1)…
    await openExercise(page, 0);
    const zones = page.locator('.zone-input');
    await expect(zones.nth(1)).toHaveValue('18');
    await expect(zones.nth(2)).toHaveValue('9');

    // …and on the persisted set, with Claude's inferred RPE as the weight.
    const set = await lastSet(page);
    expect(set.zone_minutes).toEqual([
      { zone: 2, minutes: 18 },
      { zone: 3, minutes: 9 },
    ]);
    expect(set.weight).toBe(5);
  });

  test('a non-image file is refused locally — no request, no import', async ({ page }) => {
    let calls = 0;
    await page.route('https://api.anthropic.com/**', async (route) => {
      calls++;
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    });
    await startCardioSession(page);

    const card = page.locator('.cardio-import-card');
    await card.locator('.ci-file-input').setInputFiles({
      name: 'notes.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('not a screenshot'),
    });

    await expect(card).toContainText("isn't a usable image");
    expect(calls).toBe(0);
    expect((await lastSet(page)).zone_minutes ?? null).toBeNull();
  });

  test('a file one byte over the 5 MB cap is refused locally — no request', async ({ page }) => {
    let calls = 0;
    await page.route('https://api.anthropic.com/**', async (route) => {
      calls++;
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    });
    await startCardioSession(page);

    const card = page.locator('.cardio-import-card');
    await card.locator('.ci-file-input').setInputFiles({
      name: 'huge.png',
      mimeType: 'image/png',
      buffer: Buffer.alloc(5 * 1024 * 1024 + 1, 7),
    });

    await expect(card).toContainText("isn't a usable image");
    expect(calls).toBe(0);
    expect((await lastSet(page)).zone_minutes ?? null).toBeNull();
  });
});
