import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

async function goToExercises(page: Parameters<typeof freshPage>[0]) {
  await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
  await page.waitForSelector('.new-exercise-btn');
}

test.describe('New exercise creation', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
    await goToExercises(page);
  });

  // 1
  test('"+ New Exercise" button is visible', async ({ page }) => {
    await expect(page.locator('.new-exercise-btn')).toBeVisible();
  });

  // 2
  test('clicking the button reveals the new-exercise form', async ({ page }) => {
    await expect(page.locator('.new-exercise-form')).not.toBeVisible();
    await page.locator('.new-exercise-btn').click();
    await expect(page.locator('.new-exercise-form')).toBeVisible();
  });

  // 3
  test('Add button is disabled when the name field is empty', async ({ page }) => {
    await page.locator('.new-exercise-btn').click();
    const addBtn = page.locator('.new-exercise-form button').filter({ hasText: 'Add' });
    await expect(addBtn).toBeDisabled();
  });

  // 4
  test('Add button enables once a name is typed', async ({ page }) => {
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    const addBtn = page.locator('.new-exercise-form button').filter({ hasText: 'Add' });
    await expect(addBtn).toBeEnabled();
  });

  // 5
  test('saving adds a new exercise card to the list', async ({ page }) => {
    const before = await page.locator('.ex-card').count();
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();
    await expect(page.locator('.ex-card')).toHaveCount(before + 1);
    await expect(page.locator('.ex-card').filter({ hasText: 'Cable Fly' })).toBeVisible();
  });

  // 6
  test('form is hidden after a successful save', async ({ page }) => {
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();
    await expect(page.locator('.new-exercise-form')).not.toBeVisible();
  });

  // 7
  test('Cancel button hides the form without adding an exercise', async ({ page }) => {
    const before = await page.locator('.ex-card').count();
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Cancel' }).click();
    await expect(page.locator('.new-exercise-form')).not.toBeVisible();
    await expect(page.locator('.ex-card')).toHaveCount(before);
  });

  // 8
  test('new exercise persists after page reload', async ({ page }) => {
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();
    await expect(page.locator('.ex-card').filter({ hasText: 'Cable Fly' })).toBeVisible();

    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await goToExercises(page);
    await expect(page.locator('.ex-card').filter({ hasText: 'Cable Fly' })).toBeVisible();
  });

  // 9
  test('new exercise card can be expanded and sets logged', async ({ page }) => {
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();

    const card = page.locator('.ex-card').filter({ hasText: 'Cable Fly' });
    await card.locator('.exercise-chevron').click();
    await expect(card.locator('.exercise-body')).toHaveClass(/open/);
    await expect(card.locator('.set-row').first()).toBeVisible();
  });

  // 10
  test('custom reps min/max appear in the exercise meta', async ({ page }) => {
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-form input[type="text"]').fill('Cable Fly');
    // Change reps min to 6, reps max to 10
    const inputs = page.locator('.new-exercise-form input[type="number"]');
    await inputs.nth(1).fill('6');   // reps_min
    await inputs.nth(2).fill('10');  // reps_max
    await page.locator('.new-exercise-form button').filter({ hasText: 'Add' }).click();

    const card = page.locator('.ex-card').filter({ hasText: 'Cable Fly' });
    await expect(card.locator('.exercise-meta')).toContainText('6–10');
  });
});
