import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8081';

function scheduledWorkout(id: string, date: string, name: string) {
  return {
    id,
    date,
    name,
    rationale: '',
    source: 'Coach',
    exercises: [
      {
        library_id: 'Barbell_Bench_Press_-_Medium_Grip',
        name: 'Barbell Bench Press - Medium Grip',
        target_sets: 3,
        reps_min: 8,
        reps_max: 12,
        rest_seconds: 120,
        notes: null,
      },
    ],
    created_at: `${date}T08:00:00.000Z`,
  };
}

function historyEntryFor(dayId: string, date: string, completed: boolean) {
  return {
    id: `e-${dayId}`,
    date,
    created_at: `${date}T18:00:00.000Z`,
    exercise_name: 'Barbell Bench Press - Medium Grip',
    exercise_id: 'Barbell_Bench_Press_-_Medium_Grip',
    session_id: 'sess-test',
    day_id: dayId,
    day_name: 'Push',
    target_sets: 3,
    reps_min: 8,
    reps_max: 12,
    sets: [
      {
        set_number: 1,
        reps: 10,
        weight: 135,
        completed,
        completed_date: completed ? date : null,
      },
    ],
    finalized: true,
  };
}

async function seedAndOpen(
  page: import('@playwright/test').Page,
  scheduled: ReturnType<typeof scheduledWorkout>[],
  history: ReturnType<typeof historyEntryFor>[],
) {
  await enableDateMock(page, MOCK_NOW);
  await freshPage(page);
  await page.evaluate(
    ([s, h]) => {
      localStorage.setItem('ws_scheduled_workouts', s as string);
      localStorage.setItem('ws_ex_history', h as string);
    },
    [JSON.stringify(scheduled), JSON.stringify(history)] as [string, string],
  );
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

test.describe('Home: TODAY card after completion', () => {
  test('shows the scheduled workout when no matching history exists yet', async ({ page }) => {
    const w = scheduledWorkout('w-today', TODAY, 'Push Day');
    await seedAndOpen(page, [w], []);

    await expect(page.locator('.today-card')).toContainText('TODAY');
    await expect(page.locator('.today-card')).toContainText('Push Day');
    await expect(page.locator('button').filter({ hasText: /Start workout|Resume workout/ })).toBeVisible();
  });

  test('hides the start button and shows DONE badge once history has a completed set for that workout', async ({ page }) => {
    const w = scheduledWorkout('w-today', TODAY, 'Push Day');
    const completed = historyEntryFor('w-today', TODAY, true);
    await seedAndOpen(page, [w], [completed]);

    // The DONE card replaces the actionable TODAY card.
    await expect(page.locator('.today-card .today-badge')).toHaveText('DONE');
    await expect(page.locator('.today-card')).toContainText('Push Day');
    await expect(page.locator('.today-card')).toContainText("Today's workout is logged");

    // Crucially, no Start/Resume button is visible inside the today-card.
    const startBtn = page.locator('.today-card button').filter({ hasText: /Start workout|Resume workout/ });
    await expect(startBtn).toHaveCount(0);
  });

  test('still shows the workout if the history entry has only incomplete sets', async ({ page }) => {
    const w = scheduledWorkout('w-today', TODAY, 'Push Day');
    const draft = historyEntryFor('w-today', TODAY, false);
    await seedAndOpen(page, [w], [draft]);

    await expect(page.locator('.today-card .today-badge')).toHaveText('TODAY');
    await expect(page.locator('button').filter({ hasText: /Start workout|Resume workout/ })).toBeVisible();
  });

  test('a completed history entry from a different day_id does not hide today', async ({ page }) => {
    const w = scheduledWorkout('w-today', TODAY, 'Push Day');
    const unrelated = historyEntryFor('w-other', TODAY, true);
    await seedAndOpen(page, [w], [unrelated]);

    await expect(page.locator('.today-card .today-badge')).toHaveText('TODAY');
    await expect(page.locator('button').filter({ hasText: /Start workout|Resume workout/ })).toBeVisible();
  });
});
