---
name: ntd-usage
description: ntd (Nothing Todo) AI Todo 应用 — CLI & API 使用指南
version: 1.0.0
executors: [claudecode, atomcode, joinai, hermes, codex, codebuddy, opencode, kimi]
---

# ntd (Nothing Todo) 使用指南

ntd 是一个 AI Todo 应用，包含 Rust 后端 + React 前端。
它允许 AI agent 通过 CLI 或 REST API 管理 Todo、tag、executor 和 skill。

## 启动服务

```bash
ntd daemon start          # 启动生产服务（端口 8088）
ntd daemon stop           # 停止
ntd daemon restart        # 重启
ntd daemon status         # 查看状态
```

开发模式：

```bash
make dev                  # 开发模式（端口 18088）
make stop                 # 停止开发实例
```

## CLI 命令参考

所有 CLI 命令通过 `ntd` 二进制执行。

### Todo 管理

```bash
ntd todo list [--status pending|running|completed|failed|archived] [--page 1] [--limit 50] [--search keyword] [--tag-id 1] [--output raw|pretty]
ntd todo get <id> [--output raw|pretty]
ntd todo create <title> [--desc "description"] [--executor claudecode] [--workspace /path] [--worktree] [--tags "1,2"]
ntd todo update <id> [--title "new title"] [--desc "description"] [--status pending]
ntd todo delete <id>
ntd todo archive <id>
ntd todo run <id> [--message "prompt"] [--stdin]         # 远程执行，使用 --stdin 从 stdin 读取内容
```

### Tag 管理

```bash
ntd tag list
ntd tag create <name> [--color "#ff0000"]
ntd tag delete <id>
```

### 统计

```bash
ntd stats
```

### 通用选项

```bash
ntd <command> --output json|pretty|raw    # 输出格式
ntd <command> --fields "id,title,status"  # 字段筛选（仅 raw 模式有效）
```

### 输出解析指南

- `--output raw` — 最简输出，无 `ApiResponse` 包裹，适合 AI 解析
- `--output pretty` — 带颜色高亮，适合人看
- `--output json`（默认）— 带 `ApiResponse` 包裹的标准 JSON

`--fields` 用于精确指定返回字段（如 `id,title,status`），减少 token 消耗。

### 退出码

所有命令成功退出码为 0。错误时打印结构化 JSON 到 stderr 并退出码为 1：

```json
{"error":true,"message":"错误描述"}
```

## REST API 参考

服务器运行后可通过 HTTP 访问以下端点。

