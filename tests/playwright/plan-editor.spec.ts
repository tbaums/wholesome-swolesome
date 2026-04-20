import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

test.describe('Plan editor', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'Plan' }).click();
    await page.waitForSelector('.card.day-item');
  });

  test('plan editor shows all 7 training days', async ({ page }) => {
    const dayItems = page.locator('.card.day-item');
    await expect(dayItems).toHaveCount(7);
    await expect(dayItems.first()).toContainText('Lower A: Glute Tension');
  });

  test('tapping a day navigates to the day editor', async ({ page }) => {
    await page.locator('.card.day-item').first().click();
    await expect(page.locator('.back-btn')).toBeVisible();
    // Day editor shows exercise cards (each has an .exercise-header inside a .card)
    await expect(page.locator('.exercise-header').first()).toBeVisible();
  });

  test('day name can be edited and persists', async ({ page }) => {
    await page.locator('.card.day-item').first().click();
    const nameInput = page.locator('input[type="text"]').first();
    await nameInput.fill('My Custom Day');
    await nameInput.press('Tab'); // triggers on:blur → save

    await page.locator('.back-btn').click();
    await expect(page.locator('.card.day-item').first()).toContainText(
      'My Custom Day',
    );
  });

  test('add exercise appends a new exercise to the day', async ({ page }) => {
    await page.locator('.card.day-item').first().click();
    const before = await page.locator('.exercise-header').count();
    await page.getByText('+ Add Exercise').click();
    const after = await page.locator('.exercise-header').count();
    expect(after).toBe(before + 1);
  });
});
