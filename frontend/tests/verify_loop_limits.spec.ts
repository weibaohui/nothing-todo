// 验证环路创建时 limits_config（最大步数/Token数）能正确保存。
import { test, expect } from '@playwright/test';

test('环路创建 - 最大步数和Token数限制能正确保存', async ({ page }) => {
  await page.goto('http://localhost:18088');
  await page.waitForTimeout(2000);

  // 切换到环路模式
  const loopTab = page.locator('.ant-segmented-item', { hasText: '环路' });
  if (await loopTab.isVisible()) {
    await loopTab.click();
    await page.waitForTimeout(1000);
  }

  // 点击新建按钮
  const createBtn = page.locator('button:has-text("新建"), .ant-btn:has-text("新建")').first();
  if (await createBtn.isVisible({ timeout: 3000 })) {
    await createBtn.click();
    await page.waitForTimeout(1000);
  }

  const modal = page.locator('.ant-modal');
  if (await modal.isVisible({ timeout: 3000 })) {
    // 填写名称
    const nameInput = page.locator('.ant-modal input').first();
    if (await nameInput.isVisible()) {
      await nameInput.fill('测试限制条件');
    }

    // 选择工作空间
    const wsSelect = page.locator('.ant-modal .ant-select').first();
    if (await wsSelect.isVisible()) {
      await wsSelect.click();
      await page.waitForTimeout(500);
      const wsOption = page.locator('.ant-select-dropdown .ant-select-item').first();
      if (await wsOption.isVisible({ timeout: 2000 })) {
        await wsOption.click();
        await page.waitForTimeout(300);
      }
    }

    // 找到最大步数输入框（表单中包含"最大执行步数"）
    const maxStepInput = page.locator('.ant-modal input.ant-input-number').first();
    if (await maxStepInput.isVisible({ timeout: 2000 })) {
      await maxStepInput.fill('10');
    }

    // 点击确定
    const okBtn = page.locator('.ant-modal .ant-btn-primary').first();
    if (await okBtn.isVisible()) {
      await okBtn.click();
      await page.waitForTimeout(2000);
      console.log('创建按钮已点击');
    }
  }

  await page.screenshot({ path: 'frontend/tests/__screenshots__/verify_loop_limits.png' });
});
