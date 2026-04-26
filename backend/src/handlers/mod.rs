use axum::{
    Router,
    routing::{get, post, put, delete},
    response::{Html, IntoResponse, Response, Json},
    extract::{State, Path, Query, WebSocketUpgrade},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::adapters::ExecutorRegistry;
use crate::Assets;
use crate::db::Database;
use crate::executor_service::run_todo_execution;
use crate::models::{
    ApiResponse, codes,
    CreateTodoRequest, UpdateTodoRequest, UpdateTagsRequest, CreateTagRequest, ExecuteRequest, TodoIdQuery,
    UpdateSchedulerRequest, ExecutionRecordsPage,
    Todo, Tag, ExecutionSummary, ParsedLogEntry,
};
use crate::scheduler::TodoScheduler;
use crate::task_manager::TaskManager;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub executor_registry: Arc<ExecutorRegistry>,
    pub tx: broadcast::Sender<ExecEvent>,
    pub scheduler: Arc<TodoScheduler>,
    pub task_manager: Arc<TaskManager>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ExecEvent {
    Started { task_id: String, todo_id: i64, todo_title: String },
    Output { task_id: String, entry: ParsedLogEntry },
    Finished { task_id: String, todo_id: i64, success: bool, result: Option<String> },
}

#[derive(Debug)]
pub enum AppError {
    NotFound,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, codes::NOT_FOUND, "Not found".to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, codes::BAD_REQUEST, msg.clone()),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, codes::INTERNAL, msg.clone()),
        };
        let body = Json(ApiResponse::<()>::err(code, &message));
        (status, body).into_response()
    }
}

// Todo handlers
pub async fn get_todos(State(state): State<AppState>) -> Result<Json<ApiResponse<Vec<Todo>>>, AppError> {
    Ok(Json(ApiResponse::ok(state.db.get_todos().await)))
}

pub async fn create_todo(State(state): State<AppState>, Json(req): Json<CreateTodoRequest>) -> Result<Json<ApiResponse<Todo>>, AppError> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let prompt = if req.prompt.is_empty() { req.title.clone() } else { req.prompt.clone() };
    let id = state.db.create_todo(&req.title, &prompt).await;

    for tag_id in &req.tag_ids {
        state.db.add_todo_tag(id, *tag_id).await;
    }

    Ok(Json(ApiResponse::ok(Todo {
        id,
        title: req.title,
        prompt,
        status: crate::models::TodoStatus::Pending,
        created_at: now.clone(),
        updated_at: now,
        tag_ids: req.tag_ids.clone(),
        executor: Some("claudecode".to_string()),
        scheduler_enabled: false,
        scheduler_config: None,
        scheduler_next_run_at: None,
        task_id: None,
    })))
}

pub async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodoRequest>,
) -> Result<Json<ApiResponse<Todo>>, AppError> {
    let prompt = if req.prompt.is_empty() { req.title.clone() } else { req.prompt.clone() };
    state.db.update_todo_full(
        id, &req.title, &prompt, req.status,
        req.executor.as_deref(), req.scheduler_enabled, req.scheduler_config.as_deref(),
    ).await;

    match state.db.get_todo(id).await {
        Some(todo) => Ok(Json(ApiResponse::ok(todo))),
        None => Err(AppError::NotFound),
    }
}

pub async fn update_todo_tags(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTagsRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.db.set_todo_tags(id, &req.tag_ids).await;
    Ok(Json(ApiResponse::ok(())))
}

pub async fn delete_todo(State(state): State<AppState>, Path(id): Path<i64>) -> Result<Json<ApiResponse<()>>, AppError> {
    state.db.delete_todo(id).await;
    Ok(Json(ApiResponse::ok(())))
}

// Tag handlers
pub async fn get_tags(State(state): State<AppState>) -> Result<Json<ApiResponse<Vec<Tag>>>, AppError> {
    Ok(Json(ApiResponse::ok(state.db.get_tags().await)))
}

pub async fn create_tag(State(state): State<AppState>, Json(req): Json<CreateTagRequest>) -> Result<Json<ApiResponse<Tag>>, AppError> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let id = state.db.create_tag(&req.name, &req.color).await;
    Ok(Json(ApiResponse::ok(Tag {
        id,
        name: req.name,
        color: req.color,
        created_at: now,
    })))
}

