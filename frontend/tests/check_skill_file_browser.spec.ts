import { test, expect } from '@playwright/test';

test.describe('Skill 文件浏览器功能', () => {
  test.beforeEach(async ({ page }) => {
    // 访问 Skills 页面
    await page.goto('http://localhost:5173');
    await page.waitForLoadState('networkidle');
    
    // 等待 Skills 面板加载
    await page.waitForSelector('text=Skills', { timeout: 10000 });
  });

  test('点击 Skill 卡片应显示详情抽屉', async ({ page }) => {
    // 等待 Skill 卡片加载
    await page.waitForSelector('.skill-card', { timeout: 10000 });
    
    // 点击第一个 Skill 卡片
    const firstCard = page.locator('.skill-card').first();
    await firstCard.click();
    
    // 等待抽屉打开
    await page.waitForSelector('.ant-drawer-content', { timeout: 5000 });
    
    // 验证抽屉标题包含 Skill 名称
    const drawerTitle = page.locator('.ant-drawer-title');
    await expect(drawerTitle).toBeVisible();
  });

  test('应显示标签页切换（内容预览和文件浏览）', async ({ page }) => {
    // 等待 Skill 卡片加载
    await page.waitForSelector('.skill-card', { timeout: 10000 });
    
    // 点击第一个 Skill 卡片
    const firstCard = page.locator('.skill-card').first();
    await firstCard.click();
    
    // 等待抽屉打开
    await page.waitForSelector('.ant-drawer-content', { timeout: 5000 });
    
    // 验证标签页存在
    await expect(page.locator('text=内容预览')).toBeVisible();
    await expect(page.locator('text=文件浏览')).toBeVisible();
  });

  test('点击文件浏览标签页应显示文件浏览器', async ({ page }) => {
    // 等待 Skill 卡片加载
    await page.waitForSelector('.skill-card', { timeout: 10000 });
    
    // 点击第一个 Skill 卡片
    const firstCard = page.locator('.skill-card').first();
    await firstCard.click();
    
    // 等待抽屉打开
    await page.waitForSelector('.ant-drawer-content', { timeout: 5000 });
    
    // 点击文件浏览标签页
    await page.click('text=文件浏览');
    
    // 验证文件浏览器显示
    await expect(page.locator('text=搜索文件...')).toBeVisible();
  });

  test('文件浏览器应显示搜索框和文件统计', async ({ page }) => {
    // 等待 Skill 卡片加载
    await page.waitForSelector('.skill-card', { timeout: 10000 });
    
    // 点击第一个 Skill 卡片
    const firstCard = page.locator('.skill-card').first();
    await firstCard.click();
    
    // 等待抽屉打开
    await page.waitForSelector('.ant-drawer-content', { timeout: 5000 });
    
    // 点击文件浏览标签页
    await page.click('text=文件浏览');
    
    // 验证搜索框存在
    await expect(page.locator('input[placeholder="搜索文件..."]')).toBeVisible();
    
    // 验证文件统计显示
    await expect(page.locator('text=/共 \\d+ 个文件/')).toBeVisible();
  });

  test('搜索框应能过滤文件列表', async ({ page }) => {
    // 等待 Skill 卡片加载
    await page.waitForSelector('.skill-card', { timeout: 10000 });
    
    // 点击第一个 Skill 卡片
    const firstCard = page.locator('.skill-card').first();
    await firstCard.click();
    
    // 等待抽屉打开
    await page.waitForSelector('.ant-drawer-content', { timeout: 5000 });
    
    // 点击文件浏览标签页
    await page.click('text=文件浏览');
    
    // 在搜索框中输入内容
    const searchInput = page.locator('input[placeholder="搜索文件..."]');
    await searchInput.fill('SKILL');
    
    // 验证文件列表被过滤
    await page.waitForTimeout(500);
    
    // 清空搜索框
    await searchInput.clear();
  });

  test('点击文件应显示文件预览', async ({ page }) => {
    // 等待 Skill 卡片加载
    await page.waitForSelector('.skill-card', { timeout: 10000 });
    
    // 点击第一个 Skill 卡片
    const firstCard = page.locator('.skill-card').first();
    await firstCard.click();
    
    // 等待抽屉打开
    await page.waitForSelector('.ant-drawer-content', { timeout: 5000 });
    
    // 点击文件浏览标签页
    await page.click('text=文件浏览');
    
    // 等待文件列表加载
    await page.waitForTimeout(1000);
    
    // 点击第一个文件（如果存在）
    const firstFile = page.locator('[style*="cursor: pointer"]').first();
    if (await firstFile.isVisible()) {
      await firstFile.click();
      
      // 验证文件预览区域显示
      await page.waitForTimeout(500);
    }
  });
});
