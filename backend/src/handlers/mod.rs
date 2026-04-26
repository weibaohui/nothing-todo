use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::adapters::ExecutorRegistry;
use crate::Assets;
use crate::db::Database;
use crate::models::ParsedLogEntry;
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
    Started {
        task_id: String,
        todo_id: i64,
        todo_title: String,
    },
    Output {
        task_id: String,
        entry: ParsedLogEntry,
    },
    Finished {
        task_id: String,
        todo_id: i64,
        success: bool,
        result: Option<String>,
    },
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
            Self::NotFound => (StatusCode::NOT_FOUND, crate::models::codes::NOT_FOUND, "Not found".to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, crate::models::codes::BAD_REQUEST, msg.clone()),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, crate::models::codes::INTERNAL, msg.clone()),
        };
        let body = axum::Json(crate::models::ApiResponse::<()>::err(code, &message));
        (status, body).into_response()
    }
}

mod todo;
mod tag;
mod execution;
mod scheduler;

// WebSocket handler
pub async fn events_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut ws| async move {
        let mut rx = state.tx.subscribe();
        let _ = ws
            .send(axum::extract::ws::Message::Text("Connected".into()))
            .await;

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    if json.is_empty() {
                        continue;
                    }
                    if ws
                        .send(axum::extract::ws::Message::Text(json.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}

// Static file handler
pub async fn index_handler() -> Result<Html<String>, AppError> {
    let content = Assets::get("index.html")
        .ok_or_else(|| AppError::Internal("index.html not found in embedded assets".to_string()))?;
    Ok(Html(String::from_utf8_lossy(&content.data).to_string()))
}

pub async fn static_handler(Path(path): axum::extract::Path<String>) -> Response {
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
        None => match Assets::get("index.html") {
            Some(content) => {
                Html(String::from_utf8_lossy(&content.data).to_string()).into_response()
            }
            None => (StatusCode::NOT_FOUND, "Not found").into_response(),
        },
    }
}

// Build router
pub fn create_app(
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
    scheduler: Arc<TodoScheduler>,
    task_manager: Arc<TaskManager>,
) -> Router {
    let state = AppState {
        db,
        executor_registry,
        tx,
        scheduler,
        task_manager,
    };

    Router::new()
        .route("/", get(index_handler))
        .route("/xyz/todos", get(todo::get_todos))
        .route("/xyz/todos", post(todo::create_todo))
        .route("/xyz/todos/{id}/force-status", put(todo::force_update_todo_status))
        .route("/xyz/todos/{id}/tags", put(todo::update_todo_tags))
        .route("/xyz/todos/{id}/summary", get(execution::get_execution_summary))
        .route("/xyz/todos/{id}/scheduler", put(scheduler::update_scheduler))
        .route("/xyz/todos/{id}", put(todo::update_todo))
        .route("/xyz/todos/{id}", delete(todo::delete_todo))
        .route("/xyz/tags", get(tag::get_tags))
        .route("/xyz/tags", post(tag::create_tag))
        .route("/xyz/tags/{id}", delete(tag::delete_tag))
        .route("/xyz/execution-records", get(execution::get_execution_records))
        .route("/xyz/execute", post(execution::execute_handler))
        .route("/xyz/execute/stop", post(execution::stop_execution_handler))
        .route("/xyz/events", get(events_handler))
        .route("/xyz/scheduler/todos", get(scheduler::get_scheduler_todos))
        .route("/assets/{*path}", get(static_handler))
        .with_state(state)
}
