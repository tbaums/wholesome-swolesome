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
