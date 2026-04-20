import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/playwright',
  timeout: 30_000,
  retries: process.env.CI ? 2 : 0,
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
    command: '/Users/mt/.cargo/bin/trunk serve --no-autoreload',
    url: 'http://localhost:8080',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
