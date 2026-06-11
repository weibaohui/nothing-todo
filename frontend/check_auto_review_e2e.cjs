// Playwright 端到端验证: 评审结果展示在 UI 中
const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ colorScheme: 'light', viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const logs = [];
  page.on('console', msg => { if (msg.type() === 'error') logs.push(`[console.error] ${msg.text()}`); });
  page.on('pageerror', err => logs.push(`[pageerror] ${err.message}`));

  const results = { pass: 0, fail: 0, items: [] };
  const record = (name, ok, detail = '') => {
    results.items.push({ name, ok, detail });
    if (ok) results.pass++; else results.fail++;
    console.log(`${ok ? '✅' : '❌'} ${name}${detail ? '  ' + detail : ''}`);
  };

  try {
    console.log('\n=== 1) 打开 dev 实例 ===');
    await page.goto('http://localhost:18088', { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2500);

    // Phase A: 列表应显示 [评审] 实例徽章
    console.log('\n=== 2) 列表: 评审实例 todo ===');
    const reviewBadges = await page.locator('text=[评审]').filter({ hasNotText: '模板' }).count();
    record('列表显示 [评审] 实例 todo 徽章', reviewBadges > 0, `(共 ${reviewBadges} 个)`);
    await page.screenshot({ path: '/tmp/auto_review_e2e_1_list.png', fullPage: false });

    // Phase B: 打开评审实例 todo #14
    console.log('\n=== 3) 打开评审实例 todo ===');
    const reviewCard = page.locator('.todo-item:has-text("评审")').filter({ hasNotText: '模板' }).first();
    if (await reviewCard.count() > 0) {
      await reviewCard.click();
      await page.waitForTimeout(2500);
      await page.screenshot({ path: '/tmp/auto_review_e2e_2_review_detail.png', fullPage: false });

      // 通过 API 验证评审实例 result 中包含 RATING
      const reviewResult = await page.evaluate(async () => {
        const r = await fetch('/api/execution-records?todo_id=14&limit=1', { credentials: 'include' });
        const j = await r.json();
        return j?.data?.records?.[0]?.result;
      });
      const hasRating = /RATING\s*:?\s*\d+/i.test(reviewResult || '');
      record('评审实例 record.result 包含 RATING:N', hasRating, `(result=${(reviewResult || '').slice(0, 80)}...)`);
    } else {
      record('评审实例 todo 详情可打开', false, '找不到 [评审] 卡片');
    }

    // Phase C: 打开原 todo #13
    console.log('\n=== 4) 打开原 todo 看评分+评审状态 ===');
    const sourceCard = page.locator('.todo-item:has-text("[评审测试]")').first();
    if (await sourceCard.count() > 0) {
      await sourceCard.click();
      await page.waitForTimeout(2500);
      await page.screenshot({ path: '/tmp/auto_review_e2e_3_source_detail.png', fullPage: false });

      // 评分应显示 100
      const ratingBadge = await page.locator('button[aria-label*="已评分"]').first();
      const ratingVisible = await ratingBadge.count() > 0;
      record('原 todo 执行记录显示评分 100', ratingVisible);
      if (ratingVisible) {
        const ratingText = await ratingBadge.textContent();
        record('评分数值正确', ratingText.includes('100'), `(text=${ratingText})`);
      }

      // 评审状态徽章应显示 "✅ 评审成功"
      const reviewSuccess = await page.locator('text=评审成功').count();
      record('原 record 显示"评审成功"徽章', reviewSuccess > 0);
    } else {
      record('原 todo 详情可打开', false, '找不到 [评审测试] 卡片');
    }

    console.log('\n=== 控制台错误 ===');
    if (logs.length === 0) console.log('   无');
    for (const log of logs.slice(0, 10)) console.log(log);

    console.log('\n=== 汇总 ===');
    console.log(`通过 ${results.pass} / 失败 ${results.fail}`);
  } catch (err) {
    console.error('验证出错:', err.message);
    await page.screenshot({ path: '/tmp/auto_review_e2e_error.png' }).catch(() => {});
  } finally {
    await browser.close();
    process.exit(results.fail > 0 ? 1 : 0);
  }
})();
