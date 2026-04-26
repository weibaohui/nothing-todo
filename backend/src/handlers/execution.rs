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

pub async fn execute_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ExecuteRequest>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
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

    Ok(ApiResponse::ok(serde_json::json!({ "task_id": task_id })))
}

#[derive(Debug, Deserialize)]
pub struct StopExecutionRequest {
    pub task_id: String,
}

pub async fn stop_execution_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<StopExecutionRequest>,
) -> Result<ApiResponse<()>, AppError> {
    let cancelled = state.task_manager.cancel(&req.task_id).await;

    if cancelled {
        // 成功取消任务
        return Ok(ApiResponse::ok(()));
    }

    // 任务在TaskManager中找不到，可能是已经完成但todo状态没更新
    // 查找task_id对应的todo
    if let Some(todo) = state.db.get_todo_by_task_id(&req.task_id).await {
        if todo.task_id.as_ref() == Some(&req.task_id) {
            // todo的task_id匹配，说明这个todo还在运行状态
            // 更新todo状态为failed，清除task_id
            state.db.update_todo_status(todo.id, crate::models::TodoStatus::Failed).await;
            state.db.update_todo_task_id(todo.id, None).await;

            // 同时更新对应的执行记录为失败状态
            state.db.mark_execution_records_as_failed(todo.id).await;

            return Ok(ApiResponse::ok(()));
        }
    }

    // 找不到对应的todo或task_id不匹配，返回错误
    Err(AppError::BadRequest(
        "Task not found or already finished".to_string(),
    ))
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
