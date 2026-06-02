import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr } from './helpers';

// Mock a fixed "today" so the backdate UI's max=today constraint is deterministic
// and the test isn't subject to timezone drift in CI.
const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const YESTERDAY = '2026-06-09';
const BASE = 'http://localhost:8081';

// The date input lives in the "Logging for" card at the top of the Exercises tab.
const DATE_PICKER = 'input[type="date"]';

async function goToExercises(page: import('@playwright/test').Page) {
  await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
  await page.waitForSelector('.new-exercise-btn');
}

async function setLoggingDate(page: import('@playwright/test').Page, value: string) {
  // Fill the date input and dispatch a change event the way Leptos's on:change wants.
  await page.locator(DATE_PICKER).fill(value);
  await page.locator(DATE_PICKER).dispatchEvent('change');
}

async function addCustomExercise(page: import('@playwright/test').Page, name: string) {
  await page.locator('.new-exercise-btn').click();
  await page.waitForSelector('.new-exercise-search');
  await page.locator('.new-exercise-search').fill(name);
  await page.locator('.new-exercise-custom').click();
}

test.describe('Freeform date picker — backdate freeform logging', () => {
  test.beforeEach(async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    await goToExercises(page);
  });

  test('date picker defaults to today and shows no "Backdated" badge', async ({ page }) => {
    await expect(page.locator(DATE_PICKER)).toHaveValue(TODAY);
    // No accent-colored "Backdated" indicator when the selected date is today.
    await expect(page.locator('text=Backdated')).toHaveCount(0);
  });

  test('date picker rejects future dates via its `max` attribute', async ({ page }) => {
    await expect(page.locator(DATE_PICKER)).toHaveAttribute('max', TODAY);
  });

  test('picking yesterday surfaces a "Backdated" indicator', async ({ page }) => {
    await setLoggingDate(page, YESTERDAY);
    await expect(page.locator(DATE_PICKER)).toHaveValue(YESTERDAY);
    await expect(page.locator('text=Backdated')).toBeVisible();
  });

  test('logging a freeform set with a backdated date persists the picked date in history', async ({ page }) => {
    // Pick yesterday.
    await setLoggingDate(page, YESTERDAY);
    await addCustomExercise(page, 'My Backdated Lift');

    const card = page.locator('.ex-card').filter({ hasText: 'My Backdated Lift' });
    await expect(card).toBeVisible();
    await expect(card.locator('.exercise-body')).toHaveClass(/open/);

    // Fill the first set: 100 × 5 (weight × reps).
    const firstRow = card.locator('.set-row').first();
    const inputs = firstRow.locator('.set-num-input');
    await inputs.nth(0).fill('100');
    await inputs.nth(0).press('Tab');
    await inputs.nth(1).fill('5');
    await inputs.nth(1).press('Tab');
    await firstRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    // Pre-finalize state (still a draft): one history entry dated YESTERDAY.
    const draft = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(draft).toHaveLength(1);
    expect(draft[0].date).toBe(YESTERDAY);
    expect(draft[0].finalized).toBe(false);
    expect(draft[0].sets[0].weight).toBe(100);
    expect(draft[0].sets[0].reps).toBe(5);
    expect(draft[0].sets[0].completed).toBe(true);
    expect(draft[0].sets[0].completed_date).toBe(YESTERDAY);

    // Finalize. The ✓ uses a 2-second debounce — wait for it.
    await card.locator('.ex-complete-btn').click();
    await page.waitForTimeout(2500);

    const finalized = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(finalized).toHaveLength(1);
    expect(finalized[0].date).toBe(YESTERDAY);
    expect(finalized[0].finalized).toBe(true);
    expect(finalized[0].sets[0].completed_date).toBe(YESTERDAY);
  });

  test('default-today flow still writes today\'s date (no regression)', async ({ page }) => {
    // Don't touch the date picker — it should already be today.
    await addCustomExercise(page, 'My Today Lift');

    const card = page.locator('.ex-card').filter({ hasText: 'My Today Lift' });
    const firstRow = card.locator('.set-row').first();
    const inputs = firstRow.locator('.set-num-input');
    await inputs.nth(0).fill('50');
    await inputs.nth(0).press('Tab');
    await inputs.nth(1).fill('8');
    await inputs.nth(1).press('Tab');
    await firstRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(1);
    expect(entries[0].date).toBe(TODAY);
    expect(entries[0].sets[0].completed_date).toBe(TODAY);
  });

  test('changing date mid-session creates a separate draft per date (no cross-contamination)', async ({ page }) => {
    // Log a today-set first.
    await addCustomExercise(page, 'Cross-Date Lift');
    const card = page.locator('.ex-card').filter({ hasText: 'Cross-Date Lift' });
    const todayRow = card.locator('.set-row').first();
    await todayRow.locator('.set-num-input').nth(0).fill('100');
    await todayRow.locator('.set-num-input').nth(0).press('Tab');
    await todayRow.locator('.set-num-input').nth(1).fill('10');
    await todayRow.locator('.set-num-input').nth(1).press('Tab');
    await todayRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    // Now switch to yesterday and log a different set on the same card.
    await setLoggingDate(page, YESTERDAY);
    const yesterdayRow = card.locator('.set-row').first();
    // The card re-renders showing a fresh draft for yesterday — inputs may
    // be pre-filled from last completed (today's 100 × 10), but the entry
    // we're editing is dated YESTERDAY, not today.
    await yesterdayRow.locator('.set-num-input').nth(0).fill('80');
    await yesterdayRow.locator('.set-num-input').nth(0).press('Tab');
    await yesterdayRow.locator('.set-num-input').nth(1).fill('12');
    await yesterdayRow.locator('.set-num-input').nth(1).press('Tab');
    await yesterdayRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    // History should now have two distinct (date, draft) entries — one per date.
    const entries: Array<{ date: string; sets: Array<{ weight: number; reps: number }> }> =
      await page.evaluate(() => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'));
    expect(entries).toHaveLength(2);
    const byDate = Object.fromEntries(entries.map((e) => [e.date, e]));
    expect(byDate[TODAY]).toBeTruthy();
    expect(byDate[YESTERDAY]).toBeTruthy();
    expect(byDate[TODAY].sets[0].weight).toBe(100);
    expect(byDate[TODAY].sets[0].reps).toBe(10);
    expect(byDate[YESTERDAY].sets[0].weight).toBe(80);
    expect(byDate[YESTERDAY].sets[0].reps).toBe(12);
  });
});
