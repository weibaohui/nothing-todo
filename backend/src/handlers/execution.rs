use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;

use crate::executor_service::run_todo_execution;
use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{
    ApiResponse, ExecuteRequest, ExecutionRecordsPage, ExecutionSummary, TodoIdQuery,
};

pub async fn get_execution_records(
    State(state): State<AppState>,
    Query(query): Query<TodoIdQuery>,
) -> Result<Json<ApiResponse<ExecutionRecordsPage>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(10).max(1).min(100);
    let offset = (page - 1) * limit;
    let (records, total) = state
        .db
        .get_execution_records(query.todo_id, limit, offset)
        .await;
    Ok(Json(ApiResponse::ok(ExecutionRecordsPage {
        records,
        total,
        page,
        limit,
    })))
}

pub async fn execute_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ExecuteRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let task_id = run_todo_execution(
        state.db.clone(),
        state.executor_registry.clone(),
        state.tx.clone(),
        req.todo_id,
        req.message,
        req.executor,
        "manual",
        state.task_manager.clone(),
    )
    .await;

    Ok(Json(ApiResponse::ok(serde_json::json!({ "task_id": task_id }))))
}

#[derive(Debug, Deserialize)]
pub struct StopExecutionRequest {
    pub task_id: String,
}

pub async fn stop_execution_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<StopExecutionRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let cancelled = state.task_manager.cancel(&req.task_id).await;
    if cancelled {
        Ok(Json(ApiResponse::ok(())))
    } else {
        Err(AppError::BadRequest(
            "Task not found or already finished".to_string(),
        ))
    }
}

pub async fn get_execution_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ExecutionSummary>>, AppError> {
    Ok(Json(ApiResponse::ok(
        state.db.get_execution_summary(id).await,
    )))
}
