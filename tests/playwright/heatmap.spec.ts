import { test, expect } from '@playwright/test';
import { freshPage } from './helpers';

test.describe('Body heatmap rendering', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    await page.waitForSelector('.body-svg');
  });

  test('SVG elements have a viewBox attribute', async ({ page }) => {
    const svgs = page.locator('.body-svg');
    await expect(svgs).toHaveCount(2);

    for (let i = 0; i < 2; i++) {
      const vb = await svgs.nth(i).getAttribute('viewBox');
      expect(vb).not.toBeNull();
      expect(vb).toContain('260');
      expect(vb).toContain('480');
    }
  });

  test('SVG container does not clip content', async ({ page }) => {
    const svg = page.locator('.body-svg').first();
    const overflow = await svg.evaluate(
      (el) => window.getComputedStyle(el).overflow,
    );
    expect(overflow).toBe('visible');
  });
});
