use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use command_group::AsyncCommandGroup;

use crate::adapters::{ExecutorRegistry, parse_executor_type};
use crate::db::Database;
use crate::handlers::ExecEvent;
use crate::models::ParsedLogEntry;
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

/// Run a todo execution. Priority: explicit executor > todo stored executor > default.
pub async fn run_todo_execution(
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
    todo_id: i64,
    message: String,
    req_executor: Option<String>,
    trigger_type: &str,
    task_manager: Arc<TaskManager>,
    resume_session_id: Option<String>,
) -> ExecutionResult {
    let task_id = Uuid::new_v4().to_string();
    let mut cancel_rx = task_manager.register(task_id.clone()).await;

    // Get todo to read stored executor
    let todo = db.get_todo(todo_id).await;
    let todo_executor = todo.as_ref().and_then(|t| t.executor.clone());
    let todo_workspace = todo.as_ref().and_then(|t| t.workspace.clone());

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

    let executor = match executor_registry.get(executor_type)
        .or_else(|| executor_registry.get_default()) {
        Some(exec) => exec,
        None => {
            tracing::error!("No executor available for type {:?} and no default registered", executor_type);
            let _ = db.finish_todo_execution(todo_id, false).await;
            send_event(&tx, ExecEvent::Finished { task_id: task_id.clone(), todo_id, success: false, result: Some("No executor available".to_string()) });
            task_manager.remove(&task_id).await;
            return ExecutionResult { task_id, record_id: None };
        }
    };

    let executable_path = executor.executable_path().to_string();
    let session_id_for_executor = resume_session_id.as_deref().unwrap_or(&task_id);
    let is_resume = resume_session_id.is_some();
    let command_args = executor.command_args_with_session(&message, Some(session_id_for_executor), is_resume);

    // Update todo's executor to the one being used
    let executor_str = executor.executor_type().to_string();
    if let Err(e) = db.update_todo_executor(todo_id, &executor_str).await {
        tracing::error!("Failed to update todo executor: {}", e);
    }

    // Create execution record
    let command = format!("{} {}", executable_path, command_args.join(" "));
    let record_id = match db.create_execution_record(todo_id, &command, &executor_str, trigger_type, &task_id, Some(session_id_for_executor)).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to create execution record: {}", e);
            let _ = db.finish_todo_execution(todo_id, false).await;
            task_manager.remove(&task_id).await;
            return ExecutionResult { task_id, record_id: None };
        }
    };

    // Update todo status to running and associate with task
    if let Err(e) = db.start_todo_execution(todo_id, &task_id).await {
        tracing::error!("Failed to start todo execution: {}", e);
        let entry = ParsedLogEntry::error(format!("Failed to start todo execution: {}", e));
        send_event(&tx, ExecEvent::Output { task_id: task_id.clone(), entry });
        send_event(&tx, ExecEvent::Finished { task_id: task_id.clone(), todo_id, success: false, result: Some("Failed to start execution".to_string()) });
        let _ = db.finish_todo_execution(todo_id, false).await;
        task_manager.remove(&task_id).await;
        return ExecutionResult { task_id, record_id: Some(record_id) };
    }

    let task_id_return = task_id.clone();
    let db_clone = db.clone();
    let tx_clone = tx.clone();
    let executor_spawn = executor.clone();
    let task_manager_spawn = task_manager.clone();

    let todo_title = todo.as_ref().map(|t| t.title.clone()).unwrap_or_default();

    // 注册任务信息，用于 WebSocket 同步
    task_manager.register_info(crate::task_manager::TaskInfo {
        task_id: task_id.clone(),
        todo_id,
        todo_title: todo_title.clone(),
        executor: executor_spawn.executor_type().to_string(),
        logs: "[]".to_string(), // 初始为空，WebSocket 同步时会从数据库获取实际日志
    }).await;

    tokio::spawn(async move {
        let execution_start = std::time::Instant::now();

        send_event(&tx_clone, ExecEvent::Started { task_id: task_id.clone(), todo_id, todo_title: todo_title.clone(), executor: executor_spawn.executor_type().to_string() });

        let entry = ParsedLogEntry::info(format!("Starting {}", executor_spawn.executor_type()));
        send_event(&tx_clone, ExecEvent::Output { task_id: task_id.clone(), entry });

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
                let entry = ParsedLogEntry::error(format!("Failed to spawn executor: {}", e));
                send_event(&tx_clone, ExecEvent::Output { task_id: task_id.clone(), entry });
                send_event(&tx_clone, ExecEvent::Finished { task_id: task_id.clone(), todo_id, success: false, result: None });
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
            let _ = db_clone.update_execution_record_pid(record_id, Some(child_id as i32)).await;
        }

        let stdout_handle = child.inner().stdout.take();
        let stderr_handle = child.inner().stderr.take();

        let logs = Arc::new(Mutex::new(Vec::<ParsedLogEntry>::new()));
        let logs_for_db = logs.clone();
        let logs_for_result = logs.clone();

        let executor_for_parse = executor_spawn.clone();

        // Process stdout
        let stdout_task = if let Some(stdout_reader) = stdout_handle {
            let tx_clone = tx.clone();
            let tid = task_id.clone();
            let executor_clone = executor_for_parse.clone();
            let logs_for_db = logs_for_db.clone();
            let db_for_todo = db_clone.clone();
            let rid = record_id;

            Some(tokio::spawn(async move {
                let mut reader = BufReader::new(stdout_reader).lines();
                let mut log_count = 0u64;
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(parsed) = executor_clone.parse_output_line(&line) {
                        // Detect todo progress updates
                        if let Some(progress) = crate::todo_progress::try_extract_todo_progress(&parsed) {
                            if let Ok(progress_json) = serde_json::to_string(&progress) {
                                let _ = db_for_todo.update_execution_record_todo_progress(rid, &progress_json).await;
                            }
                            send_event(&tx_clone, ExecEvent::TodoProgress {
                                task_id: tid.clone(),
                                progress,
                            });
                        }

                        // Send stats update after tool calls or every 10 log entries
                        let is_tool_call = parsed.log_type == "tool_use" || parsed.log_type == "tool_call" || parsed.log_type == "tool";
                        log_count += 1;
                        if is_tool_call || log_count % 10 == 0 {
                            let current_logs = logs_for_db.lock().await;
                            let tool_calls = current_logs.iter()
                                .filter(|l| l.log_type == "tool_use" || l.log_type == "tool_call" || l.log_type == "tool")
                                .count() as u64;
                            let conversation_turns = current_logs.iter()
                                .filter(|l| l.log_type == "assistant" || l.log_type == "result" || l.log_type == "text")
                                .count() as u64;
                            let thinking_count = current_logs.iter()
                                .filter(|l| l.log_type == "thinking")
                                .count() as u64;
                            let stats = crate::models::ExecutionStats {
                                tool_calls,
                                conversation_turns,
                                thinking_count,
                            };
                            send_event(&tx_clone, ExecEvent::ExecutionStats {
                                task_id: tid.clone(),
                                stats,
                            });
                        }

                        logs_for_db.lock().await.push(parsed.clone());
                        send_event(&tx_clone, ExecEvent::Output { task_id: tid.clone(), entry: parsed });
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
        let stderr_task = if let Some(stderr_reader) = stderr_handle {
            Some(tokio::spawn(async move {
                let mut reader = BufReader::new(stderr_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let entry = if let Some(parsed) = executor_for_stderr.parse_stderr_line(&line) {
                        parsed
                    } else {
                        ParsedLogEntry::stderr(line.clone())
                    };
                    logs_for_stderr.lock().await.push(entry.clone());
                    send_event(&stderr_tx, ExecEvent::Output { task_id: stderr_tid.clone(), entry });
                }
            }))
        } else {
            None
        };

        let status = tokio::select! {
            biased;
            _ = cancel_rx.recv() => {
                // Cancelled (or channel closed): 使用 command-group 安全杀死整个进程组
                kill_process_tree(&mut child).await;

                // 收割僵尸进程
                let _status = child.wait().await;

                if let Some(handle) = stdout_task {
                    let _ = handle.await;
                }
                if let Some(handle) = stderr_task {
                    let _ = handle.await;
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
                send_event(&tx_clone, ExecEvent::Finished { task_id: task_id.clone(), todo_id, success: false, result: None });
                task_manager_spawn.remove(&task_id).await;
                return;
            }
            status = child.wait() => {
                // 子进程已自然退出，command-group 的进程组已自动清理

                if let Some(handle) = stdout_task {
                    let _ = handle.await;
                }
                if let Some(handle) = stderr_task {
                    let _ = handle.await;
                }

                status
            }
        };

        let exit_code = status.as_ref().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        let success = executor_spawn.check_success(exit_code);

        // Try post-execution todo progress extraction (for executors like hermes that don't expose tool calls in stdout)
        if let Some(progress) = executor_spawn.post_execution_todo_progress() {
            if let Ok(progress_json) = serde_json::to_string(&progress) {
                let _ = db_clone.update_execution_record_todo_progress(record_id, &progress_json).await;
                send_event(&tx_clone, ExecEvent::TodoProgress {
                    task_id: task_id.clone(),
                    progress,
                });
            }
        }

        let all_logs_snapshot = logs_for_result.lock().await.clone();
        let result_str = executor_spawn.get_final_result(&all_logs_snapshot).unwrap_or_default();

        // Extract execution stats from logs
        // tool_calls: tool_use (claudecode), tool_call (kimi), tool (atomcode, opencode)
        // For hermes, use get_tool_calls_count() which parses from output summary
        let tool_calls = executor_spawn.get_tool_calls_count().unwrap_or_else(|| {
            all_logs_snapshot.iter()
                .filter(|l| l.log_type == "tool_use" || l.log_type == "tool_call" || l.log_type == "tool")
                .count() as u64
        });
        // conversation_turns: assistant/result (claudecode), text (kimi, atomcode, hermes)
        let conversation_turns = all_logs_snapshot.iter()
            .filter(|l| l.log_type == "assistant" || l.log_type == "result" || l.log_type == "text")
            .count() as u64;
        // thinking_count: thinking (claudecode)
        let thinking_count = all_logs_snapshot.iter()
            .filter(|l| l.log_type == "thinking")
            .count() as u64;
        let execution_stats = crate::models::ExecutionStats {
            tool_calls,
            conversation_turns,
            thinking_count,
        };
        if let Ok(stats_json) = serde_json::to_string(&execution_stats) {
            let _ = db_clone.update_execution_record_stats(record_id, &stats_json).await;
        }

        let final_status = if success { crate::models::ExecutionStatus::Success.as_str() } else { crate::models::ExecutionStatus::Failed.as_str() };
        let logs_json = serde_json::to_string(&all_logs_snapshot)
            .unwrap_or_else(|e| { tracing::error!("Failed to serialize logs: {}", e); "[]".to_string() });
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

        let _ = db_clone.update_execution_record(record_id, final_status, &logs_json, &result_str, usage.as_ref(), model.as_deref()).await;

        let _ = db_clone.finish_todo_execution(todo_id, success).await;

        let entry = ParsedLogEntry::new(
            if success { "info" } else { "error" },
            format!("Executor finished with exit_code: {}, result: {}", exit_code, result_str),
        );
        send_event(&tx_clone, ExecEvent::Output { task_id: task_id.clone(), entry });

        send_event(&tx_clone, ExecEvent::Finished { task_id: task_id.clone(), todo_id, success, result: Some(result_str) });
        task_manager_spawn.remove(&task_id).await;
    });

    ExecutionResult { task_id: task_id_return, record_id: Some(record_id) }
}
