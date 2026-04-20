import { test, expect } from '@playwright/test';
import { freshPage, startWorkout, openExercise, fillSet, markSetDone } from './helpers';

test.describe('Persistence', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('active session survives a page reload', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    await fillSet(page, 0, '135', '8');
    await markSetDone(page, 0);

    // Hard reload
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    // App should restore to the session view
    await expect(page.locator('.ex-card').first()).toBeVisible();
    // The filled weight and done state should survive
    await openExercise(page, 0);
    await expect(
      page.locator('.set-row').first().locator('.set-num-input').first(),
    ).toHaveValue('135');
    await expect(
      page.locator('.set-row').first().locator('.set-done-btn').first(),
    ).toHaveClass(/done/);
  });

  test('history persists across a page reload', async ({ page }) => {
    // Complete a workout
    await startWorkout(page, 3);
    const exCount = await page.locator('.ex-card').count();
    for (let i = 0; i < exCount; i++) {
      // Open accordion
      await page.locator('.exercise-chevron').nth(i).click();
      await page.waitForFunction(
        (idx: number) =>
          document
            .querySelectorAll('.exercise-body')
            [idx]?.classList.contains('open'),
        i,
      );
      const doneBtns = page
        .locator('.exercise-body')
        .nth(i)
        .locator('.set-done-btn');
      const btnCount = await doneBtns.count();
      for (let j = 0; j < btnCount; j++) {
        await doneBtns.nth(j).click();
      }
    }
    await page.locator('.btn-finish').click();

    // Reload and navigate to history
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await expect(page.locator('.history-item').first()).toContainText(
      'Recovery / Aerobic Base',
    );
  });

  test('in-progress session is preserved when switching days and returning', async ({
    page,
  }) => {
    // Start Day 1 and make some progress
    await startWorkout(page, 0);
    await openExercise(page, 0);
    await fillSet(page, 0, '200', '6');

    // Navigate back to home and start Day 2
    await page.locator('.back-btn').click();
    await page.waitForSelector('.btn.btn-secondary.btn-full');
    await startWorkout(page, 1); // Day 2

    // Go back to Day 1
    await page.locator('.back-btn').click();
    await page.waitForSelector('.btn.btn-secondary.btn-full');
    await page.locator('.btn.btn-secondary.btn-full').first().click();
    await page.waitForSelector('.ex-card');

    // Day 1 session should have the weight we entered
    await openExercise(page, 0);
    await expect(
      page.locator('.set-row').first().locator('.set-num-input').first(),
    ).toHaveValue('200');
  });
});
