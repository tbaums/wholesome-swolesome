import { test, expect, Page } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';

const BASE = 'http://localhost:8080';
const SHOTS_DIR = path.join(__dirname, 'screenshots', 'walkthrough_transfemme');

fs.mkdirSync(SHOTS_DIR, { recursive: true });

let shotIdx = 0;
async function shot(page: Page, label: string) {
  shotIdx++;
  const n = String(shotIdx).padStart(2, '0');
  const file = path.join(SHOTS_DIR, `${n}_${label}.png`);
  await page.screenshot({ path: file, fullPage: true });
  console.log(`  📸 ${path.basename(file)}`);
}

// The user's prompt, hand-translated into a structured workout request.
// Femme-shaping bias: glutes / hams / hip width / posterior chain / posture,
// deemphasize lat width, trap bulk, chest mass, and direct biceps volume.
const COACH_RESPONSE = JSON.stringify(
  {
    name: 'Glutes + Posterior Chain (Femme-Shaping Day 1)',
    rationale:
      'Prioritizes glute medius/maximus, hamstrings, and hip abductors to build the curve through hips and seat. Adds upper-back posture work (face pulls, rear-delt flyes) so shoulders sit back and chest opens without adding lat flare. Skips direct biceps, heavy pressing, and shrugs — those reinforce the V-taper / trap bulk you want to deemphasize. 60-min total, hypertrophy rep ranges, moderate rest.',
    exercises: [
      {
        library_id: null,
        name: 'Barbell Hip Thrust',
        target_sets: 4,
        reps_min: 8,
        reps_max: 12,
        rest_seconds: 90,
        notes: 'Pause 1s at lockout; full hip extension. Primary glute mass driver.',
      },
      {
        library_id: null,
        name: 'Romanian Deadlift',
        target_sets: 4,
        reps_min: 8,
        reps_max: 10,
        rest_seconds: 120,
        notes: 'Hinge bias — hamstrings + glute fibers, minimal lower-back drive.',
      },
      {
        library_id: null,
        name: 'Cable Hip Abduction (standing)',
        target_sets: 3,
        reps_min: 12,
        reps_max: 15,
        rest_seconds: 60,
        notes: 'Glute medius — builds hip width / shelf above the seat.',
      },
      {
        library_id: null,
        name: 'Bulgarian Split Squat (dumbbell)',
        target_sets: 3,
        reps_min: 10,
        reps_max: 12,
        rest_seconds: 75,
        notes: 'Long stride, torso slightly forward — biases glute over quad.',
      },
      {
        library_id: null,
        name: 'Face Pull (cable, rope)',
        target_sets: 3,
        reps_min: 12,
        reps_max: 15,
        rest_seconds: 60,
        notes: 'Rear delts + external rotation — posture, opens chest without bulking pecs.',
      },
      {
        library_id: null,
        name: 'Seated Cable Row (close grip, light)',
        target_sets: 3,
        reps_min: 12,
        reps_max: 15,
        rest_seconds: 60,
        notes: 'Mid-back thickness for posture; avoid wide-grip lat work that flares the back.',
      },
    ],
  },
  null,
  2,
);

