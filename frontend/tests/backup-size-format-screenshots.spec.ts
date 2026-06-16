/**
 * 视觉证据脚本：渲染 formatFileSize 在不同档位的输出
 *
 * 不进入完整 UI，而是用一个临时 HTML 页面把各种档位渲染出来截图，
 * 让 reviewer 一眼看出 M/G 档位不再带 "B" 后缀。
 */

import { test, chromium } from '@playwright/test';

test('截图: formatFileSize 各档位渲染结果', async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 720, height: 480 } });
  const page = await context.newPage();

  // 直接打开 vite 服务并通过页面 evaluate 拿到格式化结果，拼成 HTML
  await page.goto('http://localhost:5173');
  const rows = await page.evaluate(async () => {
    const mod = await import('/src/utils/format');
    const cases: Array<[number, string]> = [
      [500, '500 B'],
      [1024, '1.0 KB'],
      [1536, '1.5 KB'],
      [2.5 * 1024 * 1024, '2.5 M'],
      [100 * 1024 * 1024, '100.0 M'],
      [1.2 * 1024 * 1024 * 1024, '1.2 G'],
      [10 * 1024 * 1024 * 1024, '10.0 G'],
    ];
    return cases.map(([bytes, expected]) => ({
      bytes,
      expected,
      actual: mod.formatFileSize(bytes),
      match: mod.formatFileSize(bytes) === expected,
    }));
  });

  // 拼一个简单的 HTML 表格，把每个 case 的预期值与实际值并排展示
  const html = `
    <!DOCTYPE html>
    <html><head><meta charset="utf-8"><title>issue #595 证据</title>
    <style>
      body { font-family: -apple-system, "Segoe UI", sans-serif; padding: 24px; background: #fafafa; }
      h1 { font-size: 18px; margin: 0 0 16px; }
      table { border-collapse: collapse; width: 100%; background: #fff; }
      th, td { padding: 10px 14px; border-bottom: 1px solid #eee; text-align: left; font-size: 14px; }
      th { background: #f4f4f5; }
      .ok { color: #2e7d32; font-weight: 600; }
      .fail { color: #c62828; font-weight: 600; }
      .meta { color: #666; font-size: 12px; margin-bottom: 16px; }
    </style></head>
    <body>
      <h1>issue #595: formatFileSize 升级（&gt;1M 显示 "XX M"，&gt;1G 显示 "XX G"）</h1>
      <div class="meta">验证日期: ${new Date().toISOString()}</div>
      <table>
        <tr><th>输入字节数</th><th>预期输出</th><th>实际输出</th><th>是否一致</th></tr>
        ${rows.map((r) => `
          <tr>
            <td>${r.bytes.toLocaleString()}</td>
            <td><code>${r.expected}</code></td>
            <td><code>${r.actual}</code></td>
            <td class="${r.match ? 'ok' : 'fail'}">${r.match ? '✓' : '✗'}</td>
          </tr>
        `).join('')}
      </table>
    </body></html>
  `;
  await page.setContent(html);
  await page.screenshot({
    path: '/tmp/issue-595-backup-size-100258/frontend/tests/__screenshots__/backup-size-format.png',
    fullPage: true,
  });
  await browser.close();
});
