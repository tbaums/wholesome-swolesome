import { test, expect } from '@playwright/test';
import { freshPage, startWorkout, completeAllSets } from './helpers';

// Helper: click the exercise-level complete button on the nth card (0-indexed).
async function clickCompleteBtn(page: import('@playwright/test').Page, cardIndex = 0) {
  await page.locator('.ex-card').nth(cardIndex).locator('.ex-complete-btn').click();
}

test.describe('Exercises tab', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.ex-card');
  });

  test('exercises nav button is present and activates the tab', async ({ page }) => {
    await expect(
      page.locator('.nav-btn').filter({ hasText: 'Exercises' }),
    ).toHaveClass(/active/);
    await expect(page.locator('.page-title')).toContainText('Exercises');
  });

  test('shows a deduplicated list of exercises', async ({ page }) => {
    const cards = page.locator('.ex-card');
    const count = await cards.count();
    expect(count).toBeGreaterThan(0);
    // Each exercise name should be unique
    const names: string[] = [];
    for (let i = 0; i < count; i++) {
      const name = await cards.nth(i).locator('.card-title').innerText();
      expect(names).not.toContain(name);
      names.push(name);
    }
  });

  test('exercise cards are collapsed by default', async ({ page }) => {
    const bodies = page.locator('.exercise-body');
    const count = await bodies.count();
    for (let i = 0; i < count; i++) {
      const hasOpen = await bodies.nth(i).evaluate((el) =>
        el.classList.contains('open'),
      );
      expect(hasOpen).toBe(false);
    }
  });

  test('chevron opens and closes an exercise card', async ({ page }) => {
    const chevron = page.locator('.exercise-chevron').first();
    const body = page.locator('.exercise-body').first();

    await chevron.click();
    await expect(body).toHaveClass(/open/);

    await chevron.click();
    await page.waitForFunction(() => {
      const b = document.querySelector('.exercise-body');
      return b && !b.classList.contains('open');
    });
    const hasOpen = await body.evaluate((el) => el.classList.contains('open'));
    expect(hasOpen).toBe(false);
  });

  test('only one exercise is open at a time', async ({ page }) => {
    const chevrons = page.locator('.exercise-chevron');
    await chevrons.nth(0).click();
    await chevrons.nth(1).click();

    const bodies = page.locator('.exercise-body');
    const openCount = await bodies.evaluateAll((els) =>
      els.filter((el) => el.classList.contains('open')).length,
    );
    expect(openCount).toBe(1);
  });

  test('opening a card shows editable set rows', async ({ page }) => {
    await page.locator('.exercise-chevron').first().click();
    const body = page.locator('.exercise-body').first();
    await expect(body).toHaveClass(/open/);
    await expect(body.locator('.set-row').first()).toBeVisible();
    await expect(body.locator('.set-done-btn').first()).toBeVisible();
    await expect(body.locator('.set-num-input').first()).toBeVisible();
  });

  test('freeform set logged from exercises tab appears in history', async ({
    page,
  }) => {
    // Open the first exercise card and log a set
    const firstCard = page.locator('.ex-card').first();
    const body = firstCard.locator('.exercise-body');
    await firstCard.locator('.exercise-chevron').click();
    await expect(body).toHaveClass(/open/);

    // Fill weight and mark the first set done
    await body.locator('.set-num-input').first().fill('100');
    await body.locator('.set-num-input').first().press('Tab');
    await body.locator('.set-done-btn').first().evaluate((el: HTMLElement) => el.click());

    // History tab should now contain a Freeform entry
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await expect(page.locator('.history-item').first()).toContainText('Freeform');
  });

  test.describe('exercise-level complete button', () => {
    test('button is always present on each exercise card', async ({ page }) => {
      await expect(page.locator('.ex-card').first().locator('.ex-complete-btn')).toBeVisible();
    });

    test('clicking button immediately shows pending state', async ({ page }) => {
      const btn = page.locator('.ex-card').first().locator('.ex-complete-btn');
      await btn.click();
      await expect(btn).toHaveClass(/ex-complete-pending/);
    });

    test('accordion closes and pending clears after ~2s', async ({ page }) => {
      const firstCard = page.locator('.ex-card').first();
      const body = firstCard.locator('.exercise-body');
      await firstCard.locator('.exercise-chevron').click();
      await expect(body).toHaveClass(/open/);
      await clickCompleteBtn(page);
      // Pending immediately, accordion still open
      await expect(firstCard.locator('.ex-complete-btn')).toHaveClass(/ex-complete-pending/);
      await expect(body).toHaveClass(/open/);
      // After 2.5s: closed and no longer pending
      await page.waitForTimeout(2500);
      await expect(body).not.toHaveClass(/open/);
      await expect(firstCard.locator('.ex-complete-btn')).not.toHaveClass(/ex-complete-pending/);
    });

    test('no sets checked: no history entry created after reset', async ({ page }) => {
      const firstCard = page.locator('.ex-card').first();
      await firstCard.locator('.exercise-chevron').click();
      await clickCompleteBtn(page);
      await page.waitForTimeout(2500);
      await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
      await expect(page.locator('.history-item')).toHaveCount(0);
    });

    test('1 set checked: that set saved to history then accordion resets', async ({ page }) => {
      const firstCard = page.locator('.ex-card').first();
      const body = firstCard.locator('.exercise-body');
      await firstCard.locator('.exercise-chevron').click();
      await expect(body).toHaveClass(/open/);
      await body.locator('.set-num-input').nth(0).fill('100');
      await body.locator('.set-num-input').nth(0).press('Tab');
      await body.locator('.set-done-btn').nth(0).evaluate((el: HTMLElement) => el.click());
      await clickCompleteBtn(page);
      await page.waitForTimeout(2500);
      await expect(body).not.toHaveClass(/open/);
      await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
      await page.waitForSelector('.history-item');
      await expect(page.locator('.history-item').first()).toContainText('Freeform');
    });

    test('2 sets checked: both sets saved to history then accordion resets', async ({ page }) => {
      const firstCard = page.locator('.ex-card').first();
      const body = firstCard.locator('.exercise-body');
      await firstCard.locator('.exercise-chevron').click();
      await expect(body).toHaveClass(/open/);
      // Set 1: weight + reps + done
      await body.locator('.set-num-input').nth(0).fill('100');
      await body.locator('.set-num-input').nth(0).press('Tab');
      await body.locator('.set-num-input').nth(1).fill('8');
      await body.locator('.set-num-input').nth(1).press('Tab');
      await body.locator('.set-done-btn').nth(0).evaluate((el: HTMLElement) => el.click());
      // Set 2: weight + reps + done
      await body.locator('.set-num-input').nth(2).fill('105');
      await body.locator('.set-num-input').nth(2).press('Tab');
      await body.locator('.set-num-input').nth(3).fill('7');
      await body.locator('.set-num-input').nth(3).press('Tab');
      await body.locator('.set-done-btn').nth(1).evaluate((el: HTMLElement) => el.click());
      await clickCompleteBtn(page);
      await page.waitForTimeout(2500);
      await expect(body).not.toHaveClass(/open/);
      // Navigate to history detail and verify exactly 2 completed sets
      await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
      await page.waitForSelector('.history-item');
      await page.locator('.history-item').first().click();
      await expect(page.locator('.progress-table')).toBeVisible();
      await expect(
        page.locator('.progress-table tbody tr').filter({ hasText: '✓' }),
      ).toHaveCount(2);
    });

    test('after reset, last logged weight and reps pre-fill new sets', async ({ page }) => {
      const firstCard = page.locator('.ex-card').first();
      const body = firstCard.locator('.exercise-body');
      await firstCard.locator('.exercise-chevron').click();
      await expect(body).toHaveClass(/open/);
      await body.locator('.set-num-input').nth(0).fill('135');
      await body.locator('.set-num-input').nth(0).press('Tab');
      await body.locator('.set-num-input').nth(1).fill('10');
      await body.locator('.set-num-input').nth(1).press('Tab');
      await body.locator('.set-done-btn').nth(0).evaluate((el: HTMLElement) => el.click());
      await clickCompleteBtn(page);
      await page.waitForTimeout(2500);
      // Reopen accordion and verify pre-fill
      await firstCard.locator('.exercise-chevron').click();
      await expect(body).toHaveClass(/open/);
      await expect(body.locator('.set-num-input').nth(0)).toHaveValue('135');
      await expect(body.locator('.set-num-input').nth(1)).toHaveValue('10');
    });

    test('individual set checkoff auto-saves without the exercise button', async ({ page }) => {
      const firstCard = page.locator('.ex-card').first();
      const body = firstCard.locator('.exercise-body');
      await firstCard.locator('.exercise-chevron').click();
      await expect(body).toHaveClass(/open/);
      await body.locator('.set-num-input').nth(0).fill('80');
      await body.locator('.set-num-input').nth(0).press('Tab');
      await body.locator('.set-done-btn').nth(0).evaluate((el: HTMLElement) => el.click());
      // Navigate to History WITHOUT clicking the exercise complete button
      await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
      await page.waitForSelector('.history-item');
      await expect(page.locator('.history-item').first()).toContainText('Freeform');
    });
  });
});
