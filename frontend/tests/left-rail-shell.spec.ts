import { test, expect } from '@playwright/test';

test('左侧主导航渲染并支持切换到设置', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('ntd_left_rail_collapsed', 'true');
  });
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto('/');

  const rail = page.getByTestId('left-rail');
  await expect(rail).toBeVisible();
  await expect(page.getByTestId('left-rail-toggle')).toBeVisible();
  await expect(page.getByTestId('left-rail-label-items')).toHaveCount(0);

  await page.getByTestId('left-rail-toggle').click();
  await expect(page.getByTestId('left-rail-label-items')).toBeVisible();

  await page.getByTestId('left-rail-workspace-switcher').click();
  await page.getByText('管理工作空间').click();
  await expect(page.getByText('添加项目目录')).toBeVisible();
});
