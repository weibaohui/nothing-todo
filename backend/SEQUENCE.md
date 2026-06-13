# ntd Backend 关键流程时序图

> 配套文档：[README.md](./README.md) · [ARCHITECTURE.md](./ARCHITECTURE.md) · [CONFIG.md](./CONFIG.md)

本文档用 mermaid `sequenceDiagram` 描述 5 个最关键的业务流程。每个图都标注了：
- 涉及模块
- 关键边界（超时 / 并发上限 / 取消 / 失败兜底）
- 与其他流程的衔接点

---

## 1. 执行 Todo 的端到端流程

**触发方式**：HTTP `POST /api/execute` 或 CLI `ntd todo execute`，亦可由 cron / webhook / 飞书 / hook 派生。

```mermaid
sequenceDiagram
    autonumber
    actor Client as 前端 / CLI
    participant H as handlers/execution.rs<br/>start_todo_execution
    participant ES as executor_service<br/>run_todo_execution
    participant TM as task_manager<br/>TaskManager
    participant REG as adapters<br/>ExecutorRegistry
    participant DB as db::Database
    participant TX as broadcast::Sender<br/>ExecEvent
    participant CG as command-group<br/>AsyncGroupChild
    participant Hook as hooks<br/>HookService
    participant WS as WebSocket<br/>前端

    Client->>H: POST /api/execute {todo_id, message, executor?}
    H->>ES: RunTodoExecutionRequest { ... }

    ES->>TM: register(task_id)
    TM-->>ES: cancel_rx (mpsc::Receiver)

    ES->>DB: get_todo(todo_id)
    DB-->>ES: Todo { executor?, ... }

    Note over ES: 并发上限检查<br/>count(running records for todo_id)<br/>&lt; max_concurrent_todos

    ES->>REG: get_or_default(executor_req | todo.executor | default)
    REG-->>ES: Arc<dyn CodeExecutor>

    ES->>DB: create_execution_record(status=Running)
    DB-->>ES: record_id

    ES->>TX: send(Started {task_id, todo_id, executor})
    TX-->>WS: 推前端

    par 子进程 + 日志解析
        ES->>CG: spawn(cli binary + args)
        CG-->>ES: child (pgid set)

        loop 每行 stdout/stderr
            CG-->>ES: BufReader line
            ES->>ES: parse → ParsedLogEntry
            ES->>DB: append execution_log
            ES->>TX: send(Output {entry})
            TX-->>WS: 推前端
        end

        ES->>CG: wait_with_output + timeout(execution_timeout_secs)
    and 取消监听
        TM->>ES: cancel_rx.recv() (用户取消)
        ES->>CG: kill() (整个进程组)
    end

    alt 正常完成
        CG-->>ES: ExitStatus::success()
        ES->>DB: update_execution_record(status=Success)
        ES->>TX: send(Finished {success=true})
    else 超时
        Note over ES: tokio timeout 触发
        ES->>CG: kill()
        ES->>DB: update_execution_record(status=Timeout)
        ES->>TX: send(Finished {success=false, "timeout"})
    else 取消
        ES->>DB: update_execution_record(status=Cancelled)
        ES->>TX: send(Finished {success=false, "cancelled by user"})
    else 进程失败
        CG-->>ES: ExitStatus::failure / spawn error
        ES->>DB: update_execution_record(status=Failed, stderr)
        ES->>TX: send(Finished {success=false})
    end

    ES->>Hook: fire_for_todo(todo_id, Finished)
    Note over Hook: 异步 tokio::spawn，<br/>读 parent.hooks 触发子 todo

    ES->>TM: remove(task_id)
    TX-->>WS: Finished 事件
```

**关键边界**：
- 并发上限检查用 `task_manager.get_all_task_infos()` 过滤掉僵尸记录（status=running 但 task_manager 不存在）。
- 取消信号通过 `mpsc` 通道送达；不要在阻塞 future 中 `recv`，要 `tokio::select!` 与子进程等待并列。
- 失败兜底：进程崩溃 / spawn 失败 / BufReader EOF 都要把 status 写到 DB；否则前端永远看到 Running。

---

## 2. Cron 调度工作流程

**触发方式**：`make dev` / `make install` 后由 ntd daemon 启动 ntd 服务，scheduler 随之初始化。

