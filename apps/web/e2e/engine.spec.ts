// INF-1 end-to-end: in a real Chromium, the UI drives the WASM engine in an AudioWorklet.
import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'English' }).click();
  await page.getByRole('button', { name: 'Engine check' }).click();
  await expect(page.getByText(/browser engine \(AudioWorklet/i)).toBeVisible();
});

test('page is cross-origin isolated so commands use shared memory', async ({ page }) => {
  expect(await page.evaluate(() => self.crossOriginIsolated)).toBe(true);
});

test('holding a chord sounds three voices, releasing lets them end', async ({ page }) => {
  const voices = page.getByTestId('active-voices');
  const c = page.getByRole('button', { name: 'C', exact: true });
  await c.hover();
  await page.mouse.down();
  await expect(voices).toHaveText('3');
  await page.mouse.up();
  await expect(voices).toHaveText('0', { timeout: 5000 });
});

test('self-test measures every note through the bridge', async ({ page }) => {
  await page.getByRole('button', { name: /run self-test/i }).click();
  const pre = page.locator('pre');
  await expect(pre).toContainText('"kind": "worklet"', { timeout: 20_000 });
  const result = JSON.parse((await pre.textContent())!);
  expect(result.loaded).toBe(true);
  expect(result.timeouts).toBe(0);
  expect(result.samples).toBe(30);
});
