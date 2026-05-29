import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr, fillSet, openExercise } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8080';

function scheduledWorkout(date: string) {
  return {
    id: 'w-dec',
    date,
    name: 'Strength Day',
    rationale: '',
    source: 'Coach',
    exercises: [
      {
        library_id: 'Barbell_Bench_Press_-_Medium_Grip',
        name: 'Barbell Bench Press - Medium Grip',
        target_sets: 3,
        reps_min: 8,
        reps_max: 12,
        rest_seconds: 120,
        notes: null,
      },
    ],
    created_at: '2026-06-09T20:00:00.000Z',
  };
}

test.describe('Decimal weight input', () => {
  test('weight input accepts arbitrary decimal values like 137.5', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());
    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledWorkout(TODAY)]),
    );
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');
    await openExercise(page, 0);

    await fillSet(page, 0, '137.5', '8');

    const weightInput = page.locator('.set-row').first().locator('.set-num-input').first();
    await expect(weightInput).toHaveValue('137.5');
  });

  // Regression guard: a reactive `prop:value` formatter used to strip the
  // trailing "." while the user was mid-typing — `.fill()` doesn't catch that
  // because it sets the value atomically, so this test types one character
  // at a time the way a real keyboard does.
  test('per-character typing of "12.5" preserves the decimal point', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());
    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledWorkout(TODAY)]),
    );
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');
    await openExercise(page, 0);

    const weightInput = page.locator('.set-row').first().locator('.set-num-input').first();
    await weightInput.focus();
    await weightInput.pressSequentially('12.5', { delay: 30 });

    await expect(weightInput).toHaveValue('12.5');
  });

  test('decimal weight persists after marking set done', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());
    await page.evaluate(
      (val) => localStorage.setItem('ws_scheduled_workouts', val),
      JSON.stringify([scheduledWorkout(TODAY)]),
    );
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');
    await openExercise(page, 0);

    await fillSet(page, 0, '22.7', '5');
    await page.locator('.set-done-btn').first().evaluate((el: HTMLElement) => el.click());

    const weightInput = page.locator('.set-row').first().locator('.set-num-input').first();
    await expect(weightInput).toHaveValue('22.7');
  });
});
