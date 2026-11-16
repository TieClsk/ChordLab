import { defineConfig, devices } from '@playwright/test';

// Locally a Chromium binary can be supplied with PW_CHROMIUM_EXECUTABLE; CI uses the
// browser installed by `playwright install chromium`.
const executablePath = process.env.PW_CHROMIUM_EXECUTABLE;

export default defineConfig({
  testDir: 'e2e',
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: 'http://localhost:4173',
    ...devices['Desktop Chrome'],
    launchOptions: {
      executablePath,
      args: ['--autoplay-policy=no-user-gesture-required', ...(executablePath ? ['--no-sandbox'] : [])],
    },
  },
  webServer: {
    command: 'pnpm preview --port 4173 --strictPort',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
  },
});