```mermaid
sequenceDiagram
    autonumber
    participant Main as main.rs::run_server
    participant Cfg as config::Config
    participant Sched as scheduler<br/>TodoScheduler
    participant DB as db::Database
    participant TZ as chrono_tz
    participant JSC as tokio_cron_scheduler<br/>JobScheduler
    participant ES as executor_service
    participant Hook as hooks<br/>HookService

    Main->>Cfg: Config::load() (含 scheduler_default_timezone)
    Cfg-->>Main: Config

    Main->>Sched: TodoScheduler::new()
    Main->>Sched: load_from_db(&ctx)

    loop 每个 todo WHERE scheduler_enabled=true
        Sched->>DB: get_todo(id)
        DB-->>Sched: Todo { scheduler_config, scheduler_timezone }
        Sched->>TZ: parse tz (e.g. "Asia/Shanghai")
        Sched->>Sched: convert_cron_to_utc(cron, tz)
        Note over Sched: 把用户本地时间转换为 UTC<br/>因为 tokio-cron-scheduler 总是 UTC
        Sched->>JSC: add(Job::new_async(cron_utc, callback))
    end

    Main->>Sched: start()
    Sched->>JSC: start()

    loop 每个 tick
        JSC->>Sched: 回调 fn(todo_id, cron)
        Sched->>ES: run_todo_execution(trigger_type="cron")
        ES->>Hook: fire_for_todo (Finished 后)
    end

    Note over Main: 周期任务：<br/>auto_backup / auto_todo_backup /<br/>auto_skill_backup / auto_usage_stats /<br/>auto_sync_custom_templates<br/>也走相同 JobScheduler 路径
```

**关键边界**：
- `convert_cron_to_utc` 仅处理 hours 字段；`*` / 单值 / `9-17` / `*/2` 中除 `*/2` 外都正确换算。
- 调度器重启时 JobScheduler 不会保留跨进程状态，必须重新从 DB 加载。
- `tz=None` 视为 UTC。

---

## 3. 飞书消息处理流程

**触发方式**：某个 `agent_bots.app_id` 配置开启后，ntd 启动时建立 WebSocket 长连接。

```mermaid
sequenceDiagram
    autonumber
    participant FS as Feishu Server
    participant Ch as feishu/channel.rs<br/>FeishuChannelService
    participant L as services/feishu_listener
    participant TM as TokenManager
    participant DB as db::Database
    participant Bind as feishu_project_bindings
    participant Debounce as MessageDebounce
    participant ES as executor_service
    participant TX as broadcast::Sender

    Note over Ch: 启动时为每个 bot_id<br/>创建独立 WS 连接

    loop 心跳 / 重连
        FS->>Ch: ping / pong
    end

    FS->>Ch: 消息事件 (message_id, chat_id, sender, content)
    Ch->>Ch: codec 解码 → ChannelMessage
    Ch->>L: dispatch

    L->>TM: get_token(bot_id)
    TM-->>L: access_token

    L->>DB: feishu_messages::insert(bot_id, msg)
    Note over L: message_id UK，幂等

    L->>Bind: get_by_chat_id(bot_id, chat_id)
    alt 命中绑定
        Bind-->>L: project_dir + resume_session_id?
        L->>L: 解析 slash command 或 default_response_todo
    else 未绑定
        L->>L: 走 slash command / default_response
    end

    L->>Debounce: push(PendingMessage {bot_id, chat_id, ...})
    Note over Debounce: 同 (bot_id, chat_id) 已存在 →<br/>abort 旧 timer + 合并<br/>否则注册新 timer (debounce_secs)

    Note over Debounce: 等 debounce_secs 或新消息打断

    Debounce->>ES: run_todo_execution(trigger_type="feishu",<br/>resume_session_id?, resume_message?,<br/>binding_id?)
    ES->>DB: create_execution_record
    ES->>TX: send(Started)

    Note over ES: 正常执行链路<br/>(见 §1)

    ES->>TX: send(Finished)
    TX-->>L: 收到 Finished
    L->>FS: 调 send_message API 回复结果<br/>(绑定 chat_id)
    L->>DB: update feishu_messages.processed=1,<br/>processed_todo_id, execution_record_id
```

**关键边界**：
- 同 `(bot_id, chat_id)` 短时间内多消息只触发一次执行；合并逻辑保留最后一条 + 历史摘要。
- `feishu_messages.message_id` 是 unique，重复投递走幂等分支不重复执行。
- `history_message_max_age_secs`（默认 600）控制历史回溯多久的消息还会被处理；超过的标 `is_history=1` 跳过。
- Token 由 `TokenManager` 缓存，过期前自动刷新；冷启动时同步获取。

---

## 4. 配置加载流程

**触发方式**：每次 `ntd` 启动时 `Config::load()`。

