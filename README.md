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

## 核心痛点

传统 Todo 工具只能**记录**任务，却无法**执行**任务。用户日常的代码任务、运维操作等，往往需要：

1. 在 Todo 应用中写下任务描述
2. 手动打开终端或 AI CLI 工具（如 Claude Code、AtomCode 等）
3. 逐字复制任务描述，等待执行结果
4. 再回到 Todo 标记完成

**ntd 的核心价值**：将任务记录与 AI 执行无缝打通，让一个 Todo 从创建到完成全部在同一个平台上闭环。AI 执行器直接读取你的任务描述并自动执行，无需人工干预——真正"无事可做"。

核心解决三个问题：

| 痛点 | 传统 Todo 工具 | ntd |
|------|--------------|-----|
| 任务执行 | 纯文字记录，需手动执行 | 一键触发 AI 执行器自动完成 |
| 进度跟踪 | 用户自行标记状态 | 实时推送执行日志，状态自动更新 |
| 批量调度 | 不支持 | 内置 Cron 定时调度器，自动周期性执行 |

---

## 核心逻辑流

ntd 是一个**单 Agent 调度系统**，不包含多 Agent 协作或长链推理编排。其核心架构如下：

```
用户创建 Todo → 触发执行 → ExecutorRegistry 选择执行器
                                        → 启动 AI CLI 子进程
                                        → 通过 broadcast channel 实时推送进度
                                        → Cron 调度器定时触发（可选）
```

| 组件 | 职责 |
|------|------|
| **ExecutorRegistry** | 适配器模式，统一管理多种 AI CLI 工具（Claude Code、JoinAI、CodeBuddy 等），每种工具实现 `CodeExecutor` trait |
| **Executor Service** | 单 Todo 执行器：启动 AI CLI 子进程，分离 stdout/stderr，解析日志，处理取消与异常终止 |
| **TaskManager** | 基于 tokio `mpsc` 通道的任务生命周期管理，支持取消信号广播 |
| **Scheduler** | 基于 `tokio-cron-scheduler` 的定时调度器，支持 Cron 表达式，服务启动时从 DB 加载 |
| **事件总线** | `broadcast::channel<ExecEvent>` 实时推送 Started / Output / Finished 事件到前端 |

**关键设计决策：**

- **单执行器单任务**：每个 Todo 在同一时刻只由一个 AI 执行器处理，不支持并行多 Agent 协作
- **子进程隔离**：Unix 环境下通过 `setpgid` 创建独立进程组，取消时级联杀死子进程树，防止僵尸进程
- **孤儿清理**：程序崩溃后自动清理状态为 `running` 但无 `task_id` 的残留执行记录

**执行流程详解：**

```
1. 用户通过前端创建/触发 Todo
   ↓
2. Backend 接收请求，确定执行器（优先级：请求指定 > Todo 存储 > 默认 JoinAI）
   ↓
3. 创建 ExecutionRecord，Todo 状态更新为 running，关联 task_id
   ↓
4. 启动 AI CLI 子进程（通过 Command::new 执行可执行文件）
   ↓
5. 异步读取 stdout/stderr，解析为结构化日志，经 broadcast channel 推送前端
   ↓
6. 子进程结束后，更新 ExecutionRecord（状态、日志 JSON、使用量、结果）
   ↓
7. Todo 状态更新为 completed/failed，task_id 解绑
```

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
| JoinAI| AI 工作流工具 | `adapters/joinai.rs` |
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
