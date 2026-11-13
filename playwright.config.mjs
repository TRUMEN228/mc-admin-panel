import { defineConfig } from '@playwright/test';

// Локально — установленный Microsoft Edge; в CI — Chromium из Playwright.
export default defineConfig({
  testDir: 'tests',
  timeout: 30_000,
  reporter: process.env.CI ? 'github' : 'list',
  use: { channel: process.env.CI ? undefined : 'msedge', viewport: { width: 1440, height: 900 }, locale: 'ru-RU' },
});
