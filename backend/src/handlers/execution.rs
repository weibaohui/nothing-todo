use axum::{
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::executor_service::run_todo_execution;
use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{
    ApiResponse, DashboardStats, ExecuteRequest, ExecutionRecordsPage, ExecutionSummary, TodoIdQuery,
};

pub async fn get_execution_records(
    State(state): State<AppState>,
    Query(query): Query<TodoIdQuery>,
) -> Result<ApiResponse<ExecutionRecordsPage>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(10).max(1).min(100);
    let offset = (page - 1) * limit;
    let (records, total) = state
        .db
        .get_execution_records(query.todo_id, limit, offset)
        .await;
    Ok(ApiResponse::ok(ExecutionRecordsPage {
        records,
        total,
        page,
        limit,
    }))
}

pub async fn get_execution_record(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<crate::models::ExecutionRecord>, AppError> {
    let record = state
        .db
        .get_execution_record(id)
        .await
        .ok_or(AppError::NotFound)?;
    Ok(ApiResponse::ok(record))
}

pub async fn execute_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ExecuteRequest>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    // Get the todo to use its prompt as fallback when message is not provided
    let todo = state.db.get_todo(req.todo_id).await
        .ok_or_else(|| AppError::BadRequest(format!("Todo {} not found", req.todo_id)))?;

    // Fall back to todo.prompt if message is None or whitespace-only
    let message = req.message
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string())
        .unwrap_or_else(|| todo.prompt.clone());

    let result = run_todo_execution(
        state.db.clone(),
        state.executor_registry.clone(),
        state.tx.clone(),
        req.todo_id,
        message,
        req.executor,
        "manual",
        state.task_manager.clone(),
    )
    .await;

    // If record_id is None, execution failed to start
    let record_id = result.record_id
        .ok_or_else(|| AppError::Internal("Failed to start execution".to_string()))?;

    Ok(ApiResponse::ok(serde_json::json!({ "task_id": result.task_id, "record_id": record_id })))
}

#[derive(Debug, Deserialize)]
pub struct StopExecutionRequest {
    pub record_id: i64,
}

pub async fn stop_execution_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<StopExecutionRequest>,
) -> Result<ApiResponse<()>, AppError> {
    tracing::info!("Stopping execution record: {}", req.record_id);

    let record = state.db.get_execution_record(req.record_id).await
        .ok_or(AppError::BadRequest("Execution record not found".to_string()))?;

    if record.status != "running" {
        return Err(AppError::BadRequest("Execution record is not running".to_string()));
    }

    if let Some(task_id) = &record.task_id {
        tracing::info!("Stopping execution record {} with task_id: {}", req.record_id, task_id);
        state.task_manager.cancel(task_id).await;
        tracing::info!("Successfully stopped execution record {}", req.record_id);
        Ok(ApiResponse::ok(()))
    } else {
        Err(AppError::BadRequest("No task_id found for this execution record".to_string()))
    }
}

pub async fn get_execution_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<ExecutionSummary>, AppError> {
    Ok(ApiResponse::ok(
        state.db.get_execution_summary(id).await,
    ))
}

pub async fn get_dashboard_stats(
    State(state): State<AppState>,
) -> Result<ApiResponse<DashboardStats>, AppError> {
    Ok(ApiResponse::ok(
        state.db.get_dashboard_stats().await,
    ))
}

pub async fn get_running_todos(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<crate::models::Todo>>, AppError> {
    let running_todos = state.db.get_running_todos().await;
    Ok(ApiResponse::ok(running_todos))
}