```mermaid
sequenceDiagram
    autonumber
    participant Main as main.rs::run_server
    participant Cfg as config::Config
    participant FS as std::fs
    participant Yaml as serde_yaml
    participant Env as std::env

    Main->>Cfg: Config::load()
    Cfg->>Env: NTD_MODE ?
    alt NTD_MODE=dev
        Cfg->>FS: path = ~/.ntd/config.dev.yaml
    else 生产
        Cfg->>FS: path = ~/.ntd/config.yaml
    end

    alt 文件不存在
        Cfg->>Cfg: Config::default() + dev 覆盖 (port, db_path)
        Cfg->>Cfg: normalize_paths()
        Cfg->>Cfg: clamp_execution_timeout_secs()
        Cfg->>FS: create_dir_all(parent)
        Cfg->>Yaml: to_string(&cfg)
        Cfg->>FS: write(path, yaml)
        Cfg-->>Main: cfg (in-memory, 已 normalize)
    else 文件存在
        Cfg->>FS: read_to_string(path)
        FS-->>Cfg: yaml content
        Cfg->>Yaml: from_str::<Config>(content)
        alt 解析失败
            Cfg->>Cfg: warn + Config::default()
        else 解析成功
            Cfg->>Cfg: cfg (含 ExecutorPaths 反序列化兜底)
        end
        Cfg->>Cfg: normalize_paths()
        Note over Cfg: db_path: ~ → home 展开<br/>executors.paths: ~ / 相对路径展开<br/>裸命令名（无 /）原样保留
        Cfg->>Cfg: clamp_execution_timeout_secs()
        Note over Cfg: 0 保留<br/>1-59 → 60<br/>> MAX_EXECUTION_TIMEOUT_SECS → MAX
        Cfg-->>Main: cfg
    end

    Note over Main: HTTP PUT /api/config → 同样的<br/>normalize + clamp 流程（去重避免跳过兜底）
```

**关键边界**：
- 用户直接编辑 `~/.ntd/config.yaml` 时 `clamp_execution_timeout_secs` 是唯一兜底（HTTP 路径用 `validate_execution_timeout_secs`）。
- `ExecutorPaths` 反序列化兼容两种 schema：`{paths: {...}}` 和老版本直接 `{...}`，避免历史配置文件升级失败。
- 配置文件写入用 `temp + rename` 原子替换；崩溃时不会留下半截 YAML。

---

## 5. 数据库初始化流程

**触发方式**：`Database::new(path)`。

```mermaid
sequenceDiagram
    autonumber
    participant Main as main.rs::run_server
    participant DB as db::Database
    participant SO as sea_orm<br/>ConnectOptions
    participant SQL as SQLite
    participant Migrate as db::Database<br/>(migrations)

    Main->>DB: Database::new(db_path)
    DB->>SO: ConnectOptions::new("sqlite://...?mode=rwc")
    SO->>SO: max_connections(10)<br/>min_connections(1)<br/>connect_timeout(5s)<br/>sqlx_logging(false)
    DB->>SQL: connect()

    DB->>SQL: PRAGMA busy_timeout=5000
    DB->>SQL: PRAGMA foreign_keys=ON
    DB->>SQL: PRAGMA journal_mode=WAL
    DB->>SQL: query("SELECT journal_mode")
    SQL-->>DB: mode (期望 "wal")

    DB->>SQL: init_tables()
    Note over DB,SQL: 遍历 entity/*.rs<br/>CREATE TABLE IF NOT EXISTS

    DB->>DB: seed_default_templates()
    Note over DB: 内置 todo_templates 写入

    DB->>DB: migrate_feishu_fk_cascade()
    Note over DB: 检查 sqlite_master.sql<br/>缺 ON DELETE CASCADE 的表<br/>→ 重建 (新表→copy→drop→rename)

    DB->>DB: migrate_logs_to_execution_logs()
    Note over DB: 老 logs 表数据迁到 execution_logs

    DB->>DB: migrate_from_config(&cfg.executors)
    Note over DB: 一次迁移：<br/>config.yaml.executors.paths →<br/>executors 表

    DB->>DB: seed_default_executors()
    Note over DB: 表为空时写入 11 个内置执行器

    DB->>DB: sync_new_executors()
    Note over DB: 增量：代码 EXECUTORS 常量 vs DB<br/>补新增、删内置已移除

    DB->>DB: backfill_session_dir()
    Note over DB: 给历史 executors 补 session_dir

    DB->>DB: cleanup_orphan_execution_records()
    Note over DB: status='running' 但进程已退出<br/>→ status='failed' + finished_at=now

    DB->>DB: cleanup_old_webhook_records(30 days)
    DB-->>Main: Database ready
```

**关键边界**：
- `PRAGMA foreign_keys=ON` 必须显式开，SQLite 默认 OFF。
- WAL 模式下允许多读单写；`max_connections=10` 是 SQLite + SeaORM 的安全上限。
- `migrate_feishu_fk_cascade` 用事务包住整个重建过程；失败回滚不留半截表。
- 增量迁移（`sync_new_executors`）让代码升级时自动同步 DB，不会让新 executor 缺失。

---

## 附录：模块路径速查

| 流程 | 入口模块 | 关键函数 |
|------|---------|---------|
| Todo 执行 | `handlers::execution::start_todo_execution` | `executor_service::run_todo_execution` |
| Cron 调度 | `scheduler::TodoScheduler::load_from_db` | `scheduler::convert_cron_to_utc` |
| 飞书消息 | `services::feishu_listener::on_message` | `services::message_debounce::push` |
| 配置加载 | `config::Config::load` | `normalize_paths` / `clamp_execution_timeout_secs` |
| DB 初始化 | `db::Database::new` | `migrate_feishu_fk_cascade` / `sync_new_executors` |
