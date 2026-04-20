import { test, expect } from '@playwright/test';
import { freshPage, startWorkout, completeAllSets } from './helpers';

async function finishWorkout(page: import('@playwright/test').Page) {
  await startWorkout(page, 3); // Day 4: Recovery — fewest sets
  await completeAllSets(page);
  await page.locator('.btn-finish').click();
}

test.describe('History', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('completed session appears in history view', async ({ page }) => {
    await finishWorkout(page);
    // Already on History view after finishing
    await expect(page.locator('.history-item').first()).toContainText(
      'Recovery / Aerobic Base',
    );
  });

  test('clicking a session opens its detail view', async ({ page }) => {
    await finishWorkout(page);
    await page.locator('.history-item').first().click();
    // Detail view shows back button and exercise entries
    await expect(page.locator('.back-btn')).toBeVisible();
    await expect(page.locator('.page-title')).toContainText(
      'Recovery / Aerobic Base',
    );
  });

  test('back button from session detail returns to history list', async ({ page }) => {
    await finishWorkout(page);
    await page.locator('.history-item').first().click();
    await page.locator('.back-btn').click();
    // Back to History: session list re-appears
    await expect(page.locator('.history-item').first()).toBeVisible();
    await expect(
      page.locator('.nav-btn').filter({ hasText: 'History' }),
    ).toHaveClass(/active/);
  });
});
