import { test, expect } from '@playwright/test';
import { enableDateMock, freshPage, localDateStr } from './helpers';

const MOCK_NOW = '2026-06-10T12:00:00.000Z';
const TODAY = localDateStr(MOCK_NOW);
const BASE = 'http://localhost:8081';

async function addCardioExercise(
  page: import('@playwright/test').Page,
  query: string,
  exactName: string,
) {
  await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
  await page.waitForSelector('.new-exercise-btn');
  await page.locator('.new-exercise-btn').click();
  await page.waitForSelector('.new-exercise-search');
  await page.locator('.new-exercise-search').fill(query);
  await page.waitForSelector('.new-exercise-result');
  await page
    .locator('.new-exercise-result')
    .filter({ has: page.getByText(exactName, { exact: true }) })
    .first()
    .click();
  await page.waitForSelector('.ex-card');
}

test.describe('Freeform cardio: Apple Health zone-import flow', () => {
  test.beforeEach(async ({ page }) => {
    await enableDateMock(page, MOCK_NOW);
    await freshPage(page);
  });

  test('adding a freeform cardio exercise shows the import card with the library_id baked in', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');

    const card = page.locator('.ex-card').filter({ hasText: 'Elliptical Trainer' });
    const importCard = card.locator('.cardio-import-card');
    await expect(importCard).toBeVisible();

    // The prompt block embeds the exact library id verbatim so Claude can use it.
    const promptText = await importCard.locator('.ci-prompt').textContent();
    expect(promptText).toContain('Elliptical_Trainer');
    expect(promptText).toContain('cardio_actuals');
    expect(promptText).toContain('Apple Health');

    // Copy + Import buttons present.
    await expect(importCard.locator('.ci-copy-btn')).toBeVisible();
    await expect(importCard.locator('button').filter({ hasText: 'Import zone minutes' })).toBeVisible();
  });

  test('a non-cardio freeform exercise does NOT show the import card', async ({ page }) => {
    await addCardioExercise(page, 'bench press', 'Barbell Bench Press - Medium Grip');
    const card = page.locator('.ex-card').filter({ hasText: 'Bench Press' });
    await expect(card.locator('.cardio-import-card')).toHaveCount(0);
  });

  test('pasting a wrapped {"cardio_actuals": ...} response writes zone_minutes to the draft entry', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');

    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const response = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [
          { zone: 1, minutes: 5 },
          { zone: 2, minutes: 18 },
          { zone: 3, minutes: 9 },
          { zone: 4, minutes: 3 },
        ],
      },
    });
    await importCard.locator('textarea').fill(response);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();

    // Status line acknowledges the import (35 = 5+18+9+3 minutes across 4 zones).
    await expect(importCard.locator('text=/35 min imported across 4 zone\\(s\\)/')).toBeVisible();

    // History has one entry for Elliptical Trainer, dated today, with zone_minutes
    // on the last set and reps == sum of zone minutes.
    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(1);
    const e = entries[0];
    expect(e.exercise_name).toBe('Elliptical Trainer');
    expect(e.date).toBe(TODAY);
    const last = e.sets[e.sets.length - 1];
    expect(last.zone_minutes).toEqual([
      { zone: 1, minutes: 5 },
      { zone: 2, minutes: 18 },
      { zone: 3, minutes: 9 },
      { zone: 4, minutes: 3 },
    ]);
    expect(last.reps).toBe(35);
    expect(last.completed).toBe(true);
    expect(last.completed_date).toBe(TODAY);
  });

  test('a bare (non-wrapped) JSON response also parses', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const bare = JSON.stringify({
      exercise_id: 'Elliptical_Trainer',
      zones: [{ zone: 2, minutes: 20 }],
    });
    await importCard.locator('textarea').fill(bare);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();
    await expect(importCard.locator('text=/20 min imported across 1 zone\\(s\\)/')).toBeVisible();
  });

  test('a fenced ```json … ``` response parses', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const fenced = '```json\n' + JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [{ zone: 2, minutes: 25 }, { zone: 3, minutes: 5 }],
      },
    }) + '\n```';
    await importCard.locator('textarea').fill(fenced);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();
    await expect(importCard.locator('text=/30 min imported across 2 zone\\(s\\)/')).toBeVisible();
  });

  test('import respects the selected_date — a backdated import writes to that date', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');

    // Backdate before importing.
    await page.locator('input[type="date"]').fill('2026-06-08');
    await page.locator('input[type="date"]').dispatchEvent('change');

    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');
    const response = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [{ zone: 2, minutes: 30 }],
      },
    });
    await importCard.locator('textarea').fill(response);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();

    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(1);
    expect(entries[0].date).toBe('2026-06-08');
    expect(entries[0].sets[entries[0].sets.length - 1].completed_date).toBe('2026-06-08');
  });

  test('malformed JSON shows an error and does NOT write history', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    await importCard.locator('textarea').fill('not actually json {');
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();
    await expect(importCard.locator('text=/✗.*JSON parse/')).toBeVisible();

    // The import must NOT silently create an entry with no zones.
    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(0);
  });

  test('prompt now asks Claude for an estimated_rpe', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const promptText = await page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card .ci-prompt')
      .textContent();
    // The fix added the estimated_rpe field to both the example JSON and
    // the inference guidance line.
    expect(promptText).toContain('estimated_rpe');
    expect(promptText).toMatch(/RPE/);
  });

  test('RPE guidance maps Z1/Z2 to 1-3, not the old 4-6', async ({ page }) => {
    // Regression guard: the original prompt told Claude that a mostly-Z2
    // session was RPE 4–6, which is much higher than the standard
    // exercise-physiology mapping (Z1 ≈ RPE 1–2, Z2 ≈ RPE 2–4). The fix
    // brings Z1/Z2 down to 1–3 and pushes the higher zones down a notch.
    // If this test fails, the prompt has drifted back toward the old values.
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const promptText = await page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card .ci-prompt')
      .textContent();
    // Each zone's range should appear within ~30 chars of the zone marker
    // (non-greedy match — catches drift even if other text moves around).
    expect(promptText).toMatch(/Z1\/?Z2[^Z]{0,40}1[–-]3/i);
    expect(promptText).toMatch(/Z3[^Z]{0,40}4[–-]6/i);
    expect(promptText).toMatch(/Z4[^Z]{0,40}7[–-]8/i);
    expect(promptText).toMatch(/Z5[^Z]{0,40}9[–-]10/i);
  });

  test('a response with estimated_rpe writes it to set.weight', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const response = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [
          { zone: 2, minutes: 17.85 },
          { zone: 4, minutes: 15.6 },
        ],
        estimated_rpe: 7,
      },
    });
    await importCard.locator('textarea').fill(response);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();

    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(1);
    const last = entries[0].sets[entries[0].sets.length - 1];
    expect(last.weight).toBe(7);
    expect(last.zone_minutes.length).toBe(2);
  });

  test('a response WITHOUT estimated_rpe leaves set.weight untouched (back-compat)', async ({ page }) => {
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const response = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [{ zone: 2, minutes: 20.0 }],
      },
    });
    await importCard.locator('textarea').fill(response);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();

    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    const last = entries[0].sets[entries[0].sets.length - 1];
    // Default weight (RPE) is 0 — confirming the import did NOT clobber it
    // with a synthetic value when the response omitted estimated_rpe.
    expect(last.weight).toBe(0);
    expect(last.zone_minutes[0].minutes).toBe(20);
  });

  test('import auto-finalizes the entry so it shows in the coach brief', async ({ page }) => {
    // Regression: an imported zone payload used to land the entry as a draft
    // (finalized: false) because the user never tapped the per-card ✓. The
    // entry's minutes still counted in cardio_minutes_in_window totals, but
    // the brief's "Recent training" section filtered it out — so the coach
    // never saw what cardio the user actually did. The import is an explicit
    // commit and should finalize.
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const response = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [{ zone: 2, minutes: 25 }, { zone: 3, minutes: 5 }],
        estimated_rpe: 4,
      },
    });
    await importCard.locator('textarea').fill(response);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();

    const entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(1);
    expect(entries[0].finalized).toBe(true);
  });

  test('re-importing for the same exercise+date updates in place — no duplicate (#39)', async ({ page }) => {
    // Regression: the first import finalizes the entry, and the upsert used to
    // match only non-finalized drafts — so a second import (e.g. a corrected
    // screenshot) appended a DUPLICATE instead of updating in place.
    await addCardioExercise(page, 'elliptical', 'Elliptical Trainer');
    const importCard = page
      .locator('.ex-card')
      .filter({ hasText: 'Elliptical Trainer' })
      .locator('.cardio-import-card');

    const first = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [{ zone: 2, minutes: 20 }],
        estimated_rpe: 5,
      },
    });
    await importCard.locator('textarea').fill(first);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();
    await expect(importCard.locator('text=/20 min imported across 1 zone\\(s\\)/')).toBeVisible();

    let entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    expect(entries).toHaveLength(1);

    // Second import: corrected screenshot for the SAME exercise + date.
    const second = JSON.stringify({
      cardio_actuals: {
        exercise_id: 'Elliptical_Trainer',
        zones: [{ zone: 2, minutes: 25 }, { zone: 3, minutes: 5 }],
        estimated_rpe: 6,
      },
    });
    await importCard.locator('textarea').fill(second);
    await importCard.locator('button').filter({ hasText: 'Import zone minutes' }).click();
    await expect(importCard.locator('text=/30 min imported across 2 zone\\(s\\)/')).toBeVisible();

    entries = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_ex_history') ?? '[]'),
    );
    // Still exactly ONE entry — updated in place, not duplicated.
    expect(entries).toHaveLength(1);
    const e = entries[0];
    expect(e.date).toBe(TODAY);
    expect(e.finalized).toBe(true);
    const last = e.sets[e.sets.length - 1];
    expect(last.reps).toBe(30);
    expect(last.zone_minutes).toEqual([
      { zone: 2, minutes: 25 },
      { zone: 3, minutes: 5 },
    ]);
    expect(last.weight).toBe(6);
  });
});
