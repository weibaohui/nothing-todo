use axum::{
    extract::{Path, Query, State},
};
use serde::Deserialize;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

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
    pub record_id: i64,
}

pub async fn stop_execution_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<StopExecutionRequest>,
) -> Result<ApiResponse<()>, AppError> {
    use crate::db::entity::execution_records;

    tracing::info!("Stopping execution record: {}", req.record_id);

    // 根据 record_id 查询执行记录
    let record = execution_records::Entity::find()
        .filter(execution_records::Column::Id.eq(req.record_id))
        .one(&state.db.conn)
        .await
        .map_err(|_| AppError::BadRequest("Failed to query execution record".to_string()))?;

    let record = match record {
        Some(r) => r,
        None => return Err(AppError::BadRequest("Execution record not found".to_string())),
    };

    // 检查记录状态是否为 running
    if record.status.as_deref() != Some("running") {
        return Err(AppError::BadRequest("Execution record is not running".to_string()));
    }

    // 获取 task_id 并通过 TaskManager 取消
    if let Some(task_id) = &record.task_id {
        tracing::info!("Stopping execution record {} with task_id: {}", req.record_id, task_id);

        // 通过 TaskManager 取消任务，这会触发 cancel_rx.recv() 分支
        // 该分支会：kill_process_group + child.kill + 更新 todo 状态 + 发 Finished 事件
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
