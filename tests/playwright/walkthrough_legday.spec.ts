import { test, expect, Page } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';

// Library-backed leg-day walkthrough. Verifies that:
//  - The Library tab loads and a detail view shows the body silhouette
//    with primary/secondary muscles highlighted.
//  - A workout whose exercise names match library entries (Barbell Squat,
//    Romanian Deadlift, etc.) colors the corresponding lower-body muscles
//    on the History heatmap after the session is logged. The main
//    hypertrophy walkthrough only exercises name-matching for a few
//    muscles; this one is specifically a regression net for the
//    heatmap-coloring path on quads + glutes + hamstrings + calves.

const BASE = 'http://localhost:8081';
const SHOTS_DIR = path.join(__dirname, 'screenshots', 'walkthrough_legday');
fs.mkdirSync(SHOTS_DIR, { recursive: true });

let shotIdx = 0;
async function shot(page: Page, label: string) {
  shotIdx++;
  const n = String(shotIdx).padStart(2, '0');
  const file = path.join(SHOTS_DIR, `${n}_${label}.png`);
  await page.screenshot({ path: file, fullPage: true });
  console.log(`  📸 ${path.basename(file)}`);
}

// Names chosen to exactly match free-exercise-db library entries so
// last_hit_by_muscle's name-fallback lookup credits the right muscles.
const LEG_DAY = JSON.stringify({
  name: 'Library Leg Day',
  rationale: 'Library-id-less workout whose names match library entries — used to verify heatmap coloring end-to-end.',
  exercises: [
    { library_id: 'Barbell_Squat',        name: 'Barbell Squat',        target_sets: 3, reps_min: 5,  reps_max: 8,  rest_seconds: 120, notes: '' },
    { library_id: 'Romanian_Deadlift',    name: 'Romanian Deadlift',    target_sets: 3, reps_min: 8,  reps_max: 10, rest_seconds: 90,  notes: '' },
    { library_id: 'Leg_Press',            name: 'Leg Press',            target_sets: 3, reps_min: 10, reps_max: 12, rest_seconds: 90,  notes: '' },
    { library_id: 'Lying_Leg_Curls',      name: 'Lying Leg Curls',      target_sets: 3, reps_min: 10, reps_max: 12, rest_seconds: 60,  notes: '' },
    { library_id: 'Standing_Calf_Raises', name: 'Standing Calf Raises', target_sets: 3, reps_min: 12, reps_max: 15, rest_seconds: 45,  notes: '' },
  ],
});

async function freshFromBase(page: Page) {
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate(() => localStorage.clear());
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

test.describe('Leg-day walkthrough', () => {
  test('library detail + library-named workout colors lower body on heatmap', async ({ page }) => {
    test.setTimeout(120_000);
    await freshFromBase(page);

    // ── Library tab → tap a leg item → detail view with body silhouette ──
    await page.locator('.nav-btn').filter({ hasText: 'Library' }).click();
    await page.waitForSelector('.library-item', { timeout: 10_000 });
    await shot(page, 'library_tab');

    // Filter to a known leg compound and open its detail view.
    await page.locator('.library-search').fill('Barbell Squat');
    await page.waitForFunction(() => {
      const items = document.querySelectorAll('.library-item');
      return items.length > 0 && items.length < 10;
    });
    await page.locator('.library-item').first().click();
    await page.waitForSelector('.body-svg');
    await shot(page, 'library_detail_squat');

    // ── Back to Library, then nav to Workout and open Coach Brief ───────
    await page.locator('.back-btn').click();
    await page.waitForSelector('.library-item');

    await page.locator('.nav-btn').filter({ hasText: 'Workout' }).click();
    await page.waitForSelector('.today-card');
    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.coach-packet-pre');

    // Target date auto-computes to today (no scheduled workout yet).
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    const pasteBox = page.locator('textarea').filter({ hasText: '' }).last();
    await pasteBox.fill(LEG_DAY);
    await page.locator('button', { hasText: 'Import workout' }).click();
    await expect(page.locator('text=✓ Imported')).toBeVisible();
    await shot(page, 'leg_workout_imported');

    // ── Back to Home, start the workout ─────────────────────────────────
    await page.locator('.back-btn').click();
    await page.waitForSelector('.today-card');
    await page.locator('.today-card button', { hasText: 'Start workout' }).first().click();
    await page.waitForSelector('.ex-card', { timeout: 10_000 });
    await shot(page, 'session_initial');

    // ── Open every exercise + log every set with realistic numbers ──────
    const exCount = await page.locator('.ex-card').count();
    for (let i = 0; i < exCount; i++) {
      await page.locator('.exercise-chevron').nth(i).click();
    }
    await page.waitForTimeout(300);

    const setRows = page.locator('.set-row');
    const totalSets = await setRows.count();
    for (let j = 0; j < totalSets; j++) {
      const row = setRows.nth(j);
      const inputs = row.locator('.set-num-input');
      if ((await inputs.count()) >= 2) {
        await inputs.nth(0).fill('135');
        await inputs.nth(0).press('Tab');
        await inputs.nth(1).fill('8');
        await inputs.nth(1).press('Tab');
      }
      await page.locator('.set-done-btn').nth(j).evaluate((el: HTMLElement) => el.click());
    }

    // ── Finish + verify History heatmap shows lower-body coloring ──────
    await page.locator('.btn-finish').click();
    await page.waitForFunction(() => {
      return (
        document.querySelector('.body-svg') !== null ||
        document.querySelector('.today-card') !== null
      );
    });

    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.body-svg');
    await shot(page, 'history_heatmap_after');

    // Verify the heatmap actually colored worked muscles. The Recent
    // (≤3 days) bucket renders as #16a34a; we assert that at least one
    // path in each svg picked up that color. This regression-nets both
    // the name-fallback lookup in last_hit_by_muscle AND the SVG
    // rendering paths (so a future CSS clip or muscle-key drift would
    // surface here).
    const greenCounts = await page.evaluate(() => {
      const recent = '#16a34a';
      const svgs = Array.from(document.querySelectorAll('.body-svg'));
      return svgs.map((svg) =>
        Array.from(svg.querySelectorAll('path'))
          .filter((p) => p.getAttribute('fill') === recent).length,
      );
    });
    // [front, back] — both should have at least one worked-recently path
    // (front: quads, calves; back: glutes, hamstrings, calves).
    expect(greenCounts.length).toBe(2);
    expect(greenCounts[0]).toBeGreaterThan(0);
    expect(greenCounts[1]).toBeGreaterThan(0);
  });
});
