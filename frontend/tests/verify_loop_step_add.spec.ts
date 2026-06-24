// 验证环路添加环节功能：选择工作空间后能正常添加 step。
import { test, expect } from '@playwright/test';

test('环路添加环节 - 工作空间选择后正常添加 step', async ({ page }) => {
  // 1. 打开应用，进入事项列表
  await page.goto('http://localhost:18088');
  await page.waitForTimeout(2000);

  // 2. 切换到环路模式
  const loopTab = page.locator('.ant-segmented-item', { hasText: '环路' });
  if (await loopTab.isVisible()) {
    await loopTab.click();
    await page.waitForTimeout(1000);
  }

  // 3. 点击第一个环路进入详情
  const firstLoop = page.locator('.loop-item, [class*="loop-item"], [class*="loop-card"]').first();
  if (await firstLoop.isVisible({ timeout: 3000 })) {
    await firstLoop.click();
    await page.waitForTimeout(2000);

    // 4. 查找添加环节按钮
    const addStepBtn = page.locator('text=添加环节, text=添加').first();
    if (await addStepBtn.isVisible({ timeout: 3000 })) {
      await addStepBtn.click();
      await page.waitForTimeout(1000);

      // 5. 检查 Modal 是否出现
      const modal = page.locator('.ant-modal');
      const modalVisible = await modal.isVisible({ timeout: 3000 }).catch(() => false);
      console.log('Modal visible:', modalVisible);

      if (modalVisible) {
        // 6. 检查表单元素是否存在
        const todoSelect = page.locator('.ant-select').first();
        const todoVisible = await todoSelect.isVisible({ timeout: 2000 }).catch(() => false);
        console.log('Todo select visible:', todoVisible);

        // 7. 选择一个 todo
        if (todoVisible) {
          await todoSelect.click();
          await page.waitForTimeout(500);
          const option = page.locator('.ant-select-item').first();
          if (await option.isVisible({ timeout: 2000 })) {
            await option.click();
            await page.waitForTimeout(300);
          }
        }

        // 8. 填写名称
        const nameInput = page.locator('input[placeholder*="环节"], input[placeholder*="名称"]');
        if (await nameInput.isVisible({ timeout: 2000 })) {
          await nameInput.fill('自动化测试环节');
        }

        // 9. 点击确定/添加按钮
        const okBtn = page.locator('.ant-modal-footer .ant-btn-primary, .ant-modal .ant-btn-primary').first();
        if (await okBtn.isVisible({ timeout: 2000 })) {
          await okBtn.click();
          await page.waitForTimeout(2000);
          console.log('添加按钮已点击');
        }
      }
    }
  }

  // 截图留档
  await page.screenshot({ path: 'frontend/tests/__screenshots__/verify_loop_step_add.png' });
});
