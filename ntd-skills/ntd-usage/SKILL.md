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

### 执行管理

```bash
ntd execution resume <id> [--message "prompt"]        # 恢复/继续执行一个已完成的 todo
ntd todo list --status running --output raw            # 查看正在运行的 todo
```

`execution resume` 用于在首次执行完成后，基于已有结果继续补充 prompt 重新执行。常用于分步调试或追加上下文。

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
| `~/.ntd/daemon.log` | 生产日志 |
| `~/.ntd/daemon.pid` | 生产 PID |
