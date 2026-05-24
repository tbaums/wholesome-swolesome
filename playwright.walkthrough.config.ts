import { defineConfig, devices } from '@playwright/test';

// Walkthrough config: phone-shaped Chromium runs that generate screenshots
// of full app flows. The dev container can't run WebKit (missing libs), so
// we use Chromium with mobile emulation as a stand-in.
//
// Run all walkthroughs:    npx playwright test --config=playwright.walkthrough.config.ts
// Run one walkthrough:     npx playwright test --config=playwright.walkthrough.config.ts walkthrough_strength
//
// Screenshots land in tests/playwright/screenshots/<spec-name>/ (gitignored).
export default defineConfig({
  testDir: './tests/playwright',
  testMatch: /walkthrough_.+\.spec\.ts/,
  timeout: 180_000,
  retries: 0,
  // Walkthroughs share localStorage + the trunk dev server and write to
  // screenshot dirs — run them sequentially to avoid cross-contamination.
  workers: 1,
  reporter: 'list',
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
    // --no-autoreload is essential: trunk's auto-reload signal will reload
    // the page mid-test (especially during slow fullPage screenshots),
    // wiping in-memory view state and breaking everything after it.
    command: `${process.env.TRUNK ?? 'trunk'} serve --no-autoreload`,
    url: 'http://localhost:8080',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
