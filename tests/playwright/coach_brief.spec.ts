import { test, expect } from '@playwright/test';
import { enableDateMock, localDateStr } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);

function tomorrow(isoStr: string): string {
  const d = new Date(isoStr);
  d.setDate(d.getDate() + 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

const TOMORROW = tomorrow(MOCK_NOW);

const BASE = 'http://localhost:8080';

async function freshWithMock(page: import('@playwright/test').Page) {
  await enableDateMock(page, MOCK_NOW);
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate(() => localStorage.clear());
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

async function openCoachBrief(page: import('@playwright/test').Page) {
  await page.locator('button').filter({ hasText: 'Generate workout with Claude' }).click();
  await page.waitForSelector('input[type="date"]');
}

test.describe('Coach Brief default date', () => {
  test('defaults to today when no workout is scheduled', async ({ page }) => {
    await freshWithMock(page);
    await openCoachBrief(page);

    await expect(page.locator('input[type="date"]')).toHaveValue(TODAY);
  });

  test('defaults to tomorrow when today already has a scheduled workout', async ({ page }) => {
    await freshWithMock(page);

    await page.evaluate(
      (date) => {
        const scheduled = [{
          id: 'w-test',
          date,
          name: 'Test Workout',
          rationale: '',
          source: 'Coach',
          exercises: [],
          created_at: '2026-06-10T08:00:00.000Z',
        }];
        localStorage.setItem('ws_scheduled_workouts', JSON.stringify(scheduled));
      },
      TODAY,
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await openCoachBrief(page);

    await expect(page.locator('input[type="date"]')).toHaveValue(TOMORROW);
  });
});
