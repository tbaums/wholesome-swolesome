import { test, expect } from '@playwright/test';
import { freshPage, startWorkout, completeAllSets } from './helpers';

test.describe('Home screen', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('day grid shows all 7 training days', async ({ page }) => {
    const dayBtns = page.locator('.btn.btn-secondary.btn-full');
    await expect(dayBtns).toHaveCount(7);
    await expect(dayBtns.first()).toContainText('Lower A: Glute Tension');
    await expect(dayBtns.nth(6)).toContainText('Whole Body: Light / Polish');
  });

  test('recent sessions shows empty state when no history', async ({ page }) => {
    await expect(page.locator('.empty-icon')).toBeVisible();
    await expect(page.locator('.empty')).toContainText('No sessions yet');
  });

  test('completed session appears in recent sessions', async ({ page }) => {
    // Use Day 4 (Recovery) — fewest sets, fastest to complete
    await startWorkout(page, 3);
    await completeAllSets(page);
    await page.locator('.btn-finish').click();

    // Navigate home
    await page.locator('.nav-btn').filter({ hasText: 'Workout' }).click();

    await expect(page.locator('.history-item').first()).toContainText(
      'Recovery / Aerobic Base',
    );
  });
});
