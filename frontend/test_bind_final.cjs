const { chromium } = require('playwright');

const TUNNEL_URL = 'https://t-b22867a235154e28.hostc.dev';

(async () => {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  page.on('console', msg => {
    if (msg.type() === 'error') {
      console.log('[Console Error]:', msg.text());
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

  // 等待二维码生成
  console.log('5. 等待 5 秒让二维码生成...');
  await page.waitForTimeout(5000);

  // 截图
  await page.screenshot({ path: '/tmp/test_final.png', fullPage: true });

  // 检查 Modal 内容
  const modalBody = await page.locator('.ant-modal-body').innerHTML();
  console.log('Modal body preview:', modalBody.substring(0, 200));

  // 检查二维码
  const qrImg = await page.locator('img[alt="QR Code"]').first();
  const qrVisible = await qrImg.isVisible().catch(() => false);
  console.log('二维码可见性:', qrVisible);

  if (qrVisible) {
    console.log('✅ 二维码已显示！');
  } else {
    console.log('❌ 二维码未显示');
    // 检查是否有错误
    const errorText = await page.locator('.ant-modal-body').textContent();
    console.log('Modal 内容:', errorText?.substring(0, 200));
  }

  await browser.close();
  console.log('\n测试完成');
})();
