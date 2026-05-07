const { chromium } = require('playwright');

const TUNNEL_URL = 'https://t-c0263bdb63e24cda.hostc.dev';

(async () => {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  page.on('console', msg => {
    console.log(`[Console ${msg.type()}]:`, msg.text());
  });

  page.on('pageerror', err => {
    console.log('[Page Error]:', err.message);
  });

  // 监听 API 请求
  page.on('response', res => {
    if (res.url().includes('feishu') || res.url().includes('agent-bots')) {
      console.log('API Response:', res.status(), res.url());
    }
  });

  console.log('1. 打开首页...');
  await page.goto(TUNNEL_URL, { waitUntil: 'networkidle' });
  await page.waitForTimeout(2000);

  console.log('2. 点击设置...');
  const configBtn = await page.locator('[aria-label="配置管理"]').first();
  await configBtn.click();
  await page.waitForTimeout(2000);

  console.log('3. 点击消息 Tab...');
  const messageTab = await page.locator('.ant-tabs-tab').filter({ hasText: '消息' }).first();
  await messageTab.click();
  await page.waitForTimeout(2000);

  console.log('4. 点击绑定按钮...');
  const bindBtn = await page.locator('button:has-text("绑定飞书智能体")').first();
  await bindBtn.click();

  // 等待一段时间观察
  console.log('5. 等待 5 秒...');
  await page.waitForTimeout(5000);

  // 截图
  await page.screenshot({ path: '/tmp/test_debug.png', fullPage: true });

  // 检查 Modal 内容
  const modalBody = await page.locator('.ant-modal-body').innerHTML();
  console.log('Modal body HTML:', modalBody.substring(0, 300));

  await browser.close();
  console.log('\n测试完成');
})();
