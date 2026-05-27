import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

function todayStr(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function tomorrowStr(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

async function openCoachBrief(page: import('@playwright/test').Page) {
  await page.locator('button').filter({ hasText: 'Generate workout with Claude' }).click();
  await page.waitForSelector('.coach-packet-pre');
}

test.describe('Coach Brief default date', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('defaults to today when no workout is scheduled', async ({ page }) => {
    await openCoachBrief(page);

    const dateText = page.locator('.fw-600').filter({ hasText: /^\d{4}-\d{2}-\d{2}$/ });
    await expect(dateText).toHaveText(todayStr());
  });

  test('defaults to tomorrow when today already has a scheduled workout', async ({ page }) => {
    const today = todayStr();
    await page.evaluate(
      (date) => {
        const scheduled = [{
          id: 'w-test',
          date,
          name: 'Test Workout',
          rationale: '',
          source: 'Coach',
          exercises: [],
          created_at: new Date().toISOString(),
        }];
        localStorage.setItem('ws_scheduled_workouts', JSON.stringify(scheduled));
      },
      today,
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await openCoachBrief(page);

    const dateText = page.locator('.fw-600').filter({ hasText: /^\d{4}-\d{2}-\d{2}$/ });
    await expect(dateText).toHaveText(tomorrowStr());
  });
});
