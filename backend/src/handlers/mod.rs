use axum::{
    Router,
    routing::{get, post, put, delete},
    response::{Html, IntoResponse, Response, Json},
    extract::{State, Path, Query, WebSocketUpgrade},
    http::StatusCode,
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::adapters::ExecutorRegistry;
use crate::Assets;
use crate::db::Database;
use crate::executor_service::run_todo_execution;
use crate::models::{
    CreateTodoRequest, UpdateTodoRequest, UpdateTagsRequest, CreateTagRequest, ExecuteRequest, TodoIdQuery,
    UpdateSchedulerRequest,
    Todo, Tag, ExecutionRecord, ExecutionSummary, ParsedLogEntry,
};
use crate::scheduler::TodoScheduler;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub executor_registry: Arc<ExecutorRegistry>,
    pub tx: broadcast::Sender<ExecEvent>,
    pub scheduler: Arc<TodoScheduler>,
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

    // Save tag associations
    for tag_id in &req.tag_ids {
        state.db.add_todo_tag(id, *tag_id);
    }

    Json(Todo {
        id,
        title: req.title,
        description: req.description,
        status: "pending".to_string(),
        created_at: now.clone(),
        updated_at: now,
        tag_ids: req.tag_ids.clone(),
        executor: Some("claudecode".to_string()),
        scheduler_enabled: false,
        scheduler_config: None,
        task_id: None,
    })
}

pub async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodoRequest>,
) -> Json<Todo> {
    state.db.update_todo_full(
        id,
        &req.title,
        &req.description,
        &req.status,
        req.executor.as_deref(),
        req.scheduler_enabled,
        req.scheduler_config.as_deref(),
    );

    let todo = state.db.get_todo(id).unwrap();
    Json(todo)
}

pub async fn update_todo_tags(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTagsRequest>,
) -> StatusCode {
    state.db.set_todo_tags(id, &req.tag_ids);
    StatusCode::OK
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
    let task_id = run_todo_execution(
        state.db.clone(),
        state.executor_registry.clone(),
        state.tx.clone(),
        req.todo_id,
        req.message,
        req.executor,
    ).await;

    Json(serde_json::json!({ "status": "started", "task_id": task_id }))
}

// Scheduler handlers
#[axum::debug_handler]
pub async fn update_scheduler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSchedulerRequest>,
) -> Json<Todo> {
    let todo = state.db.get_todo(id);
    let old_task_id = todo.as_ref().and_then(|t| t.task_id.clone());

    // Remove old scheduled task if exists
    if let Some(ref task_id_str) = old_task_id {
        if let Ok(uuid) = uuid::Uuid::parse_str(task_id_str) {
            let _ = state.scheduler.remove_task(uuid).await;
        }
    }

    let new_task_id = if req.scheduler_enabled {
        if let Some(ref config) = req.scheduler_config {
            match state.scheduler.add_task(
                state.db.clone(),
                state.executor_registry.clone(),
                state.tx.clone(),
                id,
                config.clone(),
            ).await {
                Ok(uuid) => Some(uuid.to_string()),
                Err(e) => {
                    log::error!("Failed to add scheduled task for todo {}: {}", id, e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    state.db.update_todo_scheduler(id, req.scheduler_enabled, req.scheduler_config.as_deref(), new_task_id.as_deref());

    let todo = state.db.get_todo(id).unwrap();
    Json(todo)
}

pub async fn get_scheduler_todos(
    State(state): State<AppState>,
) -> Json<Vec<Todo>> {
    Json(state.db.get_scheduler_todos())
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
pub fn create_app(db: Arc<Database>, executor_registry: Arc<ExecutorRegistry>, tx: broadcast::Sender<ExecEvent>, scheduler: Arc<TodoScheduler>) -> Router {
    let state = AppState { db, executor_registry, tx, scheduler };

    Router::new()
        .route("/", get(index_handler))
        .route("/xyz/todos", get(get_todos))
        .route("/xyz/todos", post(create_todo))
        .route("/xyz/todos/{id}/force-status", put(force_update_todo_status))
        .route("/xyz/todos/{id}/tags", put(update_todo_tags))
        .route("/xyz/todos/{id}/summary", get(get_execution_summary))
        .route("/xyz/todos/{id}/scheduler", put(update_scheduler))
        .route("/xyz/todos/{id}", put(update_todo))
        .route("/xyz/todos/{id}", delete(delete_todo))
        .route("/xyz/tags", get(get_tags))
        .route("/xyz/tags", post(create_tag))
        .route("/xyz/tags/{id}", delete(delete_tag))
        .route("/xyz/execution-records", get(get_execution_records))
        .route("/xyz/execute", post(execute_handler))
        .route("/xyz/events", get(events_handler))
        .route("/xyz/scheduler/todos", get(get_scheduler_todos))
        .route("/assets/{*path}", get(static_handler))
        .with_state(state)
}
