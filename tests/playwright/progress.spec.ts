import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

// Minimal ExerciseEntry fixture mirroring the Rust struct shape — keep aligned
// with src/models.rs and the schema used by tests/playwright/history.spec.ts.
function makeEntry(opts: {
  id: string;
  exerciseName: string;
  date: string;
  createdAt: string;
  sessionId?: string | null;
  dayName?: string | null;
  sets: Array<{ reps: number; weight: number; completed: boolean }>;
}) {
  return {
    id: opts.id,
    date: opts.date,
    created_at: opts.createdAt,
    exercise_name: opts.exerciseName,
    exercise_id: `ex-${opts.id}`,
    session_id: opts.sessionId ?? null,
    day_id: null,
    day_name: opts.dayName ?? null,
    target_sets: 3,
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

async function openFirstProgress(page: import('@playwright/test').Page) {
  await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
  await page.waitForSelector('.history-item');
  await page.locator('.history-item').first().click();
  await page.waitForSelector('.progress-table');
  await page.getByText('View progress →').click();
  await page.waitForSelector('.page-title');
}

test.describe('Progress view', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('page title shows the exercise name', async ({ page }) => {
    const squat = makeEntry({
      id: 'sq1', exerciseName: 'Squat', date: '2026-04-01', createdAt: '2026-04-01T10:00:00.000Z',
      sets: [{ reps: 5, weight: 225, completed: true }],
    });
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([squat]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await openFirstProgress(page);

    await expect(page.locator('.page-title')).toContainText('Squat');
    await expect(page.locator('.progress-table tbody tr')).toHaveCount(1);
  });

  test('lists every set ever logged for the exercise, newest date first', async ({ page }) => {
    const day1 = makeEntry({
      id: 'a', exerciseName: 'Bench', date: '2026-04-01', createdAt: '2026-04-01T10:00:00.000Z',
      sets: [
        { reps: 8, weight: 100, completed: true },
        { reps: 8, weight: 100, completed: true },
      ],
    });
    const day2 = makeEntry({
      id: 'b', exerciseName: 'Bench', date: '2026-05-01', createdAt: '2026-05-01T10:00:00.000Z',
      sets: [
        { reps: 6, weight: 110, completed: true },
        { reps: 5, weight: 115, completed: false }, // a missed final set
      ],
    });
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([day1, day2]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await openFirstProgress(page);

    const rows = page.locator('.progress-table tbody tr');
    await expect(rows).toHaveCount(4); // 2 + 2 sets
    // Newest first: first two rows are dated 2026-05-01
    await expect(rows.nth(0)).toContainText('2026-05-01');
    await expect(rows.nth(1)).toContainText('2026-05-01');
    await expect(rows.nth(2)).toContainText('2026-04-01');
    // The missed set renders an em dash for "Done"
    await expect(rows.filter({ hasText: '—' })).toHaveCount(1);
  });

  test('personal-best card highlights the heaviest completed set', async ({ page }) => {
    const entry = makeEntry({
      id: 'a', exerciseName: 'Deadlift', date: '2026-04-01', createdAt: '2026-04-01T10:00:00.000Z',
      sets: [
        { reps: 5, weight: 225, completed: true },
        { reps: 3, weight: 275, completed: true },
        { reps: 1, weight: 315, completed: false }, // failed top set — must NOT win
      ],
    });
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([entry]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await openFirstProgress(page);

    // PB card uses 275 (best COMPLETED), not 315 (failed)
    const pb = page.locator('.text-accent').first();
    await expect(pb).toContainText('275');
    await expect(pb).not.toContainText('315');
  });

  test('back button returns to the history list', async ({ page }) => {
    const entry = makeEntry({
      id: 'a', exerciseName: 'Row', date: '2026-04-01', createdAt: '2026-04-01T10:00:00.000Z',
      sets: [{ reps: 8, weight: 95, completed: true }],
    });
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([entry]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');
    await openFirstProgress(page);

    await page.locator('.back-btn').click();
    await expect(page.locator('.history-item')).not.toHaveCount(0);
  });
});
