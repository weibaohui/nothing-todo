const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ colorScheme: 'dark' });
  const page = await context.newPage();

  await page.goto('https://t-600b43689eae40d3.hostc.dev');
  await page.waitForTimeout(3000);

  const result = await page.evaluate(() => {
    const items = document.querySelectorAll('.todo-item-title');
    return Array.from(items).slice(0, 5).map(el => ({
      text: el.textContent,
      html: el.innerHTML.substring(0, 100)
    }));
  });

  console.log('=== TodoList 标题预览 ===');
  result.forEach(r => console.log('  ', r.text));

  await page.screenshot({ path: '/tmp/todo_id_check.png', fullPage: false });
  console.log('截图已保存到 /tmp/todo_id_check.png');

  await browser.close();
})();
