# ntd 配置项说明

> 配套文档：[README.md](./README.md) · [ARCHITECTURE.md](./ARCHITECTURE.md) · [SEQUENCE.md](./SEQUENCE.md)

配置文件位置：
- 生产：`~/.ntd/config.yaml`
- 开发（`NTD_MODE=dev`）：`~/.ntd/config.dev.yaml`

首次启动若文件不存在，ntd 会用默认值创建一份 YAML（已归一化路径、clamp 后的超时）。
HTTP `PUT /api/config` 会走同样的归一化 + clamp 流程再原子写回磁盘。

---

## 顶层字段一览

```yaml
# 示例（生产默认）
port: 8088
host: 0.0.0.0
db_path: ~/.ntd/data.db
log_level: INFO

auto_backup_enabled: false
auto_backup_cron: "0 0 3 * * *"
auto_backup_max_files: 30

auto_todo_backup_enabled: false
auto_todo_backup_cron: "0 0 4 * * *"
auto_todo_backup_max_files: 30

auto_sync_custom_templates_enabled: false
auto_sync_custom_templates_cron: "0 0 4 * * *"

auto_skill_backup_enabled: false
auto_skill_backup_cron: "0 0 5 * * *"
auto_skill_backup_max_files: 30

auto_usage_stats_enabled: false
auto_usage_stats_cron: "0 0 1 * * *"

slash_command_rules: []           # 全局斜杠命令路由
default_response_todo_id: null    # 没匹配 slash 时的兜底 todo

history_message_max_age_secs: 600 # 飞书历史消息最大处理年龄
max_concurrent_todos: 3           # 单 todo 并发上限
execution_timeout_secs: 3600      # 单次执行超时，0=不限

auto_cleanup_logs_days: 30        # 日志清理保留天数，null=不清理

scheduler_default_timezone: null  # cron 默认时区（如 "Asia/Shanghai"）

cloud_sync:
  server_url: ""
  sync_token: null
  last_sync_at: null
  default_conflict_mode: "overwrite"
```

---

## 字段详细说明

### 服务

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `port` | u16 | 8088（开发 18088） | HTTP 服务监听端口 |
| `host` | string | `0.0.0.0` | 监听地址；建议生产改成具体 IP 或反代 |
| `db_path` | string | `~/.ntd/data.db` | SQLite 文件路径；`~` 会展开为 `$HOME`；`./relative` 也会展开 |
| `log_level` | string | `INFO` | `RUST_LOG` 覆盖；tracing-subscriber EnvFilter 用 |
| `max_concurrent_todos` | u32 | 3 | 同一 todo 并发执行上限 |
| `execution_timeout_secs` | u64 | 3600 | 单次执行超时（秒）。0 = 不限制。上限 604800（7 天）。YAML 加载时 clamp |

### 备份（数据库 / Todo / Skill）

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `auto_backup_enabled` | bool | false | 是否开启 `~/.ntd/data.db` 周期备份 |
| `auto_backup_cron` | string (6 字段) | `0 0 3 * * *` | 数据库备份 cron（每天凌晨 3 点） |
| `auto_backup_max_files` | usize | 30 | 最多保留的备份文件数，超出按 mtime 删除 |
| `auto_todo_backup_enabled` | bool | false | 是否开启 Todo 列表备份 |
| `auto_todo_backup_cron` | string (6 字段) | `0 0 4 * * *` | Todo 备份 cron（每天凌晨 4 点） |
| `auto_todo_backup_max_files` | usize | 30 | 同上 |
| `auto_skill_backup_enabled` | bool | false | 是否开启 Skill 备份 |
| `auto_skill_backup_cron` | string (6 字段) | `0 0 5 * * *` | Skill 备份 cron（每天凌晨 5 点） |
| `auto_skill_backup_max_files` | usize | 30 | 同上 |

> **Cron 字段**：`秒 分 时 日 月 周`，6 字段。`tokio-cron-scheduler` 要求 6 字段；若时区不是 UTC，会按 `scheduler_default_timezone` 转换。

### 自定义模板 / 统计归档

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `auto_sync_custom_templates_enabled` | bool | false | 是否开启自定义模板（todo_templates 表）远程拉取 |
| `auto_sync_custom_templates_cron` | string (6 字段) | `0 0 4 * * *` | 同步 cron |
| `auto_usage_stats_enabled` | bool | false | 是否开启 AI 使用统计自动归档 |
| `auto_usage_stats_cron` | string (6 字段) | `0 0 1 * * *` | 归档 cron（每天凌晨 1 点） |

### 飞书斜杠命令路由

```yaml
slash_command_rules:
  - slash_command: "/joke"
    todo_id: 8
    enabled: true
  - slash_command: "/review-pr"
    todo_id: 12
    enabled: false
```

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `slash_command` | string | `/todo` | 斜杠命令字符串；匹配时大小写敏感、前缀匹配 |
| `todo_id` | i64 | 0 | 触发后启动的 todo id |
| `enabled` | bool | true | 是否启用；false 时这条规则被跳过 |

