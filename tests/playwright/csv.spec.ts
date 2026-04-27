import { test, expect } from '@playwright/test';
import { promises as fs } from 'fs';
import { freshPage, startWorkout, openExercise, completeAllSets } from './helpers';

// ── helpers ───────────────────────────────────────────────────────────────────

async function goToImportExport(page: Parameters<typeof freshPage>[0]) {
  await page.locator('.nav-btn').filter({ hasText: 'Plan' }).click();
  await page.waitForSelector('.day-item');
  await page.locator('button').filter({ hasText: 'Import / Export' }).click();
  await page.waitForSelector('textarea');
}

const VALID_CSV =
  'day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes\n' +
  'csv-d1,CSV Day A,csv-e1,CSV Squat,3,8,12,Main,\n' +
  'csv-d1,CSV Day A,csv-e2,CSV Press,3,6,10,Main,\n' +
  'csv-d2,CSV Day B,csv-e3,CSV Row,4,8,12,Main,\n';

// ── tests ─────────────────────────────────────────────────────────────────────

test.describe('CSV import / export', () => {
  test.beforeEach(async ({ page }) => {
    await freshPage(page);
  });

  // 1
  test('Import Plan button is disabled when textarea is empty', async ({ page }) => {
    await goToImportExport(page);
    const importBtn = page.locator('button').filter({ hasText: 'Import Plan' });
    await expect(importBtn).toBeDisabled();
  });

  // 2
  test('Import Plan button enables once text is entered', async ({ page }) => {
    await goToImportExport(page);
    await page.locator('textarea').fill(VALID_CSV);
    const importBtn = page.locator('button').filter({ hasText: 'Import Plan' });
    await expect(importBtn).toBeEnabled();
  });

  // 3
  test('valid CSV import replaces plan and navigates to Plan view', async ({ page }) => {
    await goToImportExport(page);
    await page.locator('textarea').fill(VALID_CSV);
    await page.locator('button').filter({ hasText: 'Import Plan' }).click();
    // Should navigate to PlanEditorView
    await page.waitForSelector('.day-item');
    // The imported days are present
    await expect(page.locator('.day-item').filter({ hasText: 'CSV Day A' })).toBeVisible();
    await expect(page.locator('.day-item').filter({ hasText: 'CSV Day B' })).toBeVisible();
  });

  // 4
  test('valid CSV import shows success toast', async ({ page }) => {
    await goToImportExport(page);
    await page.locator('textarea').fill(VALID_CSV);
    await page.locator('button').filter({ hasText: 'Import Plan' }).click();
    await expect(page.locator('.toast')).toContainText('imported');
  });

  // 5
  test('invalid CSV import shows an error and stays on Import/Export page', async ({ page }) => {
    await goToImportExport(page);
    await page.locator('textarea').fill('this,is,not,valid\nbad row\n');
    await page.locator('button').filter({ hasText: 'Import Plan' }).click();
    // Error paragraph should appear
    await expect(page.locator('p[style*="danger"]')).toBeVisible();
    // Should still be on import/export page (textarea still present)
    await expect(page.locator('textarea')).toBeVisible();
  });

  // 6
  test('exported plan CSV has correct header and contains plan day names', async ({ page }) => {
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      (async () => {
        await goToImportExport(page);
        await page.locator('button').filter({ hasText: 'Download Plan CSV' }).click();
      })(),
    ]);
    expect(download.suggestedFilename()).toBe('workout_plan.csv');
    const path = await download.path();
    const content = await fs.readFile(path!, 'utf8');
    expect(content).toMatch(/^day_id,day_name,exercise_id,exercise_name,target_sets,reps_min,reps_max,category,notes/);
    // Default plan has at least one day
    expect(content.split('\n').length).toBeGreaterThan(2);
  });

  // 7
  test('export-then-import round-trip preserves all day names', async ({ page }) => {
    // First, navigate to plan view and capture current day names
    await page.locator('.nav-btn').filter({ hasText: 'Plan' }).click();
    await page.waitForSelector('.day-item');
    const originalDayNames = await page.locator('.day-item .card-title').allInnerTexts();

    // Export the plan
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      (async () => {
        await page.locator('button').filter({ hasText: 'Import / Export' }).click();
        await page.waitForSelector('textarea');
        await page.locator('button').filter({ hasText: 'Download Plan CSV' }).click();
      })(),
    ]);
    const path = await download.path();
    const csvContent = await fs.readFile(path!, 'utf8');

    // Now clear localStorage and reload, then import the downloaded CSV
    await page.evaluate(() => localStorage.clear());
    await page.reload();
    await page.waitForSelector('.bottom-nav');

    await goToImportExport(page);
    await page.locator('textarea').fill(csvContent);
    await page.locator('button').filter({ hasText: 'Import Plan' }).click();
    await page.waitForSelector('.day-item');

    const importedDayNames = await page.locator('.day-item .card-title').allInnerTexts();
    expect(importedDayNames).toEqual(originalDayNames);
  });

  // 8
  test('exported history CSV has correct header and completed sets', async ({ page }) => {
    // Complete a workout so there's something in history
    await startWorkout(page, 3); // Day 4 — fewest sets
    await completeAllSets(page);
    await page.locator('.btn-finish').click();
    await page.waitForSelector('.history-item');

    // Export history CSV from the History view
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator('button').filter({ hasText: 'Export CSV' }).click(),
    ]);
    expect(download.suggestedFilename()).toBe('workout_history.csv');
    const path = await download.path();
    const content = await fs.readFile(path!, 'utf8');
    expect(content).toMatch(/^entry_id,date,day_name,exercise_name,set_number,reps,weight_lbs,completed/);
    // At least one data row with completed=true
    expect(content).toContain('true');
  });

  // 9
  test('history CSV has no rows when history is empty', async ({ page }) => {
    await page.locator('.nav-btn').filter({ hasText: 'History' }).click();
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator('button').filter({ hasText: 'Export CSV' }).click(),
    ]);
    const path = await download.path();
    const content = await fs.readFile(path!, 'utf8');
    const lines = content.trim().split('\n');
    // Only the header line, no data rows
    expect(lines).toHaveLength(1);
    expect(lines[0]).toMatch(/^entry_id/);
  });
});
