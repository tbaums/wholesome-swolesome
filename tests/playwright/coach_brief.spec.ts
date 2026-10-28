import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

function todayStr(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

function tomorrowStr(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

async function openCoachBrief(page: import('@playwright/test').Page) {
  await page.locator('button').filter({ hasText: 'Generate workout with Claude' }).click();
  await page.waitForSelector('.brief-copy-btn');
}

test.describe('Coach Brief default date', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  test('defaults to today when no workout is scheduled', async ({ page }) => {
    await openCoachBrief(page);

    const dateText = page.locator('.fw-600').filter({ hasText: /^\d{4}-\d{2}-\d{2}$/ });
    await expect(dateText).toHaveText(todayStr());
  });

  test('defaults to tomorrow when today already has a scheduled workout', async ({ page }) => {
    const today = todayStr();
    await page.evaluate(
      (date) => {
        const scheduled = [{
          id: 'w-test',
          date,
          name: 'Test Workout',
          rationale: '',
          source: 'Coach',
          exercises: [],
          created_at: new Date().toISOString(),
        }];
        localStorage.setItem('ws_scheduled_workouts', JSON.stringify(scheduled));
      },
      today,
    );
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await openCoachBrief(page);

    const dateText = page.locator('.fw-600').filter({ hasText: /^\d{4}-\d{2}-\d{2}$/ });
    await expect(dateText).toHaveText(tomorrowStr());
  });
});

// ── The brief is read-on-demand (#49) ────────────────────────────────────────
// Copy and the JSON paste box are the actions; the packet itself lives behind
// the "View brief" overlay so neither one sits below a screenful of markdown.

const WORKOUT_JSON = JSON.stringify({
  name: 'Imported After Viewing',
  rationale: 'Pasted by the test.',
  exercises: [
    {
      library_id: 'Barbell_Squat',
      name: 'Barbell Squat',
      target_sets: 3,
      reps_min: 5,
      reps_max: 8,
      rest_seconds: 120,
      notes: '',
    },
  ],
});

/** Record `navigator.clipboard.writeText` calls instead of writing, so the test
 *  needs no clipboard permission and can compare what each copy path copied.
 *  Must be installed before the page navigates. */
async function recordClipboard(page: import('@playwright/test').Page) {
  await page.addInitScript(() => {
    (window as any).__copied = [];
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: (t: string) => {
          (window as any).__copied.push(t);
          return Promise.resolve();
        },
      },
    });
  });
}

const copied = (page: import('@playwright/test').Page) =>
  page.evaluate(() => (window as any).__copied as string[]);

const pasteBox = (page: import('@playwright/test').Page) =>
  page.locator('.card').filter({ hasText: "Paste Claude's JSON response" }).locator('textarea');

