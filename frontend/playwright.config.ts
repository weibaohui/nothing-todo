import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '.',
  testMatch: 'e2e-test.spec.ts',
  timeout: 30000,
  use: {
    headless: true,
    baseURL: 'http://localhost:5173',
  },
});