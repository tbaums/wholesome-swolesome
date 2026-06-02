import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr, openExercise } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8081';

// A scheduled workout mixing one body-only exercise that's GENUINELY weightless
// (Plank — equipment: "body only", category: "strength", and not on the
// WEIGHTABLE_BODYWEIGHT_IDS allow-list) and one barbell exercise so a single
// spec exercises both render paths.
function mixedWorkout(date: string) {
  return {
    id: 'w-mix',
    date,
    name: 'Plank + Bench',
    rationale: '',
    source: 'Coach',
    exercises: [
      {
        library_id: 'Plank',
        name: 'Plank',
        target_sets: 3,
        reps_min: 30,
        reps_max: 60,
        rest_seconds: 60,
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
  test('session view: weightless body-only exercise renders ONLY a reps input; barbell still renders two', async ({ page }) => {
    await startMixedSession(page);

    const plank = page.locator('.ex-card').filter({ hasText: 'Plank' });
    const bench = page.locator('.ex-card').filter({ hasText: 'Bench Press' });

    await openExercise(page, 0); // Plank
    const plankRows = plank.locator('.set-row');
    await expect(plankRows).toHaveCount(3);

    // Each Plank set row has exactly ONE input (reps) and no "×" separator.
    for (let i = 0; i < 3; i++) {
      const row = plankRows.nth(i);
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

  test('logging a weightless set persists reps and keeps weight at 0 in localStorage', async ({ page }) => {
    await startMixedSession(page);
    await openExercise(page, 0); // Plank

    const firstRow = page.locator('.ex-card').filter({ hasText: 'Plank' }).locator('.set-row').first();
    const repsInput = firstRow.locator('.set-num-input'); // single input
    await repsInput.fill('45');
    await repsInput.press('Tab');
    await firstRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    // active_session in localStorage should show reps=45 and weight=0 (default).
    const session = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_active_session') ?? 'null'),
    );
    expect(session).not.toBeNull();
    const plankLog = session.exercise_logs.find((e: { exercise_name: string }) => e.exercise_name === 'Plank');
    expect(plankLog).toBeTruthy();
    expect(plankLog.sets[0].reps).toBe(45);
    expect(plankLog.sets[0].weight).toBe(0);
    expect(plankLog.sets[0].completed).toBe(true);
  });

  test('freeform tab: adding a weightless body-only library entry renders only a reps input', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.new-exercise-btn');

    await page.locator('.new-exercise-btn').click();
    await page.waitForSelector('.new-exercise-search');
    // Plank is on the body-only list and NOT on WEIGHTABLE_BODYWEIGHT_IDS, so
    // it should hide the weight input.
    await page.locator('.new-exercise-search').fill('plank');
    await page.waitForSelector('.new-exercise-result');
    await page
      .locator('.new-exercise-result')
      .filter({ has: page.getByText('Plank', { exact: true }) })
      .first()
      .click();

    const card = page.locator('.ex-card').filter({ hasText: 'Plank' });
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
      exercise_name: 'Plank',
      exercise_id: 'Plank',
      session_id: null,
      day_id: null,
      day_name: null,
      target_sets: 3,
      reps_min: 30,
      reps_max: 60,
      sets: [
        { set_number: 1, reps: 45, weight: 0, completed: true, completed_date: TODAY },
        { set_number: 2, reps: 40, weight: 0, completed: true, completed_date: TODAY },
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
    expect(after[0].sets[0].reps).toBe(45);
    expect(after[0].sets[1].weight).toBe(0);
    expect(after[0].sets[1].reps).toBe(40);
  });
});

// ── Weightable bodyweight exceptions ─────────────────────────────────────────
//
// "Body only" exercises where adding weight is common (Decline Crunch with a
// plate, weighted Pullups via belt, etc.) keep the weight × reps inputs.

function weightableScheduled(date: string, libraryId: string, name: string) {
  return {
    id: `w-${libraryId}`,
    date,
    name: `Weighted ${name}`,
    rationale: '',
    source: 'Coach',
    exercises: [
      {
        library_id: libraryId,
        name,
        target_sets: 3,
        reps_min: 8,
        reps_max: 12,
        rest_seconds: 90,
        notes: null,
      },
    ],
    created_at: `${date}T08:00:00.000Z`,
  };
}

async function startWeightableSession(
  page: import('@playwright/test').Page,
  libraryId: string,
  name: string,
) {
  await enableDateMock(page, MOCK_NOW);
  await freshPage(page);
  await page.evaluate(
    (val) => localStorage.setItem('ws_scheduled_workouts', val),
    JSON.stringify([weightableScheduled(TODAY, libraryId, name)]),
  );
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.locator('button').filter({ hasText: 'Start workout' }).click();
  await page.waitForSelector('.ex-card');
  await openExercise(page, 0);
}

test.describe('Weightable body-only exercises keep the weight input', () => {
  test('Decline Crunch shows weight × reps (it can be loaded with a plate)', async ({ page }) => {
    await startWeightableSession(page, 'Decline_Crunch', 'Decline Crunch');
    const firstRow = page.locator('.ex-card').filter({ hasText: 'Decline Crunch' }).locator('.set-row').first();
    await expect(firstRow.locator('.set-num-input')).toHaveCount(2);
    await expect(firstRow.locator('.set-x')).toHaveCount(1);
    await expect(firstRow.locator('.set-num-input').first()).toHaveAttribute('placeholder', 'wt');
    await expect(firstRow.locator('.set-num-input').last()).toHaveAttribute('placeholder', 'reps');
  });

  test('Pullups shows weight × reps (commonly loaded via dip belt)', async ({ page }) => {
    await startWeightableSession(page, 'Pullups', 'Pullups');
    const firstRow = page.locator('.ex-card').filter({ hasText: 'Pullups' }).locator('.set-row').first();
    await expect(firstRow.locator('.set-num-input')).toHaveCount(2);
    await expect(firstRow.locator('.set-x')).toHaveCount(1);
  });

  test('Dips - Triceps Version shows weight × reps', async ({ page }) => {
    await startWeightableSession(page, 'Dips_-_Triceps_Version', 'Dips - Triceps Version');
    const firstRow = page.locator('.ex-card').filter({ hasText: 'Dips - Triceps Version' }).locator('.set-row').first();
    await expect(firstRow.locator('.set-num-input')).toHaveCount(2);
    await expect(firstRow.locator('.set-x')).toHaveCount(1);
  });

  test('logging weighted Decline Crunch persists both weight and reps', async ({ page }) => {
    await startWeightableSession(page, 'Decline_Crunch', 'Decline Crunch');
    const firstRow = page.locator('.ex-card').filter({ hasText: 'Decline Crunch' }).locator('.set-row').first();
    const inputs = firstRow.locator('.set-num-input');
    await inputs.nth(0).fill('25');
    await inputs.nth(0).press('Tab');
    await inputs.nth(1).fill('10');
    await inputs.nth(1).press('Tab');
    await firstRow.locator('.set-done-btn').evaluate((el: HTMLElement) => el.click());

    const session = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_active_session') ?? 'null'),
    );
    const log = session.exercise_logs.find((e: { exercise_name: string }) => e.exercise_name === 'Decline Crunch');
    expect(log.sets[0].weight).toBe(25);
    expect(log.sets[0].reps).toBe(10);
    expect(log.sets[0].completed).toBe(true);
  });
});
