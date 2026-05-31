import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

async function goToExercises(page: Parameters<typeof freshPage>[0]) {
  await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
  await page.waitForSelector('.new-exercise-btn');
}

async function openPicker(page: Parameters<typeof freshPage>[0]) {
  await page.locator('.new-exercise-btn').click();
  await page.waitForSelector('.new-exercise-search');
}

test.describe('New exercise picker (library + custom fallback)', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
    await goToExercises(page);
  });

  // 1
  test('"+ New Exercise" button is visible', async ({ page }) => {
    await expect(page.locator('.new-exercise-btn')).toBeVisible();
  });

  // 2
  test('clicking the button reveals the picker', async ({ page }) => {
    await expect(page.locator('.new-exercise-form')).not.toBeVisible();
    await openPicker(page);
    await expect(page.locator('.new-exercise-form')).toBeVisible();
    await expect(page.locator('.new-exercise-search')).toBeVisible();
  });

  // 3
  test('library results render when the picker opens', async ({ page }) => {
    await openPicker(page);
    // Library is fetched asynchronously; give it a moment.
    await expect(page.locator('.new-exercise-result').first()).toBeVisible({ timeout: 10_000 });
    const count = await page.locator('.new-exercise-result').count();
    expect(count).toBeGreaterThan(1);
  });

  // 4
  test('typing in the search box filters the results', async ({ page }) => {
    await openPicker(page);
    await page.waitForSelector('.new-exercise-result');
    await page.locator('.new-exercise-search').fill('treadmill');
    // All visible results should contain "treadmill" (case-insensitive)
    const names = await page.locator('.new-exercise-result .library-item-name').allTextContents();
    expect(names.length).toBeGreaterThan(0);
    for (const n of names) {
      expect(n.toLowerCase()).toContain('treadmill');
    }
  });

  // 5
  test('tapping a library result adds it as a card and closes the picker', async ({ page }) => {
    const before = await page.locator('.ex-card').count();
    await openPicker(page);
    await page.waitForSelector('.new-exercise-result');
    await page.locator('.new-exercise-search').fill('treadmill');
    const firstName = await page
      .locator('.new-exercise-result .library-item-name')
      .first()
      .textContent();
    await page.locator('.new-exercise-result').first().click();

    await expect(page.locator('.new-exercise-form')).not.toBeVisible();
    await expect(page.locator('.ex-card')).toHaveCount(before + 1);
    await expect(
      page.locator('.ex-card').filter({ hasText: firstName!.trim() }),
    ).toBeVisible();
  });

  // 6
  test('newly-added exercise opens its accordion automatically', async ({ page }) => {
    await openPicker(page);
    await page.waitForSelector('.new-exercise-result');
    await page.locator('.new-exercise-search').fill('treadmill');
    const firstName = (
      await page.locator('.new-exercise-result .library-item-name').first().textContent()
    )!.trim();
    await page.locator('.new-exercise-result').first().click();

    const card = page.locator('.ex-card').filter({ hasText: firstName });
    await expect(card.locator('.exercise-body')).toHaveClass(/open/);
    await expect(card.locator('.set-row').first()).toBeVisible();
  });

  // 7
  test('Cancel hides the picker without adding anything', async ({ page }) => {
    const before = await page.locator('.ex-card').count();
    await openPicker(page);
    await page.locator('.new-exercise-search').fill('treadmill');
    await page.locator('.new-exercise-form button').filter({ hasText: 'Cancel' }).click();
    await expect(page.locator('.new-exercise-form')).not.toBeVisible();
    await expect(page.locator('.ex-card')).toHaveCount(before);
  });

  // 8
  test('"+ Use ... as custom" appears for queries with no exact library match', async ({ page }) => {
    await openPicker(page);
    await page.waitForSelector('.new-exercise-result');
    await page.locator('.new-exercise-search').fill('My Special Lift xyzzy');
    await expect(page.locator('.new-exercise-custom')).toBeVisible();
    await expect(page.locator('.new-exercise-custom')).toContainText('My Special Lift xyzzy');
  });

  // 9
  test('tapping the custom fallback creates a freeform exercise card', async ({ page }) => {
    const before = await page.locator('.ex-card').count();
    await openPicker(page);
    await page.waitForSelector('.new-exercise-result');
    await page.locator('.new-exercise-search').fill('Cable Fly');
    await page.locator('.new-exercise-custom').click();
    await expect(page.locator('.ex-card')).toHaveCount(before + 1);
    await expect(page.locator('.ex-card').filter({ hasText: 'Cable Fly' })).toBeVisible();
  });

  // 11 — regression guard: button must sit ABOVE the exercise cards so users
  // don't have to scroll past a long list to add a new one.
  test('"+ New Exercise" button sits above the exercise cards (DOM and visual order)', async ({ page }) => {
    // Seed enough history to push the bottom of the list well off-screen.
    await page.evaluate(() => {
      const today = new Date().toISOString().slice(0, 10);
      const mk = (name: string, i: number) => ({
        id: `seed-${i}`,
        date: today,
        created_at: `${today}T10:0${i}:00.000Z`,
        exercise_name: name,
        exercise_id: `seed-${i}`,
        session_id: null,
        day_id: null,
        day_name: null,
        target_sets: 3,
        reps_min: 8,
        reps_max: 12,
        sets: [
          { set_number: 1, reps: 10, weight: 100, completed: true, completed_date: today },
        ],
        finalized: true,
      });
      const names = ['Bench Press', 'Squat', 'Deadlift', 'Row', 'OHP', 'Pull-up', 'Curl', 'Dip'];
      localStorage.setItem('ws_ex_history', JSON.stringify(names.map(mk)));
    });
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await goToExercises(page);

    // At least one exercise card should be on screen now.
    await expect(page.locator('.ex-card').first()).toBeVisible();

    // 1) DOM order — the button comes before the first .ex-card.
    const buttonComesFirst = await page.evaluate(() => {
      const btn = document.querySelector('.new-exercise-btn');
      const card = document.querySelector('.ex-card');
      if (!btn || !card) return null;
      // DOCUMENT_POSITION_FOLLOWING (4) is set when `card` follows `btn`.
      const pos = btn.compareDocumentPosition(card);
      return (pos & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    });
    expect(buttonComesFirst).toBe(true);

    // 2) Visual order — button's top edge is above the first card's top edge.
    const btnBox = await page.locator('.new-exercise-btn').boundingBox();
    const cardBox = await page.locator('.ex-card').first().boundingBox();
    expect(btnBox).not.toBeNull();
    expect(cardBox).not.toBeNull();
    expect(btnBox!.y).toBeLessThan(cardBox!.y);
  });

  // 10
  test('added exercise persists after page reload', async ({ page }) => {
    await openPicker(page);
    await page.waitForSelector('.new-exercise-result');
    await page.locator('.new-exercise-search').fill('treadmill');
    const firstName = (
      await page.locator('.new-exercise-result .library-item-name').first().textContent()
    )!.trim();
    await page.locator('.new-exercise-result').first().click();
    await expect(page.locator('.ex-card').filter({ hasText: firstName })).toBeVisible();

    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await goToExercises(page);
    await expect(page.locator('.ex-card').filter({ hasText: firstName })).toBeVisible();
  });
});
