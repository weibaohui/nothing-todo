use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use command_group::AsyncCommandGroup;

use crate::adapters::{parse_executor_type, ExecutorRegistry};
use crate::db::{Database, NewExecutionRecord};
use crate::handlers::ExecEvent;
use crate::models::{ExecutorType, ParsedLogEntry};
use crate::task_manager::TaskManager;

fn send_event(tx: &broadcast::Sender<ExecEvent>, event: ExecEvent) {
    let _ = tx.send(event);
}

/// 使用 command-group 安全地杀死进程树
/// command-group 会自动创建进程组，kill() 时会杀死整个进程组
async fn kill_process_tree(child: &mut command_group::AsyncGroupChild) {
    if let Err(e) = child.kill().await {
        tracing::warn!("杀死进程组失败: {}", e);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionResult {
    pub task_id: String,
    pub record_id: Option<i64>,
}

pub struct RunTodoExecutionRequest {
    pub db: Arc<Database>,
    pub executor_registry: Arc<ExecutorRegistry>,
    pub tx: broadcast::Sender<ExecEvent>,
    pub task_manager: Arc<TaskManager>,
    pub todo_id: i64,
    pub message: String,
    pub req_executor: Option<String>,
    pub trigger_type: String,
    pub params: Option<std::collections::HashMap<String, String>>,
    pub resume_session_id: Option<String>,
    pub resume_message: Option<String>,
}

/// Run a todo execution. Priority: explicit executor > todo stored executor > default.
pub async fn run_todo_execution(request: RunTodoExecutionRequest) -> ExecutionResult {
    let RunTodoExecutionRequest {
        db,
        executor_registry,
        tx,
        task_manager,
        todo_id,
        message,
        req_executor,
        trigger_type,
        params,
        resume_session_id,
        resume_message,
    } = request;
    let message = params
        .as_ref()
        .map(|params| crate::models::replace_placeholders(&message, params))
        .unwrap_or(message);
    let task_id = Uuid::new_v4().to_string();
    let mut cancel_rx = task_manager.register(task_id.clone()).await;

    // Get todo to read stored executor
    let todo = match db.get_todo(todo_id).await {
        Ok(Some(t)) => Some(t),
        Ok(None) => None,
        Err(e) => {
            tracing::error!(
                "Failed to fetch todo {} for executor selection: {}",
                todo_id,
                e
            );
            None
        }
    };
    let todo_executor = todo.as_ref().and_then(|t| t.executor.clone());
    let todo_workspace = todo.as_ref().and_then(|t| t.workspace.clone());
    let todo_worktree = todo.as_ref().and_then(|t| t.worktree.clone());

    // Determine which executor to use: explicit > todo stored > default
    let executor_type = req_executor
        .as_deref()
        .and_then(|exec| {
            parse_executor_type(exec).or_else(|| {
                tracing::warn!("Unknown explicit executor '{}', trying todo executor", exec);
                None
            })
        })
        .or_else(|| {
            todo_executor.as_deref().and_then(|exec| {
                parse_executor_type(exec).or_else(|| {
                    tracing::warn!("Unknown todo executor '{}', falling back to default", exec);
                    None
                })
            })
        })
        .unwrap_or_default();

    let executor = match executor_registry
        .get(executor_type)
        .or_else(|| executor_registry.get_default())
    {
        Some(exec) => exec,
        None => {
            tracing::error!(
                "No executor available for type {:?} and no default registered",
                executor_type
            );
            let _ = db.finish_todo_execution(todo_id, false).await;
            send_event(
                &tx,
                ExecEvent::Finished {
                    task_id: task_id.clone(),
                    todo_id,
                    todo_title: todo.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
                    executor: executor_type.to_string(),
                    success: false,
                    result: Some("No executor available".to_string()),
                },
            );
            task_manager.remove(&task_id).await;
            return ExecutionResult {
                task_id,
                record_id: None,
            };
        }
    };

    let executable_path = executor.executable_path().to_string();
    let session_id_for_executor = resume_session_id.as_deref().unwrap_or(&task_id);
    let is_resume = resume_session_id.is_some();
    let mut command_args =
        executor.command_args_with_session(&message, Some(session_id_for_executor), is_resume);

    // Add worktree flag for claude_code and codex executors
    let exec_type = executor.executor_type();
    if let Some(wt) = todo_worktree.as_deref() {
        if exec_type == ExecutorType::Claudecode || exec_type == ExecutorType::Codex {
            command_args.push("--worktree".to_string());
            command_args.push(wt.to_string());
        }
    }

    // Update todo's executor to the one being used
    let executor_str = executor.executor_type().to_string();
    if let Err(e) = db.update_todo_executor(todo_id, &executor_str).await {
        tracing::error!("Failed to update todo executor: {}", e);
    }

    // Create execution record
    let command = format!("{} {}", executable_path, command_args.join(" "));
    let record_id = match db
        .create_execution_record(NewExecutionRecord {
            todo_id,
            command: &command,
            executor: &executor_str,
            trigger_type: &trigger_type,
            task_id: &task_id,
            session_id: Some(session_id_for_executor),
            resume_message: resume_message.as_deref(),
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to create execution record: {}", e);
            let _ = db.finish_todo_execution(todo_id, false).await;
            task_manager.remove(&task_id).await;
            return ExecutionResult {
                task_id,
                record_id: None,
            };
        }
    };

    // Update todo status to running and associate with task
    if let Err(e) = db.start_todo_execution(todo_id, &task_id).await {
        tracing::error!("Failed to start todo execution: {}", e);
        let entry = ParsedLogEntry::error(format!("Failed to start todo execution: {}", e));
        send_event(
            &tx,
            ExecEvent::Output {
                task_id: task_id.clone(),
                entry,
            },
        );
        send_event(
            &tx,
            ExecEvent::Finished {
                task_id: task_id.clone(),
                todo_id,
                todo_title: todo.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
                executor: executor_str.clone(),
                success: false,
                result: Some("Failed to start execution".to_string()),
            },
        );
        let _ = db.finish_todo_execution(todo_id, false).await;
        let _ = db
            .update_execution_record(
                record_id,
                crate::models::ExecutionStatus::Failed.as_str(),
                "[]",
                &format!("Failed to start todo execution: {}", e),
                None,
                None,
            )
            .await;
        task_manager.remove(&task_id).await;
        return ExecutionResult {
            task_id,
            record_id: Some(record_id),
        };
    }

    let task_id_return = task_id.clone();
    let db_clone = db.clone();
    let tx_clone = tx.clone();
    let executor_spawn = executor.clone();
    let task_manager_spawn = task_manager.clone();

    let todo_title = todo.as_ref().map(|t| t.title.clone()).unwrap_or_default();

    // 注册任务信息，用于 WebSocket 同步
    task_manager
        .register_info(crate::task_manager::TaskInfo {
            task_id: task_id.clone(),
            todo_id,
            todo_title: todo_title.clone(),
            executor: executor_spawn.executor_type().to_string(),
            logs: "[]".to_string(), // 初始为空，WebSocket 同步时会从数据库获取实际日志
        })
        .await;

    tokio::spawn(async move {
        let execution_start = std::time::Instant::now();

        send_event(
            &tx_clone,
            ExecEvent::Started {
                task_id: task_id.clone(),
                todo_id,
                todo_title: todo_title.clone(),
                executor: executor_spawn.executor_type().to_string(),
            },
        );

        let entry = ParsedLogEntry::info(format!("Starting {}", executor_spawn.executor_type()));
        send_event(
            &tx_clone,
            ExecEvent::Output {
                task_id: task_id.clone(),
                entry,
            },
        );

        // 使用 command-group 创建进程组，自动管理进程树
        let mut cmd = tokio::process::Command::new(&executable_path);

        tracing::debug!(
            executable = %executable_path,
            arg_count = command_args.len(),
            "Spawning executor"
        );

        cmd.args(&command_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped());

        // 设置工作目录（如果指定了 workspace）
        if let Some(ws) = todo_workspace.as_ref() {
            cmd.current_dir(ws);
        }

        // 使用 command-group 的 group_spawn 创建进程组
        let mut child = match cmd.group_spawn() {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!("Failed to spawn executor: {}", e);
                let entry = ParsedLogEntry::error(error_msg.clone());
                send_event(
                    &tx_clone,
                    ExecEvent::Output {
                        task_id: task_id.clone(),
                        entry,
                    },
                );
                send_event(
                    &tx_clone,
                    ExecEvent::Finished {
                        task_id: task_id.clone(),
                        todo_id,
                        todo_title: todo_title.clone(),
                        executor: executor_spawn.executor_type().to_string(),
                        success: false,
                        result: Some(error_msg),
                    },
                );
                let _ = db_clone.finish_todo_execution(todo_id, false).await;
                task_manager_spawn.remove(&task_id).await;
                return;
            }
        };

        let child_id = child.id().unwrap_or(0);

        // Close stdin immediately so child processes get EOF when they try to read it.
        // Without this, processes that read stdin after finishing work will hang forever.
        drop(child.inner().stdin.take());

        // 保存 pid 到 execution_records 表 (使用进程组 leader 的 pid)
        if child_id > 0 {
            let _ = db_clone
                .update_execution_record_pid(record_id, Some(child_id as i32))
                .await;
        }

        let stdout_handle = child.inner().stdout.take();
        let stderr_handle = child.inner().stderr.take();

        let logs = Arc::new(Mutex::new(Vec::<ParsedLogEntry>::new()));
        let logs_for_db = logs.clone();
        let logs_for_result = logs.clone();
        let flush_pending = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let unflushed_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let flush_handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>> =
            Arc::new(Mutex::new(Vec::new()));
        const FLUSH_COUNT_THRESHOLD: u64 = 5;

        let executor_for_parse = executor_spawn.clone();

        // Process stdout
        let stdout_task = if let Some(stdout_reader) = stdout_handle {
            let tx_clone = tx.clone();
            let tid = task_id.clone();
            let executor_clone = executor_for_parse.clone();
            let logs_for_db = logs_for_db.clone();
            let db_for_todo = db_clone.clone();
            let rid = record_id;
            let flush_pending_for_stdout = flush_pending.clone();
            let unflushed_for_stdout = unflushed_count.clone();
            let flush_handles_stdout = flush_handles.clone();

            Some(tokio::spawn(async move {
                let mut reader = BufReader::new(stdout_reader).lines();
                let mut log_count = 0u64;
                let mut session_id_updated = false;
                while let Ok(Some(line)) = reader.next_line().await {
                    // Extract and update session_id if present
                    if !session_id_updated {
                        if let Some(sid) = executor_clone.extract_session_id(&line) {
                            let _ = db_for_todo
                                .update_execution_record_session_id(rid, &sid)
                                .await;
                            session_id_updated = true;
                        }
                    }
                    if let Some(parsed) = executor_clone.parse_output_line(&line) {
                        // Detect todo progress updates
                        if let Some(progress) =
                            crate::todo_progress::try_extract_todo_progress(&parsed)
                        {
                            if let Ok(progress_json) = serde_json::to_string(&progress) {
                                let _ = db_for_todo
                                    .update_execution_record_todo_progress(rid, &progress_json)
                                    .await;
                            }
                            send_event(
                                &tx_clone,
                                ExecEvent::TodoProgress {
                                    task_id: tid.clone(),
                                    progress,
                                },
                            );
                        }

                        // Send stats update after tool calls or every 10 log entries
                        let is_tool_call = parsed.log_type == "tool_use"
                            || parsed.log_type == "tool_call"
                            || parsed.log_type == "tool";
                        log_count += 1;
                        if is_tool_call || log_count.is_multiple_of(10) {
                            let current_logs = logs_for_db.lock().await;
                            let tool_calls = current_logs
                                .iter()
                                .filter(|l| {
                                    l.log_type == "tool_use"
                                        || l.log_type == "tool_call"
                                        || l.log_type == "tool"
                                })
                                .count() as u64;
                            let conversation_turns = current_logs
                                .iter()
                                .filter(|l| {
                                    l.log_type == "assistant"
                                        || l.log_type == "result"
                                        || l.log_type == "text"
                                })
                                .count()
                                as u64;
                            let thinking_count = current_logs
                                .iter()
                                .filter(|l| l.log_type == "thinking")
                                .count() as u64;
                            let stats = crate::models::ExecutionStats {
                                tool_calls,
                                conversation_turns,
                                thinking_count,
                            };
                            send_event(
                                &tx_clone,
                                ExecEvent::ExecutionStats {
                                    task_id: tid.clone(),
                                    stats,
                                },
                            );
                        }

                        logs_for_db.lock().await.push(parsed.clone());
                        let prev =
                            unflushed_for_stdout.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if prev + 1 >= FLUSH_COUNT_THRESHOLD
                            && !flush_pending_for_stdout
                                .swap(true, std::sync::atomic::Ordering::Relaxed)
                        {
                            unflushed_for_stdout.store(0, std::sync::atomic::Ordering::Relaxed);
                            let snapshot = std::mem::take(&mut *logs_for_db.lock().await);
                            let db_flush = db_for_todo.clone();
                            let rid_flush = rid;
                            let fp = flush_pending_for_stdout.clone();
                            let h = tokio::spawn(async move {
                                if let Ok(json) = serde_json::to_string(&snapshot) {
                                    let _ = db_flush
                                        .append_execution_record_logs(rid_flush, &json)
                                        .await;
                                }
                                fp.store(false, std::sync::atomic::Ordering::Relaxed);
                            });
                            flush_handles_stdout.lock().await.push(h);
                        }
                        send_event(
                            &tx_clone,
                            ExecEvent::Output {
                                task_id: tid.clone(),
                                entry: parsed,
                            },
                        );
                    }
                }
            }))
        } else {
            None
        };

        // Capture stderr
        let stderr_tx = tx.clone();
        let stderr_tid = task_id.clone();
        let logs_for_stderr = logs.clone();
        let executor_for_stderr = executor_spawn.clone();
        let db_for_stderr = db_clone.clone();
        let rid_for_stderr = record_id;
        let flush_for_stderr = flush_pending.clone();
        let unflushed_for_stderr = unflushed_count.clone();
        let flush_handles_stderr = flush_handles.clone();
        let stderr_task = stderr_handle.map(|stderr_reader| {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let entry = if let Some(parsed) = executor_for_stderr.parse_stderr_line(&line) {
                        parsed
                    } else {
                        ParsedLogEntry::stderr(line.clone())
                    };
                    logs_for_stderr.lock().await.push(entry.clone());
                    let prev =
                        unflushed_for_stderr.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if prev + 1 >= FLUSH_COUNT_THRESHOLD
                        && !flush_for_stderr.swap(true, std::sync::atomic::Ordering::Relaxed)
                    {
                        unflushed_for_stderr.store(0, std::sync::atomic::Ordering::Relaxed);
                        let snapshot = std::mem::take(&mut *logs_for_stderr.lock().await);
                        let db_flush = db_for_stderr.clone();
                        let rid_flush = rid_for_stderr;
                        let fp = flush_for_stderr.clone();
                        let h = tokio::spawn(async move {
                            if let Ok(json) = serde_json::to_string(&snapshot) {
                                let _ = db_flush
                                    .append_execution_record_logs(rid_flush, &json)
                                    .await;
                            }
                            fp.store(false, std::sync::atomic::Ordering::Relaxed);
                        });
                        flush_handles_stderr.lock().await.push(h);
                    }
                    send_event(
                        &stderr_tx,
                        ExecEvent::Output {
                            task_id: stderr_tid.clone(),
                            entry,
                        },
                    );
                }
            })
        });

        // 定时兜底 flush：每 3 秒检查未刷新条目，有则写库
        let timer_db = db_clone.clone();
        let timer_logs = logs.clone();
        let timer_fp = flush_pending.clone();
        let timer_uc = unflushed_count.clone();
        let timer_handles = flush_handles.clone();
        let flush_timer = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
            loop {
                interval.tick().await;
                if timer_fp.load(std::sync::atomic::Ordering::Relaxed) {
                    continue;
                }
                let n = timer_uc.swap(0, std::sync::atomic::Ordering::Relaxed);
                if n > 0 && !timer_fp.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    let snapshot = std::mem::take(&mut *timer_logs.lock().await);
                    let db_f = timer_db.clone();
                    let rid_f = record_id;
                    let fp = timer_fp.clone();
                    let h = tokio::spawn(async move {
                        if let Ok(json) = serde_json::to_string(&snapshot) {
                            let _ = db_f.append_execution_record_logs(rid_f, &json).await;
                        }
                        fp.store(false, std::sync::atomic::Ordering::Relaxed);
                    });
                    timer_handles.lock().await.push(h);
                } else if n > 0 {
                    timer_uc.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
                }
            }
        });

        let status = tokio::select! {
            biased;
            _ = cancel_rx.recv() => {
                // Cancelled (or channel closed): 使用 command-group 安全杀死整个进程组
                kill_process_tree(&mut child).await;
                flush_timer.abort();

                // 收割僵尸进程
                let _status = child.wait().await;

                if let Some(handle) = stdout_task {
                    let _ = handle.await;
                }
                if let Some(handle) = stderr_task {
                    let _ = handle.await;
                }

                // 等待所有进行中的 flush 任务完成，防止旧快照覆盖
                for h in flush_handles.lock().await.drain(..) {
                    let _ = h.await;
                }

                let _ = db_clone.update_todo_status(todo_id, crate::models::TodoStatus::Cancelled).await;
                let _ = db_clone.update_todo_task_id(todo_id, None).await;

                // 更新 execution_records 状态为 failed
                let logs_json = serde_json::to_string(&*logs.lock().await)
                    .unwrap_or_else(|e| { tracing::error!("Failed to serialize logs: {}", e); "[]".to_string() });
                let _ = db_clone.update_execution_record(
                    record_id,
                    crate::models::ExecutionStatus::Failed.as_str(),
                    &logs_json,
                    "任务已被手动停止",
                    None,
                    None,
                ).await;

                let entry = ParsedLogEntry::error("Execution cancelled by user");
                send_event(&tx_clone, ExecEvent::Output { task_id: task_id.clone(), entry });
                send_event(&tx_clone, ExecEvent::Finished { task_id: task_id.clone(), todo_id, todo_title: todo_title.clone(), executor: executor_spawn.executor_type().to_string(), success: false, result: Some("Task was cancelled by user".to_string()) });
                task_manager_spawn.remove(&task_id).await;
                return;
            }
            status = child.wait() => {
                // 子进程已自然退出，command-group 的进程组已自动清理
                flush_timer.abort();

                if let Some(handle) = stdout_task {
                    let _ = handle.await;
                }
                if let Some(handle) = stderr_task {
                    let _ = handle.await;
                }

                status
            }
        };

        let exit_code = status
            .as_ref()
            .map(|s| s.code().unwrap_or(-1))
            .unwrap_or(-1);
        let success = executor_spawn.check_success(exit_code);

        // Try post-execution todo progress extraction (for executors like hermes that don't expose tool calls in stdout)
        if let Some(progress) = executor_spawn.post_execution_todo_progress() {
            if let Ok(progress_json) = serde_json::to_string(&progress) {
                let _ = db_clone
                    .update_execution_record_todo_progress(record_id, &progress_json)
                    .await;
                send_event(
                    &tx_clone,
                    ExecEvent::TodoProgress {
                        task_id: task_id.clone(),
                        progress,
                    },
                );
            }
        }

        // 等待所有进行中的 flush 任务完成，防止旧快照覆盖最终写入
        for h in flush_handles.lock().await.drain(..) {
            let _ = h.await;
        }

        // 从数据库读取完整的日志集（因为定期 flush 会 drain 内存中的 vec）
        let remaining = std::mem::take(&mut *logs_for_result.lock().await);
        let all_logs_snapshot = match db_clone.get_execution_record(record_id).await {
            Ok(Some(record)) if !record.logs.is_empty() && record.logs != "[]" => {
                let mut base: Vec<ParsedLogEntry> = match serde_json::from_str(&record.logs) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse execution logs JSON, record_id={}: {}",
                            record_id,
                            e
                        );
                        Vec::new()
                    }
                };
                base.extend(remaining);
                base
            }
            Ok(_) => remaining,
            Err(e) => {
                tracing::error!("Failed to fetch execution record {}: {}", record_id, e);
                remaining
            }
        };
        let result_str = executor_spawn
            .get_final_result(&all_logs_snapshot)
            .unwrap_or_default();

        // Extract execution stats from logs
        // tool_calls: tool_use (claudecode), tool_call (kimi), tool (atomcode, opencode)
        // For hermes, use get_tool_calls_count() which parses from output summary
        let tool_calls = executor_spawn.get_tool_calls_count().unwrap_or_else(|| {
            all_logs_snapshot
                .iter()
                .filter(|l| {
                    l.log_type == "tool_use" || l.log_type == "tool_call" || l.log_type == "tool"
                })
                .count() as u64
        });
        // conversation_turns: assistant/result (claudecode), text (kimi, atomcode, hermes)
        let conversation_turns = all_logs_snapshot
            .iter()
            .filter(|l| l.log_type == "assistant" || l.log_type == "result" || l.log_type == "text")
            .count() as u64;
        // thinking_count: thinking (claudecode)
        let thinking_count = all_logs_snapshot
            .iter()
            .filter(|l| l.log_type == "thinking")
            .count() as u64;
        let execution_stats = crate::models::ExecutionStats {
            tool_calls,
            conversation_turns,
            thinking_count,
        };
        if let Ok(stats_json) = serde_json::to_string(&execution_stats) {
            let _ = db_clone
                .update_execution_record_stats(record_id, &stats_json)
                .await;
        }

        let final_status = if success {
            crate::models::ExecutionStatus::Success.as_str()
        } else {
            crate::models::ExecutionStatus::Failed.as_str()
        };
        let logs_json = serde_json::to_string(&all_logs_snapshot).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize logs: {}", e);
            "[]".to_string()
        });
        let mut usage = executor_spawn.get_usage(&all_logs_snapshot);
        let model = executor_spawn.get_model();

        // Always use wall-clock duration (start to end of execution)
        // This ensures duration is always available, regardless of executor support
        let wall_clock_duration_ms = execution_start.elapsed().as_millis() as u64;
        match usage.as_mut() {
            Some(u) => {
                // Override executor-reported duration with actual wall-clock time
                u.duration_ms = Some(wall_clock_duration_ms);
            }
            None => {
                usage = Some(crate::models::ExecutionUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                    total_cost_usd: None,
                    duration_ms: Some(wall_clock_duration_ms),
                });
            }
        }

        let _ = db_clone
            .update_execution_record(
                record_id,
                final_status,
                &logs_json,
                &result_str,
                usage.as_ref(),
                model.as_deref(),
            )
            .await;

        let _ = db_clone.finish_todo_execution(todo_id, success).await;

        let entry = ParsedLogEntry::new(
            if success { "info" } else { "error" },
            format!(
                "Executor finished with exit_code: {}, result: {}",
                exit_code, result_str
            ),
        );
        send_event(
            &tx_clone,
            ExecEvent::Output {
                task_id: task_id.clone(),
                entry,
            },
        );

        send_event(
            &tx_clone,
            ExecEvent::Finished {
                task_id: task_id.clone(),
                todo_id,
                todo_title: todo_title.clone(),
                executor: executor_spawn.executor_type().to_string(),
                success,
                result: Some(result_str),
            },
        );
        task_manager_spawn.remove(&task_id).await;
    });

    ExecutionResult {
        task_id: task_id_return,
        record_id: Some(record_id),
    }
}

/// Run a todo execution with parameter substitution.
/// Replaces placeholders `{{key}}` in the message with corresponding values from params before execution.
pub async fn run_todo_execution_with_params(
    mut request: RunTodoExecutionRequest,
) -> ExecutionResult {
    if let Some(params) = request.params.take() {
        request.message = crate::models::replace_placeholders(&request.message, &params);
    }
    run_todo_execution(request).await
}
