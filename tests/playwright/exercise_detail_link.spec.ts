import { test, expect } from '@playwright/test';
import { enableDateMock, localDateStr } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8081';

function scheduledWorkout(date: string) {
  return {
    id: 'w-link',
    date,
    name: 'Push Day',
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

async function freshWithWorkout(page: import('@playwright/test').Page) {
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
}

test.describe('Exercise detail link from session', () => {
  test('info button is visible on exercise cards in session view', async ({ page }) => {
    await freshWithWorkout(page);
    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    await expect(page.locator('.ex-info-btn').first()).toBeVisible();
  });

  test('tapping info button navigates to library detail', async ({ page }) => {
    await freshWithWorkout(page);
    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    await page.locator('.ex-info-btn').first().click();
    await page.waitForSelector('.lib-detail-images', { timeout: 8000 });

    await expect(page.locator('.page-title')).toContainText('Bench Press');
  });

  test('back button from library detail returns to session', async ({ page }) => {
    await freshWithWorkout(page);
    await page.locator('button').filter({ hasText: 'Start workout' }).click();
    await page.waitForSelector('.ex-card');

    await page.locator('.ex-info-btn').first().click();
    await page.waitForSelector('.lib-detail-images', { timeout: 8000 });

    await page.locator('.back-btn').click();
    await page.waitForSelector('.ex-card');

    await expect(page.locator('.ex-card').first()).toBeVisible();
  });
});
