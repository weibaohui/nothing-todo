use axum::{
    Router,
    extract::{FromRequest, Path, Request, State, WebSocketUpgrade},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
};
use std::time::Duration;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{CorsLayer, Any};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use axum::extract::DefaultBodyLimit;
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

impl AppState {
    /// 根据 id 获取 todo，不存在时返回 NotFound 错误
    pub async fn require_todo(&self, id: i64) -> Result<crate::models::Todo, AppError> {
        self.db.get_todo(id).await.ok_or(AppError::NotFound)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ExecEvent {
    Started {
        task_id: String,
        todo_id: i64,
        todo_title: String,
        executor: String,
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

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        match &err {
            sea_orm::DbErr::RecordNotFound(_) => AppError::NotFound,
            _ => AppError::Internal(err.to_string()),
        }
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::BadRequest(s)
    }
}

impl<T: Serialize> IntoResponse for crate::models::ApiResponse<T> {
    fn into_response(self) -> Response {
        axum::Json(self).into_response()
    }
}

/// 自定义 JSON 提取器，将解析错误转换为统一的 ApiResponse 错误格式
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Json::<T>::from_request(req, state).await {
            Ok(axum::extract::Json(value)) => Ok(ApiJson(value)),
            Err(rejection) => Err(AppError::BadRequest(rejection.to_string())),
        }
    }
}

mod todo;
mod tag;
mod execution;
mod scheduler;
mod backup;

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
            } else if path.ends_with(".woff2") {
                "font/woff2"
            } else if path.ends_with(".woff") {
                "font/woff"
            } else if path.ends_with(".ttf") {
                "font/ttf"
            } else if path.ends_with(".eot") {
                "application/vnd.ms-fontobject"
            } else if path.ends_with(".svg") {
                "image/svg+xml"
            } else if path.ends_with(".png") {
                "image/png"
            } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                "image/jpeg"
            } else if path.ends_with(".ico") {
                "image/x-icon"
            } else if path.ends_with(".json") {
                "application/json"
            } else if path.ends_with(".webp") {
                "image/webp"
            } else {
                "application/octet-stream"
            };
            ([(header::CONTENT_TYPE, mime)], content.data.to_vec()).into_response()
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
        .route("/xyz/todos", get(todo::get_todos).post(todo::create_todo))
        .route("/xyz/todos/{id}/force-status", put(todo::force_update_todo_status))
        .route("/xyz/todos/{id}/tags", put(todo::update_todo_tags))
        .route("/xyz/todos/{id}/summary", get(execution::get_execution_summary))
        .route("/xyz/todos/{id}/scheduler", put(scheduler::update_scheduler))
        .route("/xyz/todos/{id}", get(todo::get_todo).put(todo::update_todo).delete(todo::delete_todo))
        .route("/xyz/tags", get(tag::get_tags).post(tag::create_tag))
        .route("/xyz/tags/{id}", delete(tag::delete_tag))
        .route("/xyz/execution-records", get(execution::get_execution_records))
        .route("/xyz/execution-records/{id}", get(execution::get_execution_record))
        .route("/xyz/dashboard-stats", get(execution::get_dashboard_stats))
        .route("/xyz/execute", post(execution::execute_handler))
        .route("/xyz/execute/stop", post(execution::stop_execution_handler))
        .route("/xyz/running-todos", get(execution::get_running_todos))
        .route("/xyz/events", get(events_handler))
        .route("/xyz/scheduler/todos", get(scheduler::get_scheduler_todos))
        .route("/xyz/backup/export", get(backup::export_backup))
        .route("/xyz/backup/import", post(backup::import_backup))
        .route("/assets/{*path}", get(static_handler))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:8088".parse().unwrap(),
                    "http://127.0.0.1:8088".parse().unwrap(),
                    "http://localhost:5173".parse().unwrap(),
                    "http://127.0.0.1:5173".parse().unwrap(),
                ])
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)))
        .with_state(state)
}
