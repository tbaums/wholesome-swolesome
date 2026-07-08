import { test, expect, Page } from '@playwright/test';
import { freshPage } from './helpers';

// #42 — the two large free-text goal fields (Injuries/avoid + Notes for the
// coach) are edited through a tap-to-expand full-screen overlay editor. Preview
// cards stand in for the old inline textareas; the editor commits a local draft
// on Done and discards it on Cancel.

async function goToOptions(page: Page) {
  await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
  await page.locator('button').filter({ hasText: 'Options' }).click();
  await page.waitForSelector('input[type="password"]');
}

async function readGoals(page: Page): Promise<any> {
  return page.evaluate(() => {
    const raw = localStorage.getItem('ws_goals');
    return raw ? JSON.parse(raw) : null;
  });
}

test.describe('Note editor overlay (#42)', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
    await goToOptions(page);
  });

  test('both fields render as empty preview cards, not textareas', async ({ page }) => {
    const previews = page.locator('.note-preview');
    await expect(previews).toHaveCount(2);
    await expect(previews.nth(0)).toHaveText(/injuries \/ lifts to avoid/i);
    await expect(previews.nth(1)).toHaveText(/notes for the coach/i);
    await expect(previews.nth(0)).toHaveClass(/is-empty/);
    await expect(previews.nth(1)).toHaveClass(/is-empty/);
    // GoalsEditor no longer renders any textarea (the Options view has none).
    await expect(page.locator('textarea')).toHaveCount(0);
  });

  test('Done commits the draft and persists it to goals.notes', async ({ page }) => {
    await page.locator('.note-preview').nth(1).click();
    await expect(page.locator('.note-editor')).toBeVisible();
    await expect(page.locator('.note-editor-title')).toHaveText('Notes for the coach');

    const text = 'prefer compound lifts; bias glutes';
    await page.locator('.note-editor-area').fill(text);
    await page.locator('button').filter({ hasText: 'Done' }).click();

    // Overlay closes, preview reflects the committed value, storage updated.
    await expect(page.locator('.note-editor')).toHaveCount(0);
    await expect(page.locator('.note-preview').nth(1)).toHaveText(text);
    await expect(page.locator('.note-preview').nth(1)).not.toHaveClass(/is-empty/);
    await expect.poll(async () => (await readGoals(page))?.notes).toBe(text);
  });

  test('Cancel discards the draft — nothing is saved', async ({ page }) => {
    await page.locator('.note-preview').nth(0).click();
    await expect(page.locator('.note-editor')).toBeVisible();
    await expect(page.locator('.note-editor-title')).toHaveText('Injuries / lifts to avoid');

    await page.locator('.note-editor-area').fill('THIS SHOULD NOT BE SAVED');
    await page.locator('button').filter({ hasText: 'Cancel' }).click();

    await expect(page.locator('.note-editor')).toHaveCount(0);
    await expect(page.locator('.note-preview').nth(0)).toHaveClass(/is-empty/);
    // Give any (unexpected) save effect a chance to run, then assert avoid is blank.
    await page.waitForTimeout(300);
    const goals = await readGoals(page);
    expect(goals?.avoid ?? '').toBe('');
  });
});