async function freshFromBase(page: Page) {
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate(() => localStorage.clear());
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

test.describe('Transfemme walkthrough', () => {
  test('full goal → coach → session → history flow', async ({ page }) => {
    test.setTimeout(120_000);

    // ── 1. Boot ───────────────────────────────────────────────────────────────
    await freshFromBase(page);
    await shot(page, 'home_empty');

    // ── 2. Library tab ────────────────────────────────────────────────────────
    await page.locator('.nav-btn').filter({ hasText: 'Library' }).click();
    await page.waitForSelector('.library-item', { timeout: 10_000 });
    await shot(page, 'library_tab');

    // ── 3. Exercises tab ──────────────────────────────────────────────────────
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.new-exercise-btn');
    await shot(page, 'exercises_tab');

    // ── 4. History tab (empty heatmap) ────────────────────────────────────────
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.body-svg');
    await shot(page, 'history_empty_heatmap');

    // ── 5. Open Options (Goals editor) ────────────────────────────────────────
    await page.locator('.page-header').locator('text=Options').click();
    await page.waitForSelector('text=Training goals');
    await shot(page, 'options_initial');

    // ── 6. Pick primary goal: Hypertrophy ─────────────────────────────────────
    await page.locator('.goal-pill').filter({ hasText: 'Hypertrophy' }).click();
    await expect(
      page.locator('.goal-pill.active').filter({ hasText: 'Hypertrophy' }),
    ).toBeVisible();

    // ── 7. Sessions/week = 4, session minutes = 60 ────────────────────────────
    const numberInputs = page.locator('.card input[type="number"]');
    await numberInputs.nth(0).fill('4');
    await numberInputs.nth(0).press('Tab');
    await numberInputs.nth(1).fill('60');
    await numberInputs.nth(1).press('Tab');

    // ── 8. Equipment: barbell, dumbbell, cable, machine ───────────────────────
    for (const eq of ['barbell', 'dumbbell', 'cable', 'machine']) {
      await page.locator('.goal-pill').filter({ hasText: new RegExp(`^${eq}$`) }).click();
    }

    // ── 9. Notes textarea — the user's actual prompt ──────────────────────────
    const textareas = page.locator('.card textarea');
    // avoid box first
    await textareas.nth(0).fill(
      'No heavy direct biceps work, no shrugs, no wide-grip pulldowns. Avoid anything that thickens lats or traps.',
    );
    await textareas.nth(0).press('Tab');
    // notes box second
    await textareas.nth(1).fill(
      "I'm a transfemme person trying to maximize a femme silhouette and deemphasize my muscley masc default. Bias glutes, hamstrings, hip abductors, and posture (rear delts, mid-back). Avoid lat width, trap bulk, chest mass, and direct biceps volume. Aesthetics over strength PRs.",
    );
    await textareas.nth(1).press('Tab');

    await shot(page, 'options_goals_filled');

    // ── 10. Back to Home, open Coach Brief ───────────────────────────────────
    await page.locator('.back-btn').click();
    await page.waitForSelector('.bottom-nav');
    await page.locator('.nav-btn').filter({ hasText: 'Workout' }).click();
    await page.waitForSelector('.today-card');
    await shot(page, 'home_after_goals');

    await page.locator('button', { hasText: 'Generate workout with Claude' }).click();
    await page.waitForSelector('.coach-packet-pre');
    await shot(page, 'coach_brief_top');

    // Override target date to TODAY so the imported workout shows in the
    // TODAY card (which is the only card with a Start button).
    const todayStr = await page.evaluate(() => {
      const d = new Date();
      const pad = (n: number) => String(n).padStart(2, '0');
      return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
    });
    await page.locator('input[type="date"]').fill(todayStr);

    // Scroll to bottom of the brief so paste textarea is visible
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await shot(page, 'coach_brief_bottom');

    // ── 11. Paste the Claude-style JSON response and import ──────────────────
    const pasteBox = page.locator('textarea').filter({ hasText: '' }).last();
    await pasteBox.fill(COACH_RESPONSE);
    await shot(page, 'coach_response_pasted');

    await page.locator('button', { hasText: 'Import workout' }).click();
    await expect(page.locator('text=✓ Imported')).toBeVisible();
    await shot(page, 'coach_imported');

    // ── 12. Back to Home — should see TODAY card (target_date = tomorrow) ─────
    // Importer defaults to tomorrow, so jump to the upcoming list instead of TODAY.
    await page.locator('.back-btn').click();
    await page.waitForSelector('.today-card');
    await shot(page, 'home_with_scheduled');

    // ── 13. Start the workout from the TODAY card ───────────────────────────
    await page
      .locator('.today-card button', { hasText: 'Start workout' })
      .first()
      .click();

    await page.waitForSelector('.ex-card', { timeout: 10_000 });
    await shot(page, 'session_initial');

    // ── 14. Log a few sets on the first exercise (Hip Thrust) ───────────────
    const firstChevron = page.locator('.exercise-chevron').first();
    await firstChevron.click();
    await page.waitForFunction(() => {
      const bodies = document.querySelectorAll('.exercise-body');
      return bodies[0]?.classList.contains('open');
    });
    await shot(page, 'session_exercise_open');

    // Fill the first set: 95 lb x 10 reps
    const firstRow = page.locator('.set-row').first();
    const firstInputs = firstRow.locator('.set-num-input');
    await firstInputs.nth(0).fill('95');
    await firstInputs.nth(0).press('Tab');
    await firstInputs.nth(1).fill('10');
    await firstInputs.nth(1).press('Tab');
    await page.locator('.set-done-btn').first().evaluate((el: HTMLElement) => el.click());

    // Second set
    const secondRow = page.locator('.set-row').nth(1);
    const secondInputs = secondRow.locator('.set-num-input');
    await secondInputs.nth(0).fill('105');
    await secondInputs.nth(0).press('Tab');
    await secondInputs.nth(1).fill('9');
    await secondInputs.nth(1).press('Tab');
    await page.locator('.set-done-btn').nth(1).evaluate((el: HTMLElement) => el.click());

    await shot(page, 'session_two_sets_logged');

    // ── 15. Mark every remaining set done quickly ────────────────────────────
    const exCount = await page.locator('.ex-card').count();
    for (let i = 1; i < exCount; i++) {
      await page.locator('.exercise-chevron').nth(i).click();
    }
    // small wait for animation
    await page.waitForTimeout(300);

    const doneBtns = page.locator('.set-done-btn');
    const total = await doneBtns.count();
    for (let j = 2; j < total; j++) {
      // Fill weight/reps with a default so they look real
      const row = page.locator('.set-row').nth(j);
      const inputs = row.locator('.set-num-input');
      if ((await inputs.count()) >= 2) {
        await inputs.nth(0).fill('40');
        await inputs.nth(0).press('Tab');
        await inputs.nth(1).fill('12');
        await inputs.nth(1).press('Tab');
      }
      await doneBtns.nth(j).evaluate((el: HTMLElement) => el.click());
    }
    await shot(page, 'session_all_sets_done');

    // ── 16. Finish workout ───────────────────────────────────────────────────
    await page.locator('.btn-finish').click();
    // App routes to history or home after finish — wait for either
    await page.waitForFunction(() => {
      return (
        document.querySelector('.body-svg') !== null ||
        document.querySelector('.today-card') !== null
      );
    });
    await shot(page, 'after_finish');

    // ── 17. Open History tab, see the session + heatmap shading ─────────────
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.body-svg');
    await shot(page, 'history_after_session');

    // Scroll to bottom for the session list
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await shot(page, 'history_session_list');
  });
});
