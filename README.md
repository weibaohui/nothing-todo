# ntd — Nothing Todo

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-19-blue.svg)](https://react.dev)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**ntd** (Nothing Todo) 是一个 AI 驱动的 Todo 任务管理应用。它将传统的待办事项管理与多 AI 执行器深度集成，让你的任务不仅能被记录，还能被自动执行。

> "无事可做" — 因为 AI 已经帮你做完了。

---

## 特性

- **智能任务管理** — 创建、编辑、跟踪 Todo，支持多种状态（待办、进行中、已完成、已取消、已归档）
- **多 AI 执行器支持** — 集成 Claude Code、JoinAI、CodeBuddy、OpenCode、AtomCode 等多种 AI CLI 工具
- **可视化仪表盘** — 实时统计任务完成情况，支持趋势图表和数据洞察
- **标签系统** — 灵活的标签分类，快速筛选和定位任务
- **定时调度** — 内置 Cron 调度器，支持定时触发任务执行
- **嵌入式前端** — 生产环境前端资源嵌入 Rust 二进制，单文件部署
- **跨平台** — 支持 Windows、macOS、Linux（x86_64 & ARM64）

---
![detail](docs/detail.png)
![dashboard](docs/dashboard.png)

---
## 快速开始

### 前置要求

- [Rust](https://www.rust-lang.org/tools/install) 1.85+
- [Node.js](https://nodejs.org/) 20+
- [Make](https://www.gnu.org/software/make/)

### 一键安装

```bash
# 安装所有依赖并编译
make setup

# 构建并安装到 ~/.local/bin/ntd
make install
```


### 启动服务

```bash
# 启动后台服务（端口 8088）
ntd

# 打开浏览器访问 http://localhost:8088
```

### 开发模式（推荐）

```bash
# 同时启动前端开发服务器（5173）和后端（8088）
make dev

# 或热重载模式
make watch
```
 
---
 

## 支持的 AI 执行器

| 执行器 | 说明 | 适配器文件 |
|--------|------|-----------|
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) | Anthropic 官方 CLI | `adapters/claude_code.rs` |
| [JoinAI](https://github.com/xyflow/ai) | AI 工作流工具 | `adapters/joinai.rs` |
| CodeBuddy | 代码助手 | `adapters/codebuddy.rs` |
| OpenCode | 开源代码助手 | `adapters/opencode.rs` |
| AtomCode | Atom 编辑器插件 | `adapters/atomcode.rs` |

---
 
## 许可证

[MIT](LICENSE)

---

<p align="center">
  用 Rust + React + AI 打造 | 让待办事项真正被「执行」
</p>
