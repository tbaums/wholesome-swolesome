import { defineConfig, devices } from '@playwright/test';

// Local-only Chromium config — dev container can't run WebKit (missing libs).
// Use to repro/iterate on the same specs CI runs on iPhone 15 (WebKit).
//
//   npx playwright test --config=playwright.local-chromium.config.ts
//
// Walkthrough specs (separate, screenshot-producing flows) are excluded —
// run those via playwright.walkthrough.config.ts.
export default defineConfig({
  testDir: './tests/playwright',
  testIgnore: /walkthrough_.+\.spec\.ts/,
  timeout: 30_000,
  retries: 0,
  reporter: [['list']],
  projects: [
    {
      name: 'Pixel-mobile-chromium',
      use: {
        ...devices['Pixel 5'],
        serviceWorkers: 'block',
        baseURL: 'http://localhost:8080',
      },
    },
  ],
  webServer: {
    // --no-autoreload prevents trunk from reloading the page mid-test.
    command: `${process.env.TRUNK ?? 'trunk'} serve --no-autoreload`,
    url: 'http://localhost:8080',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
