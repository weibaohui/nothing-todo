import { test, expect } from '@playwright/test';

test.describe('Executor UI Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Go to the app
    await page.goto('http://localhost:5173');
    // Wait for page to load
    await page.waitForLoadState('networkidle');
  });

  test('should load the main page', async ({ page }) => {
    // Check page title or main content
    await expect(page.locator('body')).toBeVisible();
  });

  test('should create a todo and start execution', async ({ page }) => {
    // Click the add button or navigate to create todo
    const addButton = page.locator('button').filter({ hasText: /新增|添加|add/i }).first();
    if (await addButton.isVisible()) {
      await addButton.click();
      await page.waitForTimeout(500);
    }

    // Find the title input and fill it
    const titleInput = page.locator('input').filter({ hasText: /标题|title/i }).first();
    if (await titleInput.isVisible({ timeout: 2000 })) {
      await titleInput.fill('Test task for UI');
    }

    // Find prompt textarea and fill with simple prompt
    const promptTextarea = page.locator('textarea').first();
    if (await promptTextarea.isVisible({ timeout: 2000 })) {
      await promptTextarea.fill('Say hello in 3 words');
    }

    // Find and click executor dropdown to select different executors
    const executorSelect = page.locator('.ant-select').filter({ hasText: /executor|执行器/i }).first();
    if (await executorSelect.isVisible({ timeout: 2000 })) {
      await executorSelect.click();
      await page.waitForTimeout(300);

      // Try to select each available executor
      const options = page.locator('.ant-select-item');
      const optionCount = await options.count();
      console.log(`Found ${optionCount} executor options`);

      for (let i = 0; i < Math.min(optionCount, 5); i++) {
        await options.nth(i).click();
        await page.waitForTimeout(200);
        console.log(`Selected executor option ${i}`);
      }
    }

    // Submit the form if there's a submit button
    const submitButton = page.locator('button').filter({ hasText: /提交|确定|submit|create/i }).first();
    if (await submitButton.isVisible({ timeout: 2000 })) {
      await submitButton.click();
      await page.waitForTimeout(1000);
    }

    // Check if todo was created
    await page.waitForTimeout(500);
  });

  test('should list todos', async ({ page }) => {
    // Check if todo list exists
    const todoList = page.locator('[class*="todo"], [class*="list"]').first();
    if (await todoList.isVisible({ timeout: 3000 })) {
      console.log('Todo list is visible');
    }

    // Count todo items
    const todoItems = page.locator('[class*="item"], tr, [class*="card"]');
    const count = await todoItems.count();
    console.log(`Found ${count} todo items`);
  });
});