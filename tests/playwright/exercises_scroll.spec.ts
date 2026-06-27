import { test, expect } from '@playwright/test';

const BASE = 'http://localhost:8081';

// A finalized history entry for `name` on `date` (gives the exercise a
// popularity count so it appears in the Exercises-tab list).
function histEntry(name: string, id: string, date: string, n: number) {
  return {
    id: `h-${id}-${n}`,
    date,
    created_at: `${date}T12:00:00.000Z`,
    exercise_name: name,
    exercise_id: id,
    target_sets: 3,
    reps_min: 8,
    reps_max: 12,
    sets: [{ set_number: 1, reps: 10, weight: 50, completed: true, completed_date: date }],
    finalized: true,
  };
}

test.describe('Exercises tab — selecting an exercise surfaces it', () => {
  test('selecting an already-logged exercise moves its card to the top and into view', async ({ page }) => {
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');
    await page.evaluate(() => localStorage.clear());

    // Seed a long history: several POPULAR exercises (3 logs each → top of the
    // popularity-sorted list) plus the TARGET logged once (→ bottom of the list).
    const popular: [string, string][] = [
      ['Ab Roller', 'Ab_Roller'],
      ['Air Bike', 'Air_Bike'],
      ['Barbell Curl', 'Barbell_Curl'],
      ['Barbell Deadlift', 'Barbell_Deadlift'],
      ['Barbell Glute Bridge', 'Barbell_Glute_Bridge'],
      ['Barbell Hack Squat', 'Barbell_Hack_Squat'],
      ['Barbell Shrug', 'Barbell_Shrug'],
      ['Barbell Squat', 'Barbell_Squat'],
      ['Arnold Dumbbell Press', 'Arnold_Dumbbell_Press'],
      ['Barbell Bench Press - Medium Grip', 'Barbell_Bench_Press_-_Medium_Grip'],
    ];
    const history: ReturnType<typeof histEntry>[] = [];
    popular.forEach(([nm, id], i) => {
      for (let k = 0; k < 3; k++) history.push(histEntry(nm, id, `2026-06-0${(i % 5) + 1}`, k));
    });
    // Target: uniquely named, logged once → lowest popularity → bottom of list.
    history.push(histEntry('Anti-Gravity Press', 'Anti-Gravity_Press', '2026-06-01', 0));

    await page.evaluate((val) => localStorage.setItem('ws_ex_history', val), JSON.stringify(history));
    await page.goto(BASE);
    await page.waitForSelector('.bottom-nav');

    // Open the Exercises tab.
    await page.locator('.nav-btn').filter({ hasText: 'Exercises' }).click();
    await page.waitForSelector('.ex-card');

    // Pre-condition: the target is NOT the first card — it's buried near the bottom.
    await expect(page.locator('.ex-card .card-title').first()).not.toHaveText('Anti-Gravity Press');

    // Open the picker, search the target, select it.
    await page.locator('.new-exercise-btn').click();
    await page.locator('.new-exercise-search').fill('Anti-Gravity');
    await page.locator('.new-exercise-result').filter({ hasText: 'Anti-Gravity Press' }).first().click();

    // After selecting: the target card is now FIRST and visible in the viewport,
    // so you can log sets immediately without scrolling down to hunt for it.
    const firstCard = page.locator('.ex-card').first();
    await expect(firstCard.locator('.card-title')).toHaveText('Anti-Gravity Press');
    await expect(firstCard).toBeInViewport();
  });
});