pub async fn delete_tag(State(state): State<AppState>, Path(id): Path<i64>) -> Result<Json<ApiResponse<()>>, AppError> {
    state.db.delete_tag(id).await;
    Ok(Json(ApiResponse::ok(())))
}

// Execution record handlers
pub async fn get_execution_records(
    State(state): State<AppState>,
    Query(query): Query<TodoIdQuery>,
) -> Result<Json<ApiResponse<ExecutionRecordsPage>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(10).max(1).min(100);
    let offset = (page - 1) * limit;
    let (records, total) = state.db.get_execution_records(query.todo_id, limit, offset).await;
    Ok(Json(ApiResponse::ok(ExecutionRecordsPage {
        records,
        total,
        page,
        limit,
    })))
}

// Execute handler
pub async fn execute_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let task_id = run_todo_execution(
        state.db.clone(),
        state.executor_registry.clone(),
        state.tx.clone(),
        req.todo_id, req.message, req.executor,
        "manual",
        state.task_manager.clone(),
    ).await;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "task_id": task_id }))))
}

// Stop execution handler
#[derive(Debug, Deserialize)]
pub struct StopExecutionRequest {
    pub task_id: String,
}

pub async fn stop_execution_handler(
    State(state): State<AppState>,
    Json(req): Json<StopExecutionRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let cancelled = state.task_manager.cancel(&req.task_id).await;
    if cancelled {
        Ok(Json(ApiResponse::ok(())))
    } else {
        Err(AppError::BadRequest("Task not found or already finished".to_string()))
    }
}

// Scheduler handlers
pub async fn update_scheduler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSchedulerRequest>,
) -> Result<Json<ApiResponse<Todo>>, AppError> {
    if req.scheduler_enabled {
        if let Some(ref config) = req.scheduler_config {
            if let Err(e) = state.scheduler.upsert_task(
                state.db.clone(), state.executor_registry.clone(), state.tx.clone(),
                id, config.clone(), state.task_manager.clone(),
            ).await {
                tracing::error!("Failed to upsert scheduled task for todo {}: {}", id, e);
            }
        } else {
            state.scheduler.remove_task_for_todo(id).await;
        }
    } else {
        state.scheduler.remove_task_for_todo(id).await;
    }

    state.db.update_todo_scheduler(id, req.scheduler_enabled, req.scheduler_config.as_deref()).await;

    match state.db.get_todo(id).await {
        Some(todo) => Ok(Json(ApiResponse::ok(todo))),
        None => Err(AppError::NotFound),
    }
}

pub async fn get_scheduler_todos(State(state): State<AppState>) -> Result<Json<ApiResponse<Vec<Todo>>>, AppError> {
    Ok(Json(ApiResponse::ok(state.db.get_scheduler_todos().await)))
}

// Force update todo status (for recovering from crashed executions)
pub async fn force_update_todo_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodoRequest>,
) -> Result<Json<ApiResponse<Todo>>, AppError> {
    state.db.force_update_todo_status(id, req.status).await;
    match state.db.get_todo(id).await {
        Some(todo) => Ok(Json(ApiResponse::ok(todo))),
        None => Err(AppError::NotFound),
    }
}

pub async fn get_execution_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ExecutionSummary>>, AppError> {
    Ok(Json(ApiResponse::ok(state.db.get_execution_summary(id).await)))
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

// Build router
pub fn create_app(db: Arc<Database>, executor_registry: Arc<ExecutorRegistry>, tx: broadcast::Sender<ExecEvent>, scheduler: Arc<TodoScheduler>, task_manager: Arc<TaskManager>) -> Router {
    let state = AppState { db, executor_registry, tx, scheduler, task_manager };

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
        .route("/xyz/execute/stop", post(stop_execution_handler))
        .route("/xyz/events", get(events_handler))
        .route("/xyz/scheduler/todos", get(get_scheduler_todos))
        .route("/assets/{*path}", get(static_handler))
        .with_state(state)
}