| 全局字段 | 类型 | 默认 | 说明 |
|---------|------|------|------|
| `default_response_todo_id` | i64? | null | 当消息不匹配任何 slash_command 时启动的兜底 todo；null = 不兜底 |

### 飞书历史消息

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `history_message_max_age_secs` | u64 | 600（10 分钟） | 历史拉取（`feishu_history_chats` 轮询）时，超出此时间窗口的消息被标记为 `is_history=1` 且不处理 |

### 调度器

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `scheduler_default_timezone` | string? | null | 默认时区（IANA 名，如 `Asia/Shanghai`、`America/New_York`）。todo 自身 `scheduler_timezone` 优先 |

### 日志清理

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `auto_cleanup_logs_days` | usize? | 30 | `execution_logs` 表保留天数；null = 不清理 |

### 云端同步

```yaml
cloud_sync:
  server_url: "https://ntd.example.com"
  sync_token: "ntd_xxxxxxxxxxxx"   # 可选
  last_sync_at: "2026-06-01T00:00:00Z"
  default_conflict_mode: "overwrite" # 或 "skip"
```

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `server_url` | string | `""` | 云端 ntd 服务地址；空 = 不启用 |
| `sync_token` | string? | null | 鉴权 token，`ntd_` 前缀 |
| `last_sync_at` | string? | null | 最后一次成功同步时间（ISO8601）；首次同步时为 null |
| `default_conflict_mode` | string | `overwrite` | 冲突解决模式：`overwrite`（本地覆盖云端）/ `skip`（跳过冲突） |

---

## 历史兼容性字段

### `executors`（已弃用，仅用于一次性迁移）

```yaml
# 旧版本：执行器路径直接写在 config 里
executors:
  paths:
    claudecode: /usr/local/bin/claude
    codex: codex           # 裸命令名（让 $PATH 解析）

# 新版本：执行器存在 DB 的 executors 表里
# 通过 ntd UI / API 配置，配置文件中不再写
```

- ntd 启动时 `migrate_from_config(&cfg.executors)` 会把上面的 `paths` 一次性迁到 `executors` 表。
- 迁移完成后该字段不再读；写回时也不会序列化（`#[serde(skip_serializing)]`）。
- 建议：迁移完成后从 YAML 中删除 `executors` 块。

### 嵌套结构兼容

`ExecutorPaths` 反序列化时同时接受两种 schema：

```yaml
# 新格式
executors:
  paths:
    claudecode: claude

# 旧格式（兼容）
executors:
  claudecode: claude
```

旧格式用户升级 ntd 不会失败；首次保存时统一成新格式。

---

## 配置修改建议

### 时区敏感

- 改 `scheduler_default_timezone` 后，**已注册的 cron job 不会自动重新转换**；需要重启 ntd。
- 改 `max_concurrent_todos` 后，正在执行的 task 不受影响；新发起的执行才生效。

### 备份路径

- 备份文件默认写到 `~/.ntd/backups/`；当前版本暂不开放自定义路径。
- 建议 `auto_backup_max_files` ≥ 7，留一周历史方便回滚。

### 飞书

- `history_message_max_age_secs` 设太大会被飞书 API 限流；建议保持 600。
- `slash_command_rules` 顺序不重要，匹配按消息前缀。

### 升级时注意

- 新增字段：ntd 不会自动写默认值到 YAML，下次保存时才补。可手动加上去。
- 删除字段：保留在 YAML 里也没事，会被 `#[serde(default)]` 忽略。
- 字段重命名：需要写迁移或在 release notes 里明确。

---

## HTTP 动态修改

`PUT /api/config`（handlers/config.rs）允许运行时修改以下字段（白名单，其他字段返回 400）：

- `port` / `host` / `log_level`
- `max_concurrent_todos` / `execution_timeout_secs`
- `auto_backup_*` / `auto_todo_backup_*` / `auto_skill_backup_*`
- `auto_usage_stats_*` / `auto_sync_custom_templates_*`
- `slash_command_rules` / `default_response_todo_id`
- `history_message_max_age_secs`
- `auto_cleanup_logs_days`
- `scheduler_default_timezone`
- `cloud_sync.*`

修改后立即生效（除 cron 任务需重启 scheduler）。

---

## 相关代码

- 配置定义：`backend/src/config.rs::Config`
- 加载 / 保存 / 归一化：`Config::load` / `Config::save` / `normalize_paths` / `clamp_execution_timeout_secs`
- HTTP 修改：`backend/src/handlers/config.rs`
- 周期任务注册：`backend/src/handlers/backup.rs`、`backend/src/handlers/custom_template.rs`
- 运行时校验：`backend/src/handlers/config.rs::validate_execution_timeout_secs`