test.describe('Coach Brief: the brief is read-on-demand', () => {
  test.beforeEach(async ({ page }) => {
    await recordClipboard(page);
    await freshPage(page);
    await openCoachBrief(page);
  });

  test('the default screen carries the actions and no brief body', async ({ page }) => {
    // The packet is not on the page at all — nothing to scroll past.
    await expect(page.locator('.coach-packet-pre')).toHaveCount(0);
    await expect(page.locator('.brief-viewer')).toHaveCount(0);

    // Copy is its own full-width row above the secondary pair.
    await expect(page.locator('.brief-copy-btn')).toHaveText('📋 Copy brief');
    await expect(page.locator('.brief-view-btn')).toHaveText('👁 View brief');
    await expect(page.locator('.brief-download-btn')).toHaveText('⬇ Download .md');

    // Every control is reachable without scrolling.
    await expect(page.locator('.brief-copy-btn')).toBeInViewport();
    await expect(page.locator('.brief-view-btn')).toBeInViewport();
    await expect(page.locator('.brief-download-btn')).toBeInViewport();
    await expect(pasteBox(page)).toBeInViewport();
    await expect(page.locator('button', { hasText: 'Import workout' })).toBeInViewport();

    // And the page itself is no longer a tall document.
    const overflow = await page.evaluate(
      () => document.documentElement.scrollHeight - window.innerHeight,
    );
    expect(overflow).toBeLessThanOrEqual(0);
  });

  test('Copy brief copies the whole packet with no overlay opened', async ({ page }) => {
    await page.locator('.brief-copy-btn').click();

    await expect(page.locator('.toast')).toContainText('Copied to clipboard');
    await expect(page.locator('.brief-viewer')).toHaveCount(0);

    const texts = await copied(page);
    expect(texts).toHaveLength(1);
    expect(texts[0]).toContain('# Wholesome Swolesome — Coach Brief');
    expect(texts[0]).toContain('## Response format');
  });

  test('View brief opens a full-screen overlay holding the complete brief', async ({ page }) => {
    await page.locator('.brief-view-btn').click();

    const viewer = page.locator('.brief-viewer');
    await expect(viewer).toBeVisible();
    await expect(viewer.locator('.note-editor-title')).toHaveText('Coach Brief');

    // Top and tail of the packet are both in there.
    const body = viewer.locator('.coach-packet-viewer');
    await expect(body).toContainText('Wholesome Swolesome — Coach Brief');
    await expect(body).toContainText('Response format');

    // Fixed, above the nav, and the bar clears the top safe area.
    const box = await viewer.evaluate((el) => {
      const cs = getComputedStyle(el);
      return { position: cs.position, zIndex: cs.zIndex };
    });
    expect(box.position).toBe('fixed');
    expect(box.zIndex).toBe('400');
    await expect(viewer.locator('.note-editor-bar')).toBeInViewport();

    // Only the overlay body scrolls, and it scrolls all the way to the end.
    const metrics = await body.evaluate((el) => ({
      scrollable: el.scrollHeight > el.clientHeight,
      padBottom: parseFloat(getComputedStyle(el).paddingBottom),
    }));
    expect(metrics.scrollable).toBe(true);
    expect(metrics.padBottom).toBeGreaterThanOrEqual(16);

    const reachedEnd = await body.evaluate((el) => {
      el.scrollTop = el.scrollHeight;
      return el.scrollTop + el.clientHeight >= el.scrollHeight - 2;
    });
    expect(reachedEnd).toBe(true);
  });

  test('Copy inside the overlay copies exactly what Copy brief copies', async ({ page }) => {
    await page.locator('.brief-copy-btn').click();
    await page.locator('.brief-view-btn').click();
    await page.locator('.brief-viewer-copy').click();

    await expect(page.locator('.toast')).toContainText('Copied to clipboard');
    const texts = await copied(page);
    expect(texts).toHaveLength(2);
    expect(texts[1]).toBe(texts[0]);
  });

  test('the copy toast paints above the overlay, not behind it', async ({ page }) => {
    await page.locator('.brief-view-btn').click();
    await page.locator('.brief-viewer-copy').click();
    await expect(page.locator('.toast')).toBeVisible();

    // Playwright's visibility check ignores z-order, and the overlay is opaque
    // and covers the whole viewport — so "visible" alone would still pass with
    // the toast painted underneath it. Compare the stacking instead.
    // elementFromPoint can't answer this either: the toast is pointer-events:
    // none, so it never reports itself as the element at its own centre.
    const stacking = await page.evaluate(() => {
      // The nearest ancestor that creates a stacking context — if both elements
      // share one, their z-indexes are directly comparable.
      const contextRoot = (el: HTMLElement): Element => {
        let node = el.parentElement;
        while (node && node !== document.documentElement) {
          const cs = getComputedStyle(node);
          const positioned = cs.position !== 'static' && cs.zIndex !== 'auto';
          if (
            positioned ||
            cs.transform !== 'none' ||
            cs.filter !== 'none' ||
            parseFloat(cs.opacity) < 1 ||
            cs.isolation === 'isolate' ||
            cs.willChange.includes('transform') ||
            cs.willChange.includes('opacity')
          ) {
            return node;
          }
          node = node.parentElement;
        }
        return document.documentElement;
      };
      const toast = document.querySelector('.toast') as HTMLElement;
      const viewer = document.querySelector('.brief-viewer') as HTMLElement;
      return {
        toastZ: parseInt(getComputedStyle(toast).zIndex, 10),
        viewerZ: parseInt(getComputedStyle(viewer).zIndex, 10),
        viewerOpaque: getComputedStyle(viewer).backgroundColor,
        sharedContext: contextRoot(toast) === contextRoot(viewer),
      };
    });

    expect(stacking.sharedContext).toBe(true);
    expect(stacking.toastZ).toBeGreaterThan(stacking.viewerZ);
    // The overlay really is opaque, which is why the ordering has to hold.
    expect(stacking.viewerOpaque).not.toBe('rgba(0, 0, 0, 0)');
  });

  test('Close returns to the screen with the pasted JSON still in the box', async ({ page }) => {
    await pasteBox(page).fill(WORKOUT_JSON);
    const scrollBefore = await page.evaluate(() => window.scrollY);

    await page.locator('.brief-view-btn').click();
    await expect(page.locator('.brief-viewer')).toBeVisible();
    await page.locator('.brief-viewer-close').click();

    await expect(page.locator('.brief-viewer')).toHaveCount(0);
    await expect(pasteBox(page)).toHaveValue(WORKOUT_JSON);
    // No scroll-jump on close.
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollBefore);
  });

  test('Import still works after viewing and closing the brief', async ({ page }) => {
    await page.locator('.brief-view-btn').click();
    await page.locator('.brief-viewer-close').click();

    await pasteBox(page).fill(WORKOUT_JSON);
    await page.locator('button', { hasText: 'Import workout' }).click();

    await expect(page.locator('text=✓ Imported')).toBeVisible();
    const scheduled = await page.evaluate(
      () => JSON.parse(localStorage.getItem('ws_scheduled_workouts') ?? '[]'),
    );
    expect(scheduled.map((w: { name: string }) => w.name)).toContain('Imported After Viewing');
  });
});
