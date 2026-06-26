import { test, expect } from '@playwright/test';

test('左侧主导航渲染并支持切换到设置', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto('/');

  await expect(page.getByTestId('left-rail')).toBeVisible();

  await page.getByTestId('left-rail-settings').click();
  await expect(page.getByText('系统设置')).toBeVisible();
});

