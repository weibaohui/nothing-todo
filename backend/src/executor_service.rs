use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

use crate::adapters::ExecutorRegistry;
use crate::db::Database;
use crate::handlers::ExecEvent;
use crate::models::{ParsedLogEntry, ExecutorType};
use crate::task_manager::TaskManager;

fn parse_executor_type(executor: &str) -> ExecutorType {
    match executor.to_lowercase().as_str() {
        "claudecode" | "claude" => ExecutorType::Claudecode,
        "codebuddy" | "cbc" => ExecutorType::Codebuddy,
        "opencode" => ExecutorType::Opencode,
        _ => ExecutorType::Joinai,
    }
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
) -> String {
    let task_id = Uuid::new_v4().to_string();
    let mut cancel_rx = task_manager.register(task_id.clone()).await;

    // Get todo to read stored executor
    let todo = db.get_todo(todo_id).await;
    let todo_executor = todo.as_ref().and_then(|t| t.executor.clone());

    // Determine which executor to use
    let executor_type = if let Some(exec) = req_executor {
        parse_executor_type(&exec)
    } else if let Some(exec) = todo_executor {
        parse_executor_type(&exec)
    } else {
        ExecutorType::default()
    };

    let executor = executor_registry.get(executor_type)
        .unwrap_or_else(|| executor_registry.get_default().unwrap());

    let executable_path = executor.executable_path().to_string();
    let command_args = executor.command_args(&message);

    // Update todo's executor to the one being used
    let executor_str = executor.executor_type().to_string();
    db.update_todo_executor(todo_id, &executor_str).await;

    // Create execution record
    let command = format!("{} {}", executable_path, command_args.join(" "));
    let record_id = db.create_execution_record(todo_id, &command, &executor_str, trigger_type).await;

    // Update todo status to running and associate with task
    db.start_todo_execution(todo_id, &task_id).await;

    let task_id_return = task_id.clone();
    let db_clone = db.clone();
    let tx_clone = tx.clone();
    let executor_spawn = executor.clone();
    let message_clone = message.clone();
    let task_manager_spawn = task_manager.clone();

    let todo_title = todo.as_ref().map(|t| t.title.clone()).unwrap_or_default();

    tokio::spawn(async move {
        let _ = tx_clone.send(ExecEvent::Started { task_id: task_id.clone(), todo_id, todo_title: todo_title.clone() });

        let entry = ParsedLogEntry::info(format!("Starting {} with message: {}", executor_spawn.executor_type(), message_clone));
        let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });

        let mut cmd = Command::new(&executable_path);
        cmd.args(&command_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped());

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let entry = ParsedLogEntry::error(format!("Failed to spawn executor: {}", e));
                let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });
                let _ = tx_clone.send(ExecEvent::Finished { task_id: task_id.clone(), todo_id, success: false, result: None });
                db_clone.finish_todo_execution(todo_id, false).await;
                task_manager_spawn.remove(&task_id).await;
                return;
            }
        };

        let child_id = child.id().unwrap_or(0);

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let logs = Arc::new(Mutex::new(Vec::<ParsedLogEntry>::new()));
        let logs_for_db = logs.clone();
        let logs_for_result = logs.clone();

        let executor_for_parse = executor_spawn.clone();

        // Process stdout
        let stdout_task = if let Some(stdout_reader) = stdout_handle {
            let tx_clone = tx.clone();
            let tid = task_id.clone();
            let db_clone2 = db_clone.clone();
            let rid = record_id;
            let executor_clone = executor_for_parse.clone();
            let logs_for_db = logs_for_db.clone();

            Some(tokio::spawn(async move {
                let mut reader = BufReader::new(stdout_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(parsed) = executor_clone.parse_output_line(&line) {
                        logs_for_db.lock().await.push(parsed.clone());
                        let _ = tx_clone.send(ExecEvent::Output { task_id: tid.clone(), entry: parsed });

                        let logs_json = serde_json::to_string(&*logs_for_db.lock().await).unwrap_or_default();
                        db_clone2.update_execution_record(rid, "running", &logs_json, "").await;
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
        let stderr_task = if let Some(stderr_reader) = stderr_handle {
            Some(tokio::spawn(async move {
                let mut reader = BufReader::new(stderr_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let entry = ParsedLogEntry::stderr(line.clone());
                    logs_for_stderr.lock().await.push(entry.clone());
                    let _ = stderr_tx.send(ExecEvent::Output { task_id: stderr_tid.clone(), entry });
                }
            }))
        } else {
            None
        };

        let status = tokio::select! {
            biased;
            Some(()) = cancel_rx.recv() => {
                // Cancelled: kill the process group to ensure all child processes are terminated
                #[cfg(unix)]
                if child_id > 0 {
                    unsafe {
                        libc::kill(-(child_id as i32), libc::SIGKILL);
                    }
                }

                // Also try to kill the direct child
                let _ = child.kill().await;
                let _status = child.wait().await;

                if let Some(handle) = stdout_task {
                    let _ = handle.await;
                }
                if let Some(handle) = stderr_task {
                    let _ = handle.await;
                }

                db_clone.update_todo_status(todo_id, crate::models::TodoStatus::Cancelled).await;
                db_clone.update_todo_task_id(todo_id, None).await;

                let entry = ParsedLogEntry::error("Execution cancelled by user");
                let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });
                let _ = tx_clone.send(ExecEvent::Finished { task_id: task_id.clone(), todo_id, success: false, result: None });
                task_manager_spawn.remove(&task_id).await;
                return;
            }
            status = child.wait() => {
                if let Some(handle) = stdout_task {
                    let _ = handle.await;
                }
                if let Some(handle) = stderr_task {
                    let _ = handle.await;
                }

                // Clean up the process group to ensure no grandchild processes are left behind
                #[cfg(unix)]
                if child_id > 0 {
                    unsafe {
                        libc::kill(-(child_id as i32), libc::SIGKILL);
                    }
                }

                status
            }
        };

        let exit_code = status.as_ref().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        let success = executor_spawn.check_success(exit_code);

        let all_logs_snapshot = logs_for_result.lock().await.clone();
        let result_str = executor_spawn.get_final_result(&all_logs_snapshot).unwrap_or_default();

        let final_status = if success { "success" } else { "failed" };
        let logs_json = serde_json::to_string(&all_logs_snapshot).unwrap_or_default();
        let usage = executor_spawn.get_usage(&all_logs_snapshot);
        if let Some(u) = usage {
            db_clone.update_execution_record_with_usage(record_id, final_status, &logs_json, &result_str, &u).await;
        } else {
            db_clone.update_execution_record(record_id, final_status, &logs_json, &result_str).await;
        }

        if let Some(model) = executor_spawn.get_model() {
            db_clone.update_execution_record_with_model(record_id, &model).await;
        }

        db_clone.finish_todo_execution(todo_id, success).await;

        let entry = ParsedLogEntry::new(
            if success { "info" } else { "error" },
            format!("Executor finished with exit_code: {}, result: {}", exit_code, result_str),
        );
        let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });

        let _ = tx_clone.send(ExecEvent::Finished { task_id: task_id.clone(), todo_id, success, result: Some(result_str) });
        task_manager_spawn.remove(&task_id).await;
    });

    task_id_return
}
