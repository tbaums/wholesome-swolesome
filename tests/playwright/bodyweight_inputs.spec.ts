import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr, openExercise } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8081';

// A scheduled workout mixing one body-only exercise (Pullups, id: "Pullups",
// equipment: "body only" in public/data/exercises.json) and one barbell
// exercise (Bench Press) so a single spec exercises both render paths.
function mixedWorkout(date: string) {
  return {
    id: 'w-mix',
    date,
    name: 'Bodyweight + Bench',
    rationale: '',
    source: 'Coach',
    exercises: [
      {
        library_id: 'Pullups',
        name: 'Pullups',
        target_sets: 3,
        reps_min: 5,
        reps_max: 10,
        rest_seconds: 90,
        notes: null,
      },
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

async function startMixedSession(page: import('@playwright/test').Page) {
  await enableDateMock(page, MOCK_NOW);
  await freshPage(page);
  await page.evaluate(
    (val) => localStorage.setItem('ws_scheduled_workouts', val),
    JSON.stringify([mixedWorkout(TODAY)]),
  );
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.locator('button').filter({ hasText: 'Start workout' }).click();
  await page.waitForSelector('.ex-card');
}

test.describe('Bodyweight exercises hide the weight input', () => {
  test('session view: body-only exercise renders ONLY a reps input per set; barbell still renders two', async ({ page }) => {
    await startMixedSession(page);

    // Bodyweight first, then barbell — matching the seeded order.
    const pullups = page.locator('.ex-card').filter({ hasText: 'Pullups' });
    const bench = page.locator('.ex-card').filter({ hasText: 'Bench Press' });

    await openExercise(page, 0); // Pullups
    const pullupRows = pullups.locator('.set-row');
    await expect(pullupRows).toHaveCount(3);

    // Each Pullups set row has exactly ONE input (reps) and no "×" separator.
    for (let i = 0; i < 3; i++) {
      const row = pullupRows.nth(i);
      await expect(row.locator('.set-num-input')).toHaveCount(1);
      await expect(row.locator('.set-x')).toHaveCount(0);
      // The single input is the reps input (numeric, placeholder "reps").
      const placeholder = await row.locator('.set-num-input').getAttribute('placeholder');
      expect(placeholder).toBe('reps');
    }

    // Now open Bench Press: still two inputs (weight × reps).
    await openExercise(page, 1);
    const benchRows = bench.locator('.set-row');
    for (let i = 0; i < 3; i++) {
      const row = benchRows.nth(i);
      await expect(row.locator('.set-num-input')).toHaveCount(2);
      await expect(row.locator('.set-x')).toHaveCount(1);
      // Order: weight input first, then reps.
      await expect(row.locator('.set-num-input').first()).toHaveAttribute('placeholder', 'wt');
      await expect(row.locator('.set-num-input').last()).toHaveAttribute('placeholder', 'reps');
    }
  });

  test('logging a bodyweight set persists reps and keeps weight at 0 in localStorage', async ({ page }) => {
    await startMixedSession(page);
    await openExercise(page, 0); // Pullups

    const firstRow = page.locator('.ex-card').filter({ hasText: 'Pullups' }).locator('.set-row').first();
    const repsInput = firstRow.locator('.set-num-input'); // single input
    await repsInput.fill('8');
    await repsInput.press('Tab');
    await firstRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    // active_session in localStorage should show reps=8 and weight=0 (default).
    const session = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_active_session') ?? 'null'),
    );
    expect(session).not.toBeNull();
    const pullupLog = session.exercise_logs.find((e: { exercise_name: string }) => e.exercise_name === 'Pullups');
    expect(pullupLog).toBeTruthy();
    expect(pullupLog.sets[0].reps).toBe(8);
    expect(pullupLog.sets[0].weight).toBe(0);
    expect(pullupLog.sets[0].completed).toBe(true);
  });

  test('freeform tab: adding a body-only library entry renders only a reps input', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.new-exercise-btn');

    await page.locator('.new-exercise-btn').click();
    await page.waitForSelector('.new-exercise-search');
    // Type "pullups" to find the body-only Pullups library entry.
    await page.locator('.new-exercise-search').fill('pullups');
    await page.waitForSelector('.new-exercise-result');
    // The .new-exercise-result text content includes the meta line (muscle · equipment),
    // so filter by the inner .library-item-name with an exact text match to pick the
    // body-only "Pullups" entry (not "V-Bar Pullup").
    await page
      .locator('.new-exercise-result')
      .filter({ has: page.getByText('Pullups', { exact: true }) })
      .first()
      .click();

    // Card is open by default after add; check its set rows.
    const card = page.locator('.ex-card').filter({ hasText: 'Pullups' });
    await expect(card.locator('.exercise-body')).toHaveClass(/open/);

    const rows = card.locator('.set-row');
    await expect(rows.first()).toBeVisible();
    const firstRow = rows.first();
    await expect(firstRow.locator('.set-num-input')).toHaveCount(1);
    await expect(firstRow.locator('.set-x')).toHaveCount(0);
    await expect(firstRow.locator('.set-num-input')).toHaveAttribute('placeholder', 'reps');
  });

  test('back-compat: an existing history entry with weight=0 round-trips through localStorage unchanged', async ({ page }) => {
    // Data shape that pre-existed before this change must still load and
    // be re-serializable without losing/altering the weight field.
    const oldEntry = {
      id: 'old-1',
      date: TODAY,
      created_at: `${TODAY}T10:00:00.000Z`,
      exercise_name: 'Pullups',
      exercise_id: 'Pullups',
      session_id: null,
      day_id: null,
      day_name: null,
      target_sets: 3,
      reps_min: 5,
      reps_max: 10,
      sets: [
        { set_number: 1, reps: 8, weight: 0, completed: true, completed_date: TODAY },
        { set_number: 2, reps: 7, weight: 0, completed: true, completed_date: TODAY },
      ],
      finalized: true,
    };
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([oldEntry]),
    );
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    // The app must boot without crashing and the entry must still be there.
    const after = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(after).toHaveLength(1);
    expect(after[0].id).toBe('old-1');
    expect(after[0].sets[0].weight).toBe(0);
    expect(after[0].sets[0].reps).toBe(8);
    expect(after[0].sets[1].weight).toBe(0);
    expect(after[0].sets[1].reps).toBe(7);
  });
});
