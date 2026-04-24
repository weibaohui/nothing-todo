use axum::{
    Router,
    routing::{get, post, put, delete},
    response::{Html, IntoResponse, Response, Json},
    extract::{State, Path, Query, WebSocketUpgrade},
    http::StatusCode,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use parking_lot::Mutex;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::adapters::{ExecutorRegistry, get_timestamp};
use crate::Assets;
use crate::db::Database;
use crate::models::{
    CreateTodoRequest, UpdateTodoRequest, CreateTagRequest, ExecuteRequest, TodoIdQuery,
    Todo, Tag, ExecutionRecord, ExecutionSummary, ParsedLogEntry,
};

#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ExecEvent {
    Started { task_id: String },
    Output { task_id: String, entry: ParsedLogEntry },
    Finished { task_id: String, success: bool, result: Option<String> },
}

// Todo handlers
pub async fn get_todos(State(state): State<AppState>) -> Json<Vec<Todo>> {
    Json(state.db.get_todos())
}

pub async fn create_todo(State(state): State<AppState>, Json(req): Json<CreateTodoRequest>) -> Json<Todo> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = state.db.create_todo(&req.title, &req.description);
    Json(Todo {
        id,
        title: req.title,
        description: req.description,
        status: "pending".to_string(),
        created_at: now.clone(),
        updated_at: now,
    })
}

pub async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodoRequest>,
) -> Json<Todo> {
    state.db.update_todo(id, &req.title, &req.description, &req.status);

    let todo = state.db.get_todo(id).unwrap();
    Json(todo)
}

pub async fn delete_todo(State(state): State<AppState>, Path(id): Path<i64>) -> StatusCode {
    state.db.delete_todo(id);
    StatusCode::OK
}

// Tag handlers
pub async fn get_tags(State(state): State<AppState>) -> Json<Vec<Tag>> {
    Json(state.db.get_tags())
}

pub async fn create_tag(State(state): State<AppState>, Json(req): Json<CreateTagRequest>) -> Json<Tag> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let id = state.db.create_tag(&req.name, &req.color);
    Json(Tag {
        id,
        name: req.name,
        color: req.color,
        created_at: now,
    })
}

pub async fn delete_tag(State(state): State<AppState>, Path(id): Path<i64>) -> StatusCode {
    state.db.delete_tag(id);
    StatusCode::OK
}

// Execution record handlers
pub async fn get_execution_records(
    State(state): State<AppState>,
    Query(query): Query<TodoIdQuery>,
) -> Json<Vec<ExecutionRecord>> {
    Json(state.db.get_execution_records(query.todo_id))
}

// Execute handler
pub async fn execute_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> Json<serde_json::Value> {
    let tx = state.tx.clone();
    let task_id = Uuid::new_v4().to_string();
    let todo_id = req.todo_id;

    // Determine which executor to use
    let executor_type = req.executor
        .map(|e| {
            match e.to_lowercase().as_str() {
                "claudecode" | "claude" => crate::models::ExecutorType::Claudecode,
                _ => crate::models::ExecutorType::Joinai,
            }
        })
        .unwrap_or(crate::models::ExecutorType::Joinai);

    let executor = state.executor_registry.get(executor_type)
        .unwrap_or_else(|| state.executor_registry.get_default().unwrap());

    let executable_path = executor.executable_path().to_string();
    let command_args = executor.command_args(&req.message);

    // Create execution record
    let command = format!("{} {}", executable_path, command_args.join(" "));
    let executor_str = executor.executor_type().to_string();
    let record_id = state.db.create_execution_record(todo_id, &command, &executor_str);

    // Update todo status
    state.db.update_todo_status(todo_id, "running");

    // Spawn execution
    let task_id_return = task_id.clone();
    let db = state.db.clone();
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let _ = tx_clone.send(ExecEvent::Started { task_id: task_id.clone() });

        let entry = ParsedLogEntry {
            timestamp: get_timestamp(),
            log_type: "info".to_string(),
            content: format!("Starting {} with message: {}", executor.executor_type(), req.message),
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

        // Clone executor for async task
        let executor_for_parse = executor.clone();

        // Process stdout
        if let Some(stdout_reader) = stdout_handle {
            let tx_clone = tx.clone();
            let tid = task_id.clone();
            let db_clone = db.clone();
            let rid = record_id;
            let executor_clone = executor_for_parse.clone();

            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout_reader).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if let Some(parsed) = executor_clone.parse_output_line(&line) {
                        logs_for_db.lock().push(parsed.clone());
                        let _ = tx_clone.send(ExecEvent::Output { task_id: tid.clone(), entry: parsed });

                        // Update logs in database periodically
                        let logs_json = serde_json::to_string(&*logs_for_db.lock()).unwrap_or_default();
                        db_clone.update_execution_record(rid, "running", &logs_json, "");
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

        // Wait for stderr task to complete
        if let Some(handle) = stderr_task {
            let _ = handle.await;
        }

        let exit_code = status.as_ref().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        let success = executor.check_success(exit_code);

        // Extract final result using adapter's method
        let all_logs_snapshot = logs_for_result.lock().clone();
        let result_str = executor.get_final_result(&all_logs_snapshot).unwrap_or_default();

        // Update database with usage info
        let final_status = if success { "success" } else { "failed" };
        let logs_json = serde_json::to_string(&all_logs_snapshot).unwrap_or_default();
        let usage = executor.get_usage(&all_logs_snapshot);
        if let Some(u) = usage {
            db.update_execution_record_with_usage(record_id, final_status, &logs_json, &result_str, &u);
        } else {
            db.update_execution_record(record_id, final_status, &logs_json, &result_str);
        }

        // Update model if found
        if let Some(model) = executor.get_model() {
            db.update_execution_record_with_model(record_id, &model);
        }

        db.update_todo_status(todo_id, final_status);

        let entry = ParsedLogEntry {
            timestamp: get_timestamp(),
            log_type: if success { "info".to_string() } else { "error".to_string() },
            content: format!("Executor finished with exit_code: {}, result: {}", exit_code, result_str),
            usage: None,
        };
        let _ = tx_clone.send(ExecEvent::Output { task_id: task_id.clone(), entry });

        let _ = tx_clone.send(ExecEvent::Finished { task_id: task_id.clone(), success, result: Some(result_str) });
    });

    Json(serde_json::json!({ "status": "started", "task_id": task_id_return }))
}

