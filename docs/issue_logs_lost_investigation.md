# 执行器日志丢失问题调查报告

## 问题描述

"Issue 处理" 这个 TODO 执行时，界面看到有很多输出，但数据库中执行记录没有存下任何日志。

---

## 数据库信息

**数据库路径：** `~/.ntd/data.db`

**相关 TODO：** id=45，标题="Issue 处理"

**相关执行记录：**

| record_id | todo_id | executor | status | pid | 日志数 | started_at | finished_at |
|-----------|---------|----------|--------|-----|--------|------------|-------------|
| 1990 | 45 | claudecode | failed | **NULL** | **0** | 2026-05-17T03:56:16.836Z | 2026-05-17T04:00:24.976Z |
| 1992 | 45 | claudecode | failed | **NULL** | **0** | 2026-05-17T03:56:31.308Z | 2026-05-17T04:00:24.976Z |
| 1996 | 45 | claudecode | running | 4189 | 51 | 2026-05-17T04:11:28.067Z | - |

---

## 关键发现

### 1. 1990 和 1992 的问题

- **pid 为 NULL**：`child.id()` 返回了 None，导致数据库中 pid 字段为空
- **0 条日志**：`execution_logs` 表中没有任何日志
- **stdout/stderr 全为 0**：数据库中这两个字段都是 NULL
- **运行了约 4 分钟**：1990 运行 248 秒，1992 运行 233 秒
- **result 字段**：`程序崩溃，任务被中断`

### 2. 程序崩溃时间线

```
03:56:16 - record 1990 started
03:56:31 - record 1992 started
03:57:10 - run.log 最后一条日志（程序崩溃）
03:57 ~ 04:00 - 程序无日志（崩溃状态）
04:00:24 - 程序重启，日志显示 "Cleaned up 4 orphan execution records"
04:11:28 - record 1996 started（重启后，正常有日志）
```

### 3. 对比正常执行记录

| record_id | todo_id | pid | 日志数 | 说明 |
|-----------|---------|-----|--------|------|
| 1959 | 45 | 63167 | 75 | 正常，手动终止 |
| 1965 | 45 | 66640 | 104 | 正常，手动终止 |
| 1991 | 33 | 1105 | 27 | 正常 |
| 1996 | 45 | 4189 | 51 | 正常，当前 running |

---

## flush 机制说明

执行器日志有**两层缓存刷新机制**（见 `backend/src/executor_service.rs`）：

### 按条数刷新（阈值 5 条）
```rust
const FLUSH_COUNT_THRESHOLD: u64 = 5;  // 第 408 行
if prev + 1 >= FLUSH_COUNT_THRESHOLD {
    // 触发异步 flush 到数据库
}
```

### 定时刷新（每 3 秒）
```rust
let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));  // 第 626 行
```

### 正常退出时的处理
```rust
// 第 814-819 行
let _ = flush_timer.await;
for h in flush_handles.lock().await.drain(..) {
    let _ = h.await;
}
```

**问题：** 程序崩溃时，`flush_timer` 被直接 abort，所有进行中的 flush 任务被强制终止，内存中的日志全部丢失。

---

## 待调查问题

1. **为什么 1990/1992 的 `child.id()` 返回 None？**
   - `command-group` 的 `group_spawn()` 和 `id()` 方法在什么情况下会返回 None？
   - 参考代码：`backend/src/executor_service.rs` 第 356-385 行

2. **为什么日志从未进入内存缓存？**
   - 进程运行了 4 分钟，没有任何日志被解析
   - 是否与 pid 为 NULL 有关联？

3. **程序崩溃的根本原因是什么？**
   - run.log 在 03:57:10 后就没有日志了
   - 没有找到 panic 或 error 日志

---

## 相关代码位置

- 执行器服务：`backend/src/executor_service.rs`
- 数据库操作：`backend/src/db/execution.rs`
- 日志表操作：`backend/src/db/execution.rs` 第 256-306 行
- flush 机制：`backend/src/executor_service.rs` 第 401-688 行
- orphan cleanup：`backend/src/db/execution.rs` 第 1008-1033 行
