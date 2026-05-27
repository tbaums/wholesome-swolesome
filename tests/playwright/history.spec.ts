import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

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

test.describe('History list ordering', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('history list shows newest entries first', async ({ page }) => {
    const older = makeEntry({
      id: 'h1',
      exerciseName: 'Squat',
      date: '2026-04-01',
      createdAt: '2026-04-01T10:00:00.000Z',
      dayName: 'Leg Day',
      sets: [{ reps: 5, weight: 225, completed: true }],
    });
    const newer = makeEntry({
      id: 'h2',
      exerciseName: 'Bench Press',
      date: '2026-05-15',
      createdAt: '2026-05-15T10:00:00.000Z',
      dayName: 'Push Day',
      sets: [{ reps: 8, weight: 135, completed: true }],
    });
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([older, newer]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');

    const items = page.locator('.history-item');
    await expect(items).toHaveCount(2);
    await expect(items.first()).toContainText('Bench Press');
    await expect(items.first()).toContainText('2026-05-15');
    await expect(items.last()).toContainText('Squat');
    await expect(items.last()).toContainText('2026-04-01');
  });

  test('entries on the same date sort by id descending', async ({ page }) => {
    const first = makeEntry({
      id: 'aaa',
      exerciseName: 'Curl',
      date: '2026-05-01',
      createdAt: '2026-05-01T09:00:00.000Z',
      sets: [{ reps: 10, weight: 30, completed: true }],
    });
    const second = makeEntry({
      id: 'bbb',
      exerciseName: 'Row',
      date: '2026-05-01',
      createdAt: '2026-05-01T11:00:00.000Z',
      sets: [{ reps: 8, weight: 95, completed: true }],
    });
    await page.evaluate(
      (val) => localStorage.setItem('ws_ex_history', val),
      JSON.stringify([first, second]),
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.history-item');

    const items = page.locator('.history-item');
    await expect(items).toHaveCount(2);
    // bbb has later created_at, so it appears first (newest-first)
    await expect(items.first()).toContainText('Row');
    await expect(items.last()).toContainText('Curl');
  });
});
