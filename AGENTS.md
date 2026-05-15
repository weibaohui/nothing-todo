# AGENTS.md

## 项目概述
ntd (Nothing Todo) 是一个 AI Todo 应用，基于 Rust 后端 + React 前端，支持 Codex 和 JoinAI 执行器。

## 开发流程

**禁止直接在主分支 (main) 上写代码。所有代码改动必须先创建分支，在分支上完成开发后再通过 PR 合入 main。**

**每次完成功能开发后，执行 `make restart` 重启服务以便调试。**

## 技术栈
- 后端: Rust (Axum框架)
- 前端: React + Vite + Ant Design
- 数据库: SQLite + SeaORM

## 常用命令

```bash
make install    # 构建并安装
make start     # 启动服务 (需要先 install)
make stop      # 停止服务
make restart   # 重启服务 (开发调试时常用)
make dev       # 开发模式 (前后端分离)
make build     # 仅构建
make clean     # 清理构建产物
```

## 端口
- 前端: 5173 (开发模式)
- 后端: 8088

## 目录结构
- `backend/` - Rust 后端代码
- `frontend/` - React 前端代码
- `tunnel.sh` - 内网穿透脚本

## 前端测试验证

**重要：修改前端 UI 后，必须使用 Playwright 进行自动化验证，再通知用户。**

### Playwright 测试脚本位置
测试脚本位于 `/tmp/` 目录下，文件名格式为 `check_*.js`

**运行方式**：由于 playwright 依赖在 `frontend/node_modules/` 中，需要在 `frontend/` 目录下执行：
```bash
cd frontend && npx playwright test --reporter=list
```

### 验证流程
1. 修改前端代码后，执行 `make restart` 重启服务
2. 使用 Playwright 编写测试脚本验证 UI 效果
3. 验证通过后再通知用户

### 常用验证脚本示例

```javascript
// 验证深色模式组件
const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ colorScheme: 'dark' });
  const page = await context.newPage();

  // 设置 localStorage 以触发 ThemeProvider 的深色模式
  await page.goto('https://t-600b43689eae40d3.hostc.dev');
  await page.evaluate(() => localStorage.setItem('app_theme', 'dark'));
  await page.reload();
  await page.waitForTimeout(2000);

  // 执行验证...
  const result = await page.evaluate(() => {
    const el = document.querySelector('.target-class');
    return { bg: el ? getComputedStyle(el).backgroundColor : null };
  });
  console.log('验证结果:', result);

  await page.screenshot({ path: '/tmp/verify.png' });
  await browser.close();
})();
```

### 内网穿透
如需远程验证，可使用 `tunnel.sh` 启动公网访问：
```bash
./tunnel.sh
```
