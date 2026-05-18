import { chromium } from 'playwright';
(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ colorScheme: 'dark' });
  const page = await context.newPage();

  await page.goto('https://t-600b43689eae40d3.hostc.dev');
  await page.waitForTimeout(3000);

  const result = await page.evaluate(() => {
    // 尝试多种选择器
    const selectors = ['.todo-item-title', '[class*="todo-item"]', '[class*="title"]'];
    for (const sel of selectors) {
      const items = document.querySelectorAll(sel);
      if (items.length > 0) {
        return { selector: sel, items: Array.from(items).slice(0, 3).map(el => el.textContent?.trim()) };
      }
    }
    // 打印 body 下前几个 div 的文本
    const bodyText = document.body.innerText?.substring(0, 500);
    return { bodyText };
  });

  console.log('结果:', JSON.stringify(result, null, 2));
  await page.screenshot({ path: '/tmp/todo_id_check.png', fullPage: false });
  console.log('截图已保存到 /tmp/todo_id_check.png');

  await browser.close();
})();
