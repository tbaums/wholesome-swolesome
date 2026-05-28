import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr, openExercise } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8080';

// ── Fixtures ────────────────────────────────────────────────────────────────

function scheduledWorkout(
  date: string,
  exerciseName: string,
  libraryId: string | null,
  targetSets: number,
) {
  return {
    id: `w-${date}`,
    date,
    name: 'Prefill Test Day',
    rationale: '',
    source: 'Coach',
    exercises: [
      {
        library_id: libraryId,
        name: exerciseName,
        target_sets: targetSets,
        reps_min: 8,
        reps_max: 12,
        rest_seconds: 120,
        notes: null,
      },
    ],
    created_at: `${date}T08:00:00.000Z`,
  };
}

function historyEntry(opts: {
  exerciseName: string;
  exerciseId: string;
  date: string;
  sets: Array<{ reps: number; weight: number; completed: boolean }>;
}) {
  return {
    id: `e-${opts.date}-${opts.exerciseId}`,
    date: opts.date,
    created_at: `${opts.date}T10:00:00.000Z`,
    exercise_name: opts.exerciseName,
    exercise_id: opts.exerciseId,
    session_id: null,
    day_id: null,
    day_name: null,
    target_sets: opts.sets.length,
    reps_min: 8,
    reps_max: 12,
    sets: opts.sets.map((s, i) => ({
      set_number: i + 1,
      reps: s.reps,
      weight: s.weight,
      completed: s.completed,
      completed_date: s.completed ? opts.date : null,
    })),
    finalized: true,
  };
}

async function seedAndStart(
  page: import('@playwright/test').Page,
  history: ReturnType<typeof historyEntry>[],
  scheduled: ReturnType<typeof scheduledWorkout>[],
) {
  await enableDateMock(page, MOCK_NOW);
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate(() => localStorage.clear());
  await page.evaluate(
    ([h, s]) => {
      localStorage.setItem('ws_ex_history', h as string);
      localStorage.setItem('ws_scheduled_workouts', s as string);
    },
    [JSON.stringify(history), JSON.stringify(scheduled)] as [string, string],
  );
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.locator('button').filter({ hasText: 'Start workout' }).click();
  await page.waitForSelector('.ex-card');
  await openExercise(page, 0);
}

// ── Tests ───────────────────────────────────────────────────────────────────

test.describe('Session pre-fill from prior history', () => {
  test('each set inherits weight + reps from the same set_number of the prior workout', async ({ page }) => {
    const history = [
      historyEntry({
        exerciseName: 'Bench Press',
        exerciseId: 'Barbell_Bench_Press_-_Medium_Grip',
        date: '2026-06-03',
        sets: [
          { reps: 10, weight: 135, completed: true },
          { reps: 8, weight: 145, completed: true },
          { reps: 6, weight: 155, completed: true },
        ],
      }),
    ];
    const scheduled = [
      scheduledWorkout(TODAY, 'Bench Press', 'Barbell_Bench_Press_-_Medium_Grip', 3),
    ];
    await seedAndStart(page, history, scheduled);

    const rows = page.locator('.set-row');
    await expect(rows).toHaveCount(3);

    const inputs = (i: number) => rows.nth(i).locator('.set-num-input');
    await expect(inputs(0).nth(0)).toHaveValue('135');
    await expect(inputs(0).nth(1)).toHaveValue('10');
    await expect(inputs(1).nth(0)).toHaveValue('145');
    await expect(inputs(1).nth(1)).toHaveValue('8');
    await expect(inputs(2).nth(0)).toHaveValue('155');
    await expect(inputs(2).nth(1)).toHaveValue('6');
  });

  test('extra sets beyond prior history fall back to the last prior set', async ({ page }) => {
    // Prior workout had 2 sets; today's schedule asks for 4.
    const history = [
      historyEntry({
        exerciseName: 'Squat',
        exerciseId: 'Barbell_Squat',
        date: '2026-06-03',
        sets: [
          { reps: 5, weight: 185, completed: true },
          { reps: 5, weight: 205, completed: true },
        ],
      }),
    ];
    const scheduled = [scheduledWorkout(TODAY, 'Squat', 'Barbell_Squat', 4)];
    await seedAndStart(page, history, scheduled);

    const rows = page.locator('.set-row');
    await expect(rows).toHaveCount(4);

    const inputs = (i: number) => rows.nth(i).locator('.set-num-input');
    await expect(inputs(0).nth(0)).toHaveValue('185');
    await expect(inputs(1).nth(0)).toHaveValue('205');
    // Sets 3 and 4: no matching set_number → inherit from last prior (205 × 5).
    await expect(inputs(2).nth(0)).toHaveValue('205');
    await expect(inputs(2).nth(1)).toHaveValue('5');
    await expect(inputs(3).nth(0)).toHaveValue('205');
    await expect(inputs(3).nth(1)).toHaveValue('5');
  });

  test('with no matching history, weight defaults to 0 and reps to reps_min', async ({ page }) => {
    const scheduled = [scheduledWorkout(TODAY, 'Deadlift', 'Barbell_Deadlift', 3)];
    await seedAndStart(page, [], scheduled);

    const rows = page.locator('.set-row');
    await expect(rows).toHaveCount(3);

    const inputs = (i: number) => rows.nth(i).locator('.set-num-input');
    for (let i = 0; i < 3; i++) {
      // weight 0.0 renders as empty (placeholder "wt" is shown instead of a literal 0)
      await expect(inputs(i).nth(0)).toHaveValue('');
      await expect(inputs(i).nth(1)).toHaveValue('8'); // reps_min from fixture
    }
  });
});