### Todo

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/todos` | 列出 todo（支持 `?page=&limit=&status=&search=&tag_id=`） |
| GET | `/api/todos/:id` | 获取单个 todo |
| POST | `/api/todos` | 创建 todo |
| PUT | `/api/todos/:id` | 更新 todo |
| DELETE | `/api/todos/:id` | 删除 todo |
| POST | `/api/todos/:id/run` | 执行 todo |
| POST | `/api/todos/:id/archive` | 归档 todo |

Todo 对象结构：

```json
{
  "id": 1,
  "title": "标题",
  "description": "描述",
  "status": "pending|running|completed|failed|archived",
  "priority": 0,
  "tags": [{"id": 1, "name": "tag名", "color": "#ff0000"}],
  "executor": "claudecode",
  "workspace": "/path",
  "worktree_enabled": false,
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

POST `/api/todos/:id/run` 请求体：

```json
{
  "message": "请帮我完成这个任务",
  "executor": "claudecode",
  "trigger_type": "api"
}
```

### Tag

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/tags` | 列出所有 tag |
| POST | `/api/tags` | 创建 tag（`{"name": "tag名", "color": "#ff0000"}`） |
| DELETE | `/api/tags/:id` | 删除 tag |

### Executor

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/executors` | 列出所有 executor（含配置路径） |
| GET | `/api/executors/:type` | 获取单个 executor 详情 |
| PUT | `/api/executors/:type` | 更新 executor 配置（`{"path": "/usr/bin/claude"}`） |
| GET | `/api/executors/:type/logs` | 获取 executor 日志目录列表 |

### Skill

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/skills` | 发现所有 executor 上的 skill |
| GET | `/api/skills/content?executor=claudecode&name=mcp-servers` | 获取 skill 内容 |
| POST | `/api/skills/sync` | 同步 skill（跨 executor 复制） |
| GET | `/api/skills/compare` | 比较两个 executor 的 skill 差异 |
| POST | `/api/skills/import` | 从 ZIP 导入 skill |
| GET | `/api/skills/:name/export` | 将 skill 导出为 ZIP |
| GET | `/api/skills/invocations` | 查询 skill 调用记录 |
| POST | `/api/skills/record-invocation` | 记录一次 skill 调用 |

支持以下 executor 类型：
`claudecode`, `hermes`, `codex`, `codebuddy`, `opencode`, `atomcode`, `kimi`, `joinai`

### 执行记录

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/execution-records` | 列表（`?todo_id=&status=&page=&limit=`） |
| GET | `/api/execution-records/:id` | 详情 |
| POST | `/api/execution-records/:id/resume` | 恢复执行（`{"message": "继续..."}`） |

### 其他

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/stats` | 全局统计 |
| GET | `/api/global-config` | 全局配置 |
| PUT | `/api/global-config` | 更新全局配置 |
| GET | `/health` | 健康检查 |

## 变量替换

Todo 消息和执行器的 prompt 模板中支持变量替换 `{{变量名}}`：

```
请帮我部署服务，项目目录是 {{project_dir}}，环境是 {{env}}
```

变量通过 API 调用时传入的 `params` 提供。运行时 ntd 会自动替换所有 `{{key}}` 为对应的值。

## 命令规则（Slash Commands）

ntd 支持斜杠命令，用于快速触发特定 todo：

配置方式（`~/.ntd/config.yaml`）：

```yaml
slash_command_rules:
  - slash_command: "/help"
    todo_id: 1
    enabled: true
  - slash_command: "/daily"
    todo_id: 2
    enabled: true
default_response_todo_id: 3  # 未匹配时的默认回复
```

## 模板系统

ntd 支持自定义任务模板，可在创建 todo 时使用。

### 创建模板

```bash
# 通过 API 创建
POST /api/custom-templates
{
  "name": "部署模板",
  "content": "请帮我部署服务 {{service_name}}，环境是 {{env}}",
  "tags": ["deploy"]
}
```

### 使用模板

```bash
ntd todo create "deploy" --desc "使用部署模板" --executor claudecode
```

模板创建时，会检查前置创建的 prompt todo，确保模板内容已被 agent 理解和记忆。

## 常见工作流

### 1. 创建并执行一个任务

```bash
ntd todo create "帮我 review 代码" --executor claudecode --workspace /path/to/project
ntd todo run 1 --message "请 review 当前分支的代码变更"
```

### 2. 使用特定 executor 执行

```bash
ntd todo create "写一篇周报" --executor joinai
ntd todo run 2 --message "请根据我的工作内容写一篇周报"
```

### 3. 带 tag 分类

```bash
ntd tag create "urgent" --color "#ff0000"
ntd tag create "bug"
ntd todo create "修复登录问题" --tags "1,2" --executor claudecode
```

### 4. 查看执行状态

```bash
ntd todo list --status running --output raw --fields "id,title,status"
ntd todo get 1 --output raw
```

### 5. 分步执行（先创建，再填充内容）

```bash
# 创建 todo 但先不执行
ntd todo create "分析日志" --executor claudecode --workspace /var/log

# 稍后用详细 prompt 执行
echo "请分析 /var/log/nginx/access.log 中最近 1 小时的 5xx 错误" | ntd todo run 1 --stdin

# 发现需要更多上下文，可以 resume
ntd execution resume 1 --message "再看看 error.log"
```

### 6. 使用 worktree 模式

```bash
ntd todo create "重构 UserService" --executor claudecode --workspace ~/projects/myapp --worktree
```

`--worktree` 会让 claude_code 和 hermes 以 worktree 模式启动，将当前项目目录作为工作树。

## 常用路径

| 路径 | 说明 |
|------|------|
| `~/.ntd/config.yaml` | 生产环境配置 |
| `~/.ntd/config.dev.yaml` | 开发环境配置（端口 18088） |
| `~/.ntd/data.db` | 生产数据库 |
| `~/.ntd/data.dev.db` | 开发数据库 |
| `~/.ntd/daemon.log` | 生产日志 |
| `backend.dev.log` | 开发日志 |
| `~/.ntd/daemon.pid` | 生产 PID |
| `~/.ntd/dev.pid` | 开发 PID |

## Skill 文件格式

Skill 目录结构：

```
skill-name/
  SKILL.md         # 必需：YAML frontmatter + 内容
  commands.md      # 可选：命令参考
  examples.md      # 可选：示例
  ...
```

`SKILL.md` 必须以 YAML frontmatter 开头（`---` 分隔）：

```yaml
---
name: skill-name
description: 简短描述
version: 1.0.0
executors: [claudecode, atomcode]
---
```

## 翻墙 / 代理设置

如果遇到网络问题（如 prompt 发送失败），请检查代理设置：

```bash
# 查看当前代理
echo $HTTP_PROXY $HTTPS_PROXY $http_proxy $https_proxy
```

对于 AtomCode（基于 deepseek）：

```bash
# 设置代理（BytexMesh）
export ATOMCODE_PROXY=http://127.0.0.1:61272
```

对于 JoinAI：

```bash
# JoinAI 默认使用系统代理，如在终端使用可设置
export https_proxy=http://127.0.0.1:7890
```

> 注意：如果你通过 ngrok 或 cloudflared 暴露本地 ntd 服务，请确保 executor 不需要额外代理访问本地。
