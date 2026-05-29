import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8080';

// A response payload that mirrors what Claude returns: a workout
// plus an optional top-level `vitals` block extracted from an
// Apple Health screenshot.
function responseWithVitals(opts: {
  vo2: number;
  vitalsDate: string;
}) {
  return JSON.stringify({
    name: 'Push + Vitals',
    rationale: 'Test workout to exercise the vitals import path.',
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
    vitals: {
      vo2_max: opts.vo2,
      source_date: opts.vitalsDate,
    },
  });
}

async function openCoachBrief(page: import('@playwright/test').Page) {
  await page.locator('button').filter({ hasText: 'Generate workout with Claude' }).click();
  await page.waitForSelector('.coach-packet-pre');
}

async function readGoalsFromStorage(page: import('@playwright/test').Page) {
  return await page.evaluate(() => {
    const raw = localStorage.getItem('ws_goals');
    return raw ? JSON.parse(raw) : null;
  });
}

test.describe('Coach Brief: vitals import', () => {
  test('pasting a response with vitals updates both the scheduled workout AND the VO2 max', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    // Seed an existing older VO2 max so we know the import is the source of the new value.
    await page.evaluate(() => {
      localStorage.setItem(
        'ws_goals',
        JSON.stringify({
          primary_goal: 'Hypertrophy',
          sessions_per_week: 4,
          session_minutes: 60,
          equipment: [],
          avoid: '',
          notes: '',
          vo2_max_latest: 30.0,
          vo2_max_updated: '2026-05-01',
        }),
      );
    });
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    await openCoachBrief(page);

    const payload = responseWithVitals({ vo2: 36.4, vitalsDate: '2026-06-09' });
    await page.locator('textarea').last().fill(payload);
    await page.locator('button').filter({ hasText: 'Import workout' }).click();

    // Status line should mention the vitals update.
    await expect(page.locator('text=/VO2 max → 36\\.0|VO2 max → 36\\.4/')).toBeVisible();

    // Confirm goals updated in storage.
    const goals = await readGoalsFromStorage(page);
    expect(goals.vo2_max_latest).toBeCloseTo(36.4, 1);
    expect(goals.vo2_max_updated).toBe('2026-06-09');

    // And the scheduled workout landed for today.
    const scheduled = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_scheduled_workouts') ?? '[]'),
    );
    expect(scheduled).toHaveLength(1);
    expect(scheduled[0].name).toBe('Push + Vitals');
    expect(scheduled[0].date).toBe(TODAY);
  });

  test('vitals with an older source_date are dropped silently; workout still imports', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    await page.evaluate(() => {
      localStorage.setItem(
        'ws_goals',
        JSON.stringify({
          primary_goal: 'Hypertrophy',
          sessions_per_week: 4,
          session_minutes: 60,
          equipment: [],
          avoid: '',
          notes: '',
          vo2_max_latest: 36.4,
          vo2_max_updated: '2026-06-01',
        }),
      );
    });
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    await openCoachBrief(page);

    // Older source_date than what's already stored.
    const payload = responseWithVitals({ vo2: 28.0, vitalsDate: '2026-05-10' });
    await page.locator('textarea').last().fill(payload);
    await page.locator('button').filter({ hasText: 'Import workout' }).click();

    // Workout import succeeds.
    await expect(page.locator('text=/✓ Imported/')).toBeVisible();

    // Goals NOT overwritten by the older value.
    const goals = await readGoalsFromStorage(page);
    expect(goals.vo2_max_latest).toBeCloseTo(36.4, 1);
    expect(goals.vo2_max_updated).toBe('2026-06-01');
  });

  test('response with no vitals block leaves existing goals untouched', async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
    await page.evaluate(() => {
      localStorage.setItem(
        'ws_goals',
        JSON.stringify({
          primary_goal: 'Hypertrophy',
          sessions_per_week: 4,
          session_minutes: 60,
          equipment: [],
          avoid: '',
          notes: '',
          vo2_max_latest: 36.4,
          vo2_max_updated: '2026-06-01',
        }),
      );
    });
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    await openCoachBrief(page);

    const payload = JSON.stringify({
      name: 'No-vitals Day',
      rationale: '',
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
    });
    await page.locator('textarea').last().fill(payload);
    await page.locator('button').filter({ hasText: 'Import workout' }).click();

    await expect(page.locator('text=/✓ Imported/')).toBeVisible();

    const goals = await readGoalsFromStorage(page);
    expect(goals.vo2_max_latest).toBeCloseTo(36.4, 1);
    expect(goals.vo2_max_updated).toBe('2026-06-01');
  });
});