// WebSocket handler
pub async fn events_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut ws| async move {
        let mut rx = state.tx.subscribe();
        let _ = ws.send(axum::extract::ws::Message::Text("Connected".into())).await;

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap();
                    if ws.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

// Static file handler
pub async fn index_handler() -> Html<String> {
    let content = Assets::get("index.html").unwrap();
    Html(String::from_utf8_lossy(&content.data).to_string())
}

pub async fn static_handler(Path(path): Path<String>) -> Response {
    let path = path.trim_start_matches('/');
    let full_path = if path.is_empty() {
        "index.html".to_string()
    } else {
        format!("assets/{}", path)
    };

    match Assets::get(&full_path) {
        Some(content) => {
            let mime = if path.ends_with(".js") {
                "application/javascript"
            } else if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".html") {
                "text/html"
            } else {
                "application/octet-stream"
            };
            let body = String::from_utf8_lossy(&content.data).to_string();
            ([("Content-Type", mime)], body).into_response()
        }
        None => {
            let content = Assets::get("index.html").unwrap();
            Html(String::from_utf8_lossy(&content.data).to_string()).into_response()
        }
    }
}

// Force update todo status (for recovering from crashed executions)
pub async fn force_update_todo_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodoRequest>,
) -> Json<Todo> {
    state.db.force_update_todo_status(id, &req.status);
    let todo = state.db.get_todo(id).unwrap();
    Json(todo)
}

// Get execution summary for a todo
pub async fn get_execution_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Json<ExecutionSummary> {
    Json(state.db.get_execution_summary(id))
}

// Build router
pub fn create_app(db: Arc<Database>, executor_registry: Arc<ExecutorRegistry>, tx: broadcast::Sender<ExecEvent>) -> Router {
    let state = AppState { db, executor_registry, tx };

    Router::new()
        .route("/", get(index_handler))
        .route("/xyz/todos", get(get_todos))
        .route("/xyz/todos", post(create_todo))
        .route("/xyz/todos/{id}", put(update_todo))
        .route("/xyz/todos/{id}", delete(delete_todo))
        .route("/xyz/todos/{id}/force-status", put(force_update_todo_status))
        .route("/xyz/todos/{id}/summary", get(get_execution_summary))
        .route("/xyz/tags", get(get_tags))
        .route("/xyz/tags", post(create_tag))
        .route("/xyz/tags/{id}", delete(delete_tag))
        .route("/xyz/execution-records", get(get_execution_records))
        .route("/xyz/execute", post(execute_handler))
        .route("/xyz/events", get(events_handler))
        .route("/assets/{*path}", get(static_handler))
        .with_state(state)
}
