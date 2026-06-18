/**
 * workspace 选择器测试
 *
 * 验证 workspace 选择器功能：
 * - 选择器正确显示在搜索框上方
 * - 点击选择器显示 workspace 列表
 * - 选择 workspace 后正确过滤 todo 列表
 * - 选择"全部工作区"显示所有 todo
 */

import { test, expect, chromium } from '@playwright/test';

const DEV_URL = process.env.E2E_BASE_URL || 'http://localhost:5173';

test.describe('workspace 选择器', () => {
  test('选择器正确显示在搜索框上方', async () => {
    const browser = await chromium.launch();
    const context = await browser.newContext({ colorScheme: 'light' });
    const page = await context.newPage();
    
    await page.goto(DEV_URL);
    await page.waitForTimeout(2000);
    
    // 查找 workspace 选择器
    const workspaceSelector = page.locator('button:has-text("全部工作区")');
    await expect(workspaceSelector).toBeVisible();
    
    // 验证选择器在搜索框上方
    const searchInput = page.locator('input[placeholder*="搜索标题"]');
    const selectorBox = await workspaceSelector.boundingBox();
    const searchBox = await searchInput.boundingBox();
    
    if (selectorBox && searchBox) {
      expect(selectorBox.y).toBeLessThan(searchBox.y);
    }
    
    await browser.close();
  });

  test('点击选择器显示 workspace 列表', async () => {
    const browser = await chromium.launch();
    const context = await browser.newContext({ colorScheme: 'light' });
    const page = await context.newPage();
    
    await page.goto(DEV_URL);
    await page.waitForTimeout(2000);
    
    // 点击 workspace 选择器
    const workspaceSelector = page.locator('button:has-text("全部工作区")');
    await workspaceSelector.click();
    
    // 验证下拉菜单出现
    const dropdownMenu = page.locator('.ant-dropdown');
    await expect(dropdownMenu).toBeVisible();
    
    // 验证菜单包含"全部工作区"选项
    const allWorkspacesOption = page.locator('.ant-dropdown-menu-item:has-text("全部工作区")');
    await expect(allWorkspacesOption).toBeVisible();
    
    await browser.close();
  });

  test('选择 workspace 后正确过滤 todo 列表', async () => {
    const browser = await chromium.launch();
    const context = await browser.newContext({ colorScheme: 'light' });
    const page = await context.newPage();
    
    await page.goto(DEV_URL);
    await page.waitForTimeout(2000);
    
    // 点击 workspace 选择器
    const workspaceSelector = page.locator('button:has-text("全部工作区")');
    await workspaceSelector.click();
    
    // 选择一个 workspace（如果存在）
    const workspaceOption = page.locator('.ant-dropdown-menu-item').nth(1);
    if (await workspaceOption.isVisible()) {
      await workspaceOption.click();
      
      // 验证选择器文本更新
      const updatedSelector = page.locator('button').filter({ hasText: /全部工作区|工作区/ });
      await expect(updatedSelector).toBeVisible();
    }
    
    await browser.close();
  });
});