import { Page } from '@playwright/test';

/** Register a JS Date mock that intercepts zero-arg `new Date()` and `Date.now()`.
 *  Must be called before the page navigates (e.g. before freshPage).
 *  The mock is active for all subsequent navigations in this test. */
export async function enableDateMock(page: Page, initialIso: string) {
  await page.addInitScript({
    content: `(() => {
      const RealDate = Date;
      let mockMs = new RealDate(${JSON.stringify(initialIso)}).getTime();
      class MockDate extends RealDate {
        constructor(...args) {
          if (args.length === 0) super(mockMs);
          else super(...args);
        }
        static now() { return mockMs; }
        static advanceTo(isoStr) { mockMs = new RealDate(isoStr).getTime(); }
      }
      MockDate.parse = RealDate.parse.bind(RealDate);
      MockDate.UTC = RealDate.UTC.bind(RealDate);
      globalThis.Date = MockDate;
    })();`,
  });
}

/** Advance the mock date (must have called enableDateMock first). */
export async function setMockDate(page: Page, isoStr: string) {
  await page.evaluate((iso) => (globalThis as any).Date.advanceTo(iso), isoStr);
}

/** Compute the YYYY-MM-DD string the browser would derive from an ISO timestamp,
 *  using the same local-time getters as the Rust current_date() function. */
export function localDateStr(isoStr: string): string {
  const d = new Date(isoStr);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

export const BASE = 'http://localhost:8080';

/** Reset localStorage and reload so every test starts from a known blank state. */
export async function freshPage(page: Page) {
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
  await page.evaluate(() => localStorage.clear());
  await page.goto(BASE);
  await page.waitForSelector('.bottom-nav');
}

/** Click a day button (0-indexed) and wait for the session view to appear. */
export async function startWorkout(page: Page, dayIndex = 0) {
  const dayBtns = page.locator('.btn.btn-secondary.btn-full');
  await dayBtns.nth(dayIndex).click();
  await page.waitForSelector('.ex-card');
}

/** Click the chevron to open an exercise accordion (0-indexed) and wait for animation. */
export async function openExercise(page: Page, exerciseIndex = 0) {
  const chevron = page.locator('.exercise-chevron').nth(exerciseIndex);
  await chevron.click();
  await page.waitForFunction(
    (idx: number) => {
      const bodies = document.querySelectorAll('.exercise-body');
      return bodies[idx]?.classList.contains('open');
    },
    exerciseIndex,
  );
}

/**
 * Fill the weight and reps inputs for a set row.
 * Uses Tab to blur the input so Leptos' on:change fires.
 */
export async function fillSet(
  page: Page,
  setIndex: number,
  weight: string,
  reps: string,
) {
  const row = page.locator('.set-row').nth(setIndex);
  const inputs = row.locator('.set-num-input');
  await inputs.nth(0).fill(weight);
  await inputs.nth(0).press('Tab');
  await inputs.nth(1).fill(reps);
  await inputs.nth(1).press('Tab');
}

/** Click the done (✓) button for a set row (0-indexed across the whole page). */
export async function markSetDone(page: Page, setIndex: number) {
  await page.locator('.set-done-btn').nth(setIndex).click();
}

/**
 * Open every exercise accordion and mark every set done.
 * Used to put the session into a fully-completed state.
 *
 * Uses force:true to bypass WebKit's quirky hit-testing of buttons inside
 * overflow:hidden containers (the exercise-body accordion), which otherwise
 * misreports the parent ex-card as the interceptor.
 */
export async function completeAllSets(page: Page) {
  const exCount = await page.locator('.ex-card').count();
  for (let i = 0; i < exCount; i++) {
    await openExercise(page, i);
    const body = page.locator('.exercise-body').nth(i);
    const doneBtns = body.locator('.set-done-btn');
    const btnCount = await doneBtns.count();
    for (let j = 0; j < btnCount; j++) {
      // evaluate click bypasses WebKit's hit-test quirk with overflow:hidden containers
      await doneBtns.nth(j).evaluate((el: HTMLElement) => el.click());
    }
  }
}
