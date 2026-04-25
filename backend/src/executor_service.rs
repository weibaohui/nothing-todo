use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use parking_lot::Mutex;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::adapters::{ExecutorRegistry, get_timestamp};
use crate::db::Database;
use crate::handlers::ExecEvent;
use crate::models::{ParsedLogEntry, ExecutorType};

fn parse_executor_type(executor: &str) -> ExecutorType {
    match executor.to_lowercase().as_str() {
        "claudecode" | "claude" => ExecutorType::Claudecode,
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
) -> String {
    let task_id = Uuid::new_v4().to_string();

    // Get todo to read stored executor
    let todo = db.get_todo(todo_id);
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
    db.update_todo_executor(todo_id, &executor_str);

    // Create execution record
    let command = format!("{} {}", executable_path, command_args.join(" "));
    let record_id = db.create_execution_record(todo_id, &command, &executor_str);

    // Update todo status
    db.update_todo_status(todo_id, "running");

    let task_id_return = task_id.clone();
    let db_clone = db.clone();
    let tx_clone = tx.clone();
    let executor_spawn = executor.clone();
    let message_clone = message.clone();

    tokio::spawn(async move {
        let _ = tx_clone.send(ExecEvent::Started { task_id: task_id.clone() });

        let entry = ParsedLogEntry {
            timestamp: get_timestamp(),
            log_type: "info".to_string(),
            content: format!("Starting {} with message: {}", executor_spawn.executor_type(), message_clone),
            usage: None,
        };
        let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });

        let mut child = match Command::new(&executable_path)
            .args(&command_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let entry = ParsedLogEntry {
                    timestamp: get_timestamp(),
                    log_type: "error".to_string(),
                    content: format!("Failed to spawn executor: {}", e),
                    usage: None,
                };
                let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });
                let _ = tx_clone.send(ExecEvent::Finished { task_id: task_id.clone(), success: false, result: None });
                return;
            }
        };

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let logs = Arc::new(Mutex::new(Vec::<ParsedLogEntry>::new()));
        let logs_for_db = logs.clone();
        let logs_for_result = logs.clone();

        let executor_for_parse = executor_spawn.clone();

        // Process stdout
        if let Some(stdout_reader) = stdout_handle {
            let tx_clone = tx.clone();
            let tid = task_id.clone();
            let db_clone2 = db_clone.clone();
            let rid = record_id;
            let executor_clone = executor_for_parse.clone();

            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(parsed) = executor_clone.parse_output_line(&line) {
                        logs_for_db.lock().push(parsed.clone());
                        let _ = tx_clone.send(ExecEvent::Output { task_id: tid.clone(), entry: parsed });

                        let logs_json = serde_json::to_string(&*logs_for_db.lock()).unwrap_or_default();
                        db_clone2.update_execution_record(rid, "running", &logs_json, "");
                    }
                }
            });
        }

        // Capture stderr
        let stderr_tx = tx.clone();
        let stderr_tid = task_id.clone();
        let logs_for_stderr = logs.clone();
        let stderr_task = if let Some(stderr_reader) = stderr_handle {
            Some(tokio::spawn(async move {
                let mut reader = BufReader::new(stderr_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let entry = ParsedLogEntry {
                        timestamp: get_timestamp(),
                        log_type: "stderr".to_string(),
                        content: line.clone(),
                        usage: None,
                    };
                    logs_for_stderr.lock().push(entry.clone());
                    let _ = stderr_tx.send(ExecEvent::Output { task_id: stderr_tid.clone(), entry });
                }
            }))
        } else {
            None
        };

        let status = child.wait().await;

        if let Some(handle) = stderr_task {
            let _ = handle.await;
        }

        let exit_code = status.as_ref().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        let success = executor_spawn.check_success(exit_code);

        let all_logs_snapshot = logs_for_result.lock().clone();
        let result_str = executor_spawn.get_final_result(&all_logs_snapshot).unwrap_or_default();

        let final_status = if success { "success" } else { "failed" };
        let logs_json = serde_json::to_string(&all_logs_snapshot).unwrap_or_default();
        let usage = executor_spawn.get_usage(&all_logs_snapshot);
        if let Some(u) = usage {
            db_clone.update_execution_record_with_usage(record_id, final_status, &logs_json, &result_str, &u);
        } else {
            db_clone.update_execution_record(record_id, final_status, &logs_json, &result_str);
        }

        if let Some(model) = executor_spawn.get_model() {
            db_clone.update_execution_record_with_model(record_id, &model);
        }

        db_clone.update_todo_status(todo_id, final_status);

        let entry = ParsedLogEntry {
            timestamp: get_timestamp(),
            log_type: if success { "info".to_string() } else { "error".to_string() },
            content: format!("Executor finished with exit_code: {}, result: {}", exit_code, result_str),
            usage: None,
        };
        let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });

        let _ = tx_clone.send(ExecEvent::Finished { task_id: task_id.clone(), success, result: Some(result_str) });
    });

    task_id_return
}
