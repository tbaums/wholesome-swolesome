import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/playwright',
  testIgnore: /walkthrough_.+\.spec\.ts/,
  timeout: 30_000,
  retries: process.env.CI ? 2 : 0,
  // Always emit the html report so CI can upload it as a debugging artifact.
  reporter: process.env.CI
    ? [['list'], ['html', { open: 'never' }]]
    : [['list']],
  projects: [
    {
      name: 'iPhone 15',
      use: {
        ...devices['iPhone 15'],
        // Block service worker so stale caches don't interfere
        serviceWorkers: 'block',
      },
    },
  ],
  webServer: {
    command: `${process.env.TRUNK ?? 'trunk'} serve --no-autoreload`,
    url: 'http://localhost:8081',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
