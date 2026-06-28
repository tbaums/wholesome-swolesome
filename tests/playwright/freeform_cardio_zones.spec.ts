import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:8081';

test.describe('Freeform cardio — Z1–Z5 zone grid', () => {
  test('a freeform cardio exercise shows the zone grid (not min @ RPE) and persists per-zone minutes', async ({ page }) => {
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    // Exercises tab → add a freeform cardio exercise from the library picker.
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-search').fill('Jogging');
    await page.locator('.new-exercise-result').filter({ hasText: 'Jogging, Treadmill' }).first().click();

    // The added card opens; its set row is a Z1–Z5 zone grid (library loads async).
    const zoneRow = page.locator('.set-row-zones').first();
    await expect(zoneRow).toBeVisible({ timeout: 10_000 });

    const zones = zoneRow.locator('.zone-row');
    await expect(zones).toHaveCount(5);
    await expect(zones.nth(0).locator('.zone-label')).toHaveText('Z1');
    await expect(zones.nth(4).locator('.zone-label')).toHaveText('Z5');
    await expect(zones.first().locator('.zone-input')).toHaveAttribute('placeholder', 'min');

    // The old single "min @ RPE" fallback is gone — no "@" separator.
    await expect(zoneRow.locator('.set-x')).toHaveCount(0);

    // Type minutes into Zone 2 → persists onto the freeform entry's zone_minutes.
    await zones.nth(1).locator('.zone-input').fill('12');
    await expect
      .poll(async () =>
        page.evaluate(() => {
          const h = JSON.parse(localStorage.getItem('ws_ex_history') || '[]');
          const e = h.find((x: { exercise_id: string }) => x.exercise_id === 'Jogging_Treadmill');
          const zm = e?.sets?.[0]?.zone_minutes as { zone: number; minutes: number }[] | undefined;
          return zm?.find((z) => z.zone === 2)?.minutes ?? null;
        }),
      )
      .toBe(12);
  });
});
