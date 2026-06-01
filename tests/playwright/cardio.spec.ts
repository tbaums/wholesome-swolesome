import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8080';

function scheduledWorkout(date: string) {
  return {
    id: 'w-cardio',
    date,
    name: 'Cardio Day',
    rationale: 'Easy recovery run',
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

function cardioHistoryEntry(date: string) {
  return {
    id: 'h-cardio',
    date,
    created_at: `${date}T12:00:00.000Z`,
    exercise_name: 'Jogging, Treadmill',
    exercise_id: 'Jogging_Treadmill',
    session_id: 's-cardio',
    day_id: 'w-cardio',
    day_name: 'Cardio Day',
    target_sets: 1,
    reps_min: 30,
    reps_max: 30,
    sets: [
      { set_number: 1, reps: 30, weight: 7, completed: true, completed_date: date },
    ],
    finalized: true,
  };
}

async function freshWithDate(page: import('@playwright/test').Page) {
  await enableDateMock(page, MOCK_NOW);
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate(() => localStorage.clear());
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

test.describe('Cardio session UI', () => {
  test('cardio exercise shows min/RPE inputs instead of weight/reps', async ({ page }) => {
    await freshWithDate(page);

    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledWorkout(TODAY)]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    // Start workout
    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    // Open the exercise accordion
    const chevron = page.locator('.exercise-chevron').first();
    await chevron.click();
    await page.waitForFunction(() => {
      const body = document.querySelector('.exercise-body');
      return body?.classList.contains('open');
    });

    // Library loads async — wait for the cardio UI to appear
    const setRow = page.locator('.set-row').first();
    await expect(setRow.locator('.set-x')).toHaveText('@', { timeout: 10_000 });

    // Check placeholders: "min" and "RPE"
    const inputs = setRow.locator('.set-num-input');
    await expect(inputs.first()).toHaveAttribute('placeholder', 'min');
    await expect(inputs.last()).toHaveAttribute('placeholder', 'RPE');
  });

  test('exercise meta shows minutes instead of sets x reps for cardio', async ({ page }) => {
    await freshWithDate(page);

    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledWorkout(TODAY)]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    // The meta line should say "30 min" instead of "1 sets × 30–30 reps"
    await expect(page.locator('.ex-card').first()).toContainText('30 min', { timeout: 10_000 });
  });
});

test.describe('Cardio history detail', () => {
  test('session detail shows Min/Intensity headers for cardio entries', async ({ page }) => {
    await freshPage(page);

    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([cardioHistoryEntry('2026-05-20')]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await page.locator('.history-item').first().click();
    await page.waitForSelector('.progress-table');

    // Library loads async — wait for cardio-aware headers
    await expect(page.locator('.progress-table th').nth(1)).toHaveText('Min', { timeout: 10_000 });
    await expect(page.locator('.progress-table th').nth(2)).toHaveText('Intensity');

    // Values: reps=30 should be in the Min column, weight=7 in the Intensity column
    const firstRow = page.locator('.progress-table tbody tr').first();
    const cells = firstRow.locator('td');
    await expect(cells.nth(1)).toHaveText('30');
    await expect(cells.nth(2)).toHaveText('7');
  });
});

test.describe('Cardio actuals import card', () => {
  function scheduledZoneWorkout(date: string) {
    return {
      id: 'w-zone',
      date,
      name: 'Zone 2 + Z4 intervals',
      rationale: '',
      source: 'Coach',
      exercises: [
        {
          library_id: 'Running_Treadmill',
          name: 'Running, Treadmill',
          target_sets: 1,
          reps_min: 29,
          reps_max: 29,
          rest_seconds: 0,
          notes: null,
          target_zones: [
            { zone: 1, minutes: 13 },
            { zone: 4, minutes: 16 },
          ],
        },
      ],
      created_at: '2026-06-09T20:00:00.000Z',
    };
  }

  test('copy button puts the prompt onto the clipboard with the right library_id', async ({
    page,
  }) => {
    // Stub navigator.clipboard.writeText into a window-scoped variable. WebKit
    // refuses the `clipboard-write` permission in headless contexts (CI uses
    // WebKit/iPhone 15), so we intercept the call instead of granting perms.
    await page.addInitScript(() => {
      (window as unknown as { __clipboard: string }).__clipboard = '';
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: {
          writeText: (text: string) => {
            (window as unknown as { __clipboard: string }).__clipboard = text;
            return Promise.resolve();
          },
          readText: () =>
            Promise.resolve((window as unknown as { __clipboard: string }).__clipboard),
        },
      });
    });

    await enableDateMock(page, MOCK_NOW);
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());
    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledZoneWorkout(TODAY)]),
    );
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    // The cardio-import card should be visible.
    const card = page.locator('.cardio-import-card');
    await expect(card).toBeVisible();

    // Prompt block contains the actual library_id (not the <library_id> placeholder).
    await expect(card.locator('.ci-prompt')).toContainText('Running_Treadmill');
    await expect(card.locator('.ci-prompt')).not.toContainText('<library_id>');
    await expect(card.locator('.ci-prompt')).toContainText('cardio_actuals');

    // Tap copy → stub records the prompt text.
    await card.locator('.ci-copy-btn').click();
    await expect(page.locator('.toast')).toContainText('Prompt copied');
    const clipboard = await page.evaluate(
      () => (window as unknown as { __clipboard: string }).__clipboard,
    );
    expect(clipboard).toContain('Running_Treadmill');
    expect(clipboard).toContain('cardio_actuals');
    expect(clipboard).toContain('Apple Health');
  });

  test('importing pasted JSON writes zone_minutes onto the matching set', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());
    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledZoneWorkout(TODAY)]),
    );
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    const card = page.locator('.cardio-import-card');
    await card.locator('textarea').fill(
      '```json\n{"cardio_actuals":{"exercise_id":"Running_Treadmill","zones":[{"zone":1,"minutes":12},{"zone":4,"minutes":18}]}}\n```',
    );
    await card.locator('button').filter({ hasText: 'Import cardio actuals' }).click();

    // Confirmation row appears.
    await expect(card).toContainText('Wrote zone actuals');

    // Verify the active session got the zone_minutes on the last set.
    const stored = await page.evaluate(() => localStorage.getItem('ws_active_session'));
    const session = JSON.parse(stored || '{}');
    const log = session.exercise_logs.find((e: { exercise_id: string }) => e.exercise_id === 'Running_Treadmill');
    expect(log).toBeTruthy();
    const lastSet = log.sets[log.sets.length - 1];
    expect(lastSet.zone_minutes).toEqual([
      { zone: 1, minutes: 12 },
      { zone: 4, minutes: 18 },
    ]);
  });
});
