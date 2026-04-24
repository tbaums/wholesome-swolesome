import { test, expect } from '@playwright/test';
import {
  freshPage,
  startWorkout,
  openExercise,
  fillSet,
  markSetDone,
  completeAllSets,
} from './helpers';

test.describe('Session flow', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  // 1
  test('tapping a day navigates to the session view', async ({ page }) => {
    await startWorkout(page, 0);
    await expect(page.locator('.ex-card').first()).toBeVisible();
    await expect(page.locator('.btn-finish')).toBeVisible();
  });

  // 2
  test('exercise cards are initially collapsed', async ({ page }) => {
    await startWorkout(page, 0);
    const bodies = page.locator('.exercise-body');
    const count = await bodies.count();
    for (let i = 0; i < count; i++) {
      const hasOpen = await bodies.nth(i).evaluate((el) =>
        el.classList.contains('open'),
      );
      expect(hasOpen).toBe(false);
    }
  });

  // 3
  test('chevron click opens the accordion', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    const body = page.locator('.exercise-body').first();
    await expect(body).toHaveClass(/open/);
    await expect(body.locator('.set-row').first()).toBeVisible();
  });

  // 4
  test('chevron click again closes the accordion', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    // Click chevron a second time to close
    await page.locator('.exercise-chevron').first().click();
    await page.waitForFunction(() => {
      const body = document.querySelector('.exercise-body');
      return body && !body.classList.contains('open');
    });
    const hasOpen = await page
      .locator('.exercise-body')
      .first()
      .evaluate((el) => el.classList.contains('open'));
    expect(hasOpen).toBe(false);
  });

  // 5 — key regression: accordion must stay open when a set is marked done
  test('marking a set done does not collapse the accordion', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    // Mark first set done inside the open accordion
    const firstBody = page.locator('.exercise-body').first();
    await firstBody.locator('.set-done-btn').first().click();
    // Accordion must still be open
    const stillOpen = await firstBody.evaluate((el) =>
      el.classList.contains('open'),
    );
    expect(stillOpen).toBe(true);
  });

  // 6
  test('set-done button gains done class when clicked', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    const btn = page
      .locator('.exercise-body')
      .first()
      .locator('.set-done-btn')
      .first();
    await btn.click();
    await expect(btn).toHaveClass(/done/);
  });

  // 7
  test('weight and reps inputs accept typed values', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    await fillSet(page, 0, '135', '8');
    const row = page.locator('.set-row').first();
    await expect(row.locator('.set-num-input').first()).toHaveValue('135');
    await expect(row.locator('.set-num-input').nth(1)).toHaveValue('8');
  });

  // 8
  test('add set button appends a new set row', async ({ page }) => {
    await startWorkout(page, 0);
    await openExercise(page, 0);
    const body = page.locator('.exercise-body').first();
    const before = await body.locator('.set-row').count();
    await body.locator('.add-set-btn').click();
    const after = await body.locator('.set-row').count();
    expect(after).toBe(before + 1);
  });

  // 9
  test('finish button shows checkmark only when all sets are done', async ({ page }) => {
    // Use Day 4 (Recovery) — fewest sets
    await startWorkout(page, 3);
    // Before completing — no checkmark
    await expect(page.locator('.btn-finish')).not.toContainText('✓');
    await completeAllSets(page);
    await expect(page.locator('.btn-finish')).toContainText('✓');
  });

  // 10
  test('exercise complete badge appears when all sets for that exercise are done', async ({
    page,
  }) => {
    await startWorkout(page, 3); // Day 4: first exercise has 3 sets
    await openExercise(page, 0);
    const body = page.locator('.exercise-body').first();
    const doneBtns = body.locator('.set-done-btn');
    const count = await doneBtns.count();
    for (let i = 0; i < count; i++) {
      await doneBtns.nth(i).click();
    }
    await expect(
      page.locator('.ex-card').first().locator('.exercise-complete-badge'),
    ).toBeVisible();
  });

  // 11
  test('workout nav button goes to day list while a session is active', async ({ page }) => {
    await startWorkout(page, 0);
    await page.locator('.nav-btn').filter({ hasText: 'Workout' }).click();
    // Should be back on the home/day-list screen
    await expect(page.locator('.btn.btn-secondary.btn-full').first()).toBeVisible();
    await expect(page.locator('.btn-finish')).not.toBeVisible();
  });

  // 12
  test('finish workout saves session and navigates to history', async ({ page }) => {
    await startWorkout(page, 3);
    await completeAllSets(page);
    await page.locator('.btn-finish').click();
    // Should leave the session view and activate History nav
    await expect(page.locator('.btn-finish')).not.toBeVisible();
    await expect(
      page.locator('.nav-btn').filter({ hasText: 'History' }),
    ).toHaveClass(/active/);
  });
});
