import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

test.describe('Boot + nav', () => {
  test('app boots and shows the four nav tabs', async ({ page }) => {
    await freshPage(page);
    const nav = page.locator('.bottom-nav');
    await expect(nav.locator('.nav-btn')).toHaveCount(4);
    await expect(nav).toContainText('Workout');
    await expect(nav).toContainText('Library');
    await expect(nav).toContainText('Exercises');
    await expect(nav).toContainText('History');
  });

  test('empty-state Today card shows when no scheduled workout', async ({ page }) => {
    await freshPage(page);
    await expect(page.locator('.today-card')).toContainText('No workout scheduled');
  });

  test('library tab loads and renders items', async ({ page }) => {
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'Library' }).click();
    await page.waitForSelector('.library-item', { timeout: 8000 });
    expect(await page.locator('.library-item').count()).toBeGreaterThan(10);
  });

  test('history tab renders body heatmap', async ({ page }) => {
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await expect(page.locator('.body-svg')).toHaveCount(2);
    await expect(page.locator('.heatmap-legend')).toBeVisible();
  });

  test('Options tab has Goals editor', async ({ page }) => {
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    // Options is reachable via the small button in the History header
    await page.locator('.page-header').locator('text=Options').click();
    await expect(page.locator('text=Training goals')).toBeVisible();
    await expect(page.locator('.goal-pill').first()).toBeVisible();
  });
});
