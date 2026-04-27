import { test, expect } from '@playwright/test';
import { freshPage, startWorkout, completeAllSets, openExercise, enableDateMock, setMockDate, localDateStr } from './helpers';

// Minimal ExerciseEntry fixture for localStorage injection.
function makeEntry(id: string, exerciseName: string, date: string, createdAt: string) {
  return {
    id,
    date,
    created_at: createdAt,
    exercise_name: exerciseName,
    exercise_id: `ex-${id}`,
    session_id: null,
    day_id: null,
    day_name: null,
    target_sets: 3,
    reps_min: 8,
    reps_max: 12,
    sets: [{ set_number: 1, reps: 10, weight_lbs: 100.0, completed: true, completed_date: date }],
    finalized: true,
  };
}

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

  test('clicking an entry opens its detail view', async ({ page }) => {
    await finishWorkout(page);
    await page.locator('.history-item').first().click();
    await expect(page.locator('.back-btn')).toBeVisible();
    await expect(page.locator('.progress-table')).toBeVisible();
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

test.describe('History - timestamps and ordering', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('entries are sorted oldest-first by created_at timestamp', async ({ page }) => {
    // Inject in REVERSE order (later first) to prove the app sorts, not just preserves insertion order
    const later  = makeEntry('b', 'Squat',       '2026-04-27', '2026-04-27T14:00:00.000Z');
    const earlier = makeEntry('a', 'Bench Press', '2026-04-27', '2026-04-27T10:00:00.000Z');
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([later, earlier]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    const items = page.locator('.history-item');
    await expect(items.nth(0)).toContainText('Bench Press'); // earlier timestamp first
    await expect(items.nth(1)).toContainText('Squat');       // later timestamp second
  });

  test('history list is scrolled to the bottom on load', async ({ page }) => {
    // Inject 10 entries (hours 0–9 on the same day) to overflow the viewport
    const entries = Array.from({ length: 10 }, (_, i) =>
      makeEntry(
        `entry-${i}`,
        `Exercise ${i + 1}`,
        '2026-04-27',
        `2026-04-27T${String(i).padStart(2, '0')}:00:00.000Z`,
      ),
    );
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify(entries),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');
    await expect(page.locator('.history-item').last()).toBeInViewport();
    await expect(page.locator('.history-item').first()).not.toBeInViewport();
  });

  test('finished workout entries are dated today', async ({ page }) => {
    await startWorkout(page, 3);
    await completeAllSets(page);
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.history-item');
    const today = localDateStr(new Date().toISOString());
    const dates = page.locator('.history-item .history-date');
    const count = await dates.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i++) {
      await expect(dates.nth(i)).toHaveText(today);
    }
  });

  test.describe('with mocked date', () => {
    const ISO_APR_27 = '2026-04-27T12:00:00.000Z';
    const ISO_APR_28 = '2026-04-28T12:00:00.000Z';

    test.beforeEach(async ({ page }) => {
      await enableDateMock(page, ISO_APR_27);
      await freshPage(page); // reloads with mock active
    });

    test('sets completed on Apr 27, finish clicked Apr 28 → entries dated Apr 27', async ({ page }) => {
      const expectedDate = localDateStr(ISO_APR_27);
      await startWorkout(page, 3);
      await completeAllSets(page);
      await setMockDate(page, ISO_APR_28);
      await page.locator('.btn-finish').click();
      await page.waitForSelector('.history-item');
      const dates = page.locator('.history-item .history-date');
      const count = await dates.count();
      expect(count).toBeGreaterThan(0);
      for (let i = 0; i < count; i++) {
        await expect(dates.nth(i)).toHaveText(expectedDate);
      }
    });

    test('sets checked on different days produce separate history entries', async ({ page }) => {
      const date27 = localDateStr(ISO_APR_27);
      const date28 = localDateStr(ISO_APR_28);
      await startWorkout(page, 0); // Day 1
      // Check set 1 of exercise 1 on Apr 27
      await openExercise(page, 0);
      await page.locator('.exercise-body').first().locator('.set-done-btn').nth(0)
        .evaluate((el: HTMLElement) => el.click());
      // Advance to Apr 28 and check set 2
      await setMockDate(page, ISO_APR_28);
      await page.locator('.exercise-body').first().locator('.set-done-btn').nth(1)
        .evaluate((el: HTMLElement) => el.click());
      await page.locator('.btn-finish').click();
      await page.waitForSelector('.history-item');
      // Both dates must appear somewhere in the history list
      await expect(
        page.locator('.history-item').filter({ has: page.locator('.history-date', { hasText: date27 }) }).first(),
      ).toBeVisible();
      await expect(
        page.locator('.history-item').filter({ has: page.locator('.history-date', { hasText: date28 }) }).first(),
      ).toBeVisible();
    });
  });
});
