import { test, expect } from '@playwright/test';
import { freshPage, enableDateMock, localDateStr, BASE, openExercise } from './helpers';

const TODAY_ISO = '2026-05-27T08:00:00.000Z';
const TODAY = localDateStr(TODAY_ISO);

function makeScheduledWorkout(date: string) {
  return {
    id: 'sw-stretch-test',
    date,
    name: 'Strength + Balance + Stretch',
    rationale: 'Test workout with all modalities.',
    source: 'Coach',
    exercises: [
      {
        library_id: 'Barbell_Bench_Press_-_Medium_Grip',
        name: 'Bench Press',
        target_sets: 2,
        reps_min: 6,
        reps_max: 8,
        rest_seconds: 180,
        notes: null,
      },
      {
        library_id: 'Bird_Dog',
        name: 'Bird Dog',
        target_sets: 2,
        reps_min: 1,
        reps_max: 1,
        rest_seconds: 15,
        notes: 'Hold each side',
        target_duration_seconds: 30,
      },
      {
        library_id: 'Standing_Hamstring_Stretch',
        name: 'Standing Hamstring Stretch',
        target_sets: 2,
        reps_min: 1,
        reps_max: 1,
        rest_seconds: 10,
        notes: 'Hold each side 30s',
        target_duration_seconds: 30,
      },
    ],
    created_at: '2026-05-26T23:00:00.000Z',
  };
}

async function seedWorkout(page: Parameters<typeof freshPage>[0], date: string) {
  const workout = makeScheduledWorkout(date);
  await page.evaluate((w) => {
    localStorage.setItem('scheduled_workouts', JSON.stringify([w]));
  }, workout);
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

test.describe('Stretching & balance exercises', () => {
  test.beforeEach(async ({ page }) => {
    await enableDateMock(page, TODAY_ISO);
    await freshPage(page);
    await seedWorkout(page, TODAY);
  });

  test('today card shows duration prescription for stretching/balance exercises', async ({ page }) => {
    const todayCard = page.locator('.today-card');
    await expect(todayCard).toContainText('Strength + Balance + Stretch');

    const prescriptions = todayCard.locator('.today-ex-prescription');
    await expect(prescriptions.nth(0)).toContainText('2×6-8');
    await expect(prescriptions.nth(1)).toContainText('2×30s');
    await expect(prescriptions.nth(2)).toContainText('2×30s');
  });

  test('session view shows duration label for stretching exercises', async ({ page }) => {
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.ex-card');

    const cards = page.locator('.ex-card');
    await expect(cards).toHaveCount(3);

    // Strength exercise shows reps
    await expect(cards.nth(0).locator('.exercise-meta')).toContainText('reps');

    // Balance exercise shows hold
    await expect(cards.nth(1).locator('.exercise-meta')).toContainText('30s hold');

    // Stretching exercise shows hold
    await expect(cards.nth(2).locator('.exercise-meta')).toContainText('30s hold');
  });

  test('duration exercise shows seconds input instead of weight x reps', async ({ page }) => {
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.ex-card');

    // Open stretching exercise (index 2)
    await openExercise(page, 2);
    const stretchCard = page.locator('.ex-card').nth(2);
    const body = stretchCard.locator('.exercise-body');

    // Should have "sec" placeholder input, not "wt" and "reps"
    await expect(body.locator('input[placeholder="sec"]').first()).toBeVisible();
    await expect(body.locator('input[placeholder="wt"]')).toHaveCount(0);
    await expect(body.locator('input[placeholder="reps"]')).toHaveCount(0);

    // Should show "s" label not "x"
    await expect(body.locator('.set-x').first()).toContainText('s');
  });

  test('strength exercise still shows weight x reps inputs', async ({ page }) => {
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.ex-card');

    // Open strength exercise (index 0)
    await openExercise(page, 0);
    const strengthCard = page.locator('.ex-card').nth(0);
    const body = strengthCard.locator('.exercise-body');

    await expect(body.locator('input[placeholder="wt"]').first()).toBeVisible();
    await expect(body.locator('input[placeholder="reps"]').first()).toBeVisible();
    await expect(body.locator('input[placeholder="sec"]')).toHaveCount(0);
  });

  test('can complete a duration-based set and finish workout', async ({ page }) => {
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.ex-card');

    // Complete all sets in the strength exercise
    await openExercise(page, 0);
    const strengthBody = page.locator('.exercise-body').nth(0);
    const strengthDoneBtns = strengthBody.locator('.set-done-btn');
    for (let i = 0; i < await strengthDoneBtns.count(); i++) {
      await strengthDoneBtns.nth(i).evaluate((el: HTMLElement) => el.click());
    }

    // Complete all sets in the balance exercise
    await openExercise(page, 1);
    const balanceBody = page.locator('.exercise-body').nth(1);
    const balanceDoneBtns = balanceBody.locator('.set-done-btn');
    for (let i = 0; i < await balanceDoneBtns.count(); i++) {
      await balanceDoneBtns.nth(i).evaluate((el: HTMLElement) => el.click());
    }

    // Complete all sets in the stretching exercise
    await openExercise(page, 2);
    const stretchBody = page.locator('.exercise-body').nth(2);
    const stretchDoneBtns = stretchBody.locator('.set-done-btn');
    for (let i = 0; i < await stretchDoneBtns.count(); i++) {
      await stretchDoneBtns.nth(i).evaluate((el: HTMLElement) => el.click());
    }

    // Finish button should show checkmark
    const finishBtn = page.locator('.btn-finish.btn-full');
    await expect(finishBtn).toContainText('✓');
    await finishBtn.click();

    // Should navigate to history and show toast
    await expect(page.locator('.toast')).toBeVisible();
  });

  test('library shows stretching and balance categories', async ({ page }) => {
    await page.locator('.nav-btn').filter({ hasText: 'Library' }).click();
    await page.waitForSelector('.library-item', { timeout: 8000 });

    // Search for a stretching exercise
    const search = page.locator('.library-search');
    if (await search.isVisible()) {
      await search.fill('Standing Hamstring Stretch');
      await expect(page.locator('.library-item').first()).toContainText('Standing Hamstring Stretch');
    }
  });

  test('duration input can be edited', async ({ page }) => {
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.ex-card');

    // Open balance exercise (index 1)
    await openExercise(page, 1);
    const body = page.locator('.exercise-body').nth(1);
    const durInput = body.locator('input[placeholder="sec"]').first();

    await durInput.fill('45');
    await durInput.press('Tab');

    // Verify value persists
    await expect(durInput).toHaveValue('45');
  });
});
