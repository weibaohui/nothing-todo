use axum::{
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::adapters::parse_executor_type;
use crate::adapters::ExecutorRegistry;
use crate::db::Database;
use crate::executor_service::{ExecutionResult, run_todo_execution, run_todo_execution_with_params};
use crate::handlers::{ApiJson, AppError, AppState, ExecEvent};
use crate::models::{
    ApiResponse, DashboardStats, ExecuteRequest, ExecutionRecordsPage, ExecutionStatus, ExecutionSummary, TodoIdQuery,
};
use crate::task_manager::TaskManager;

/// 统一启动一条 Todo 执行，供手动执行、消息路由等入口复用。
pub async fn start_todo_execution(
    db: Arc<Database>,
    executor_registry: Arc<ExecutorRegistry>,
    tx: broadcast::Sender<ExecEvent>,
    task_manager: Arc<TaskManager>,
    todo_id: i64,
    message_template: String,
    req_executor: Option<String>,
    trigger_type: &str,
    params: Option<HashMap<String, String>>,
    resume_session_id: Option<String>,
    resume_message: Option<String>,
) -> Result<ExecutionResult, AppError> {
    let result = if let Some(params) = params {
        run_todo_execution_with_params(
            db.clone(),
            executor_registry.clone(),
            tx.clone(),
            todo_id,
            message_template,
            req_executor,
            trigger_type,
            task_manager.clone(),
            params,
            resume_session_id,
            resume_message,
        )
        .await
    } else {
        run_todo_execution(
            db.clone(),
            executor_registry.clone(),
            tx.clone(),
            todo_id,
            message_template,
            req_executor,
            trigger_type,
            task_manager.clone(),
            resume_session_id,
            resume_message,
        )
        .await
    };

    if result.record_id.is_none() {
        return Err(AppError::Internal("Failed to start execution".to_string()));
    }

    Ok(result)
}

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
        .await?;
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
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(ApiResponse::ok(record))
}

pub async fn get_execution_records_by_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<ApiResponse<Vec<crate::models::ExecutionRecord>>, AppError> {
    let records = state
        .db
        .get_execution_records_by_session(&session_id)
        .await?;
    Ok(ApiResponse::ok(records))
}

pub async fn execute_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ExecuteRequest>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    // Get the todo to use its prompt as fallback when message is not provided
    let todo = state.db.get_todo(req.todo_id).await?
        .ok_or_else(|| AppError::BadRequest(format!("Todo {} not found", req.todo_id)))?;

    // Fall back to todo.prompt if message is None or whitespace-only
    let message = req.message
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string())
        .unwrap_or_else(|| todo.prompt.clone());

    let result = start_todo_execution(
        state.db.clone(),
        state.executor_registry.clone(),
        state.tx.clone(),
        state.task_manager.clone(),
        req.todo_id,
        message,
        req.executor,
        "manual",
        None,
        None,
        None,
    )
    .await;
    let result = result?;
    let record_id = result.record_id.expect("record_id checked above");

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

    let record = state.db.get_execution_record(req.record_id).await?
        .ok_or(AppError::BadRequest("Execution record not found".to_string()))?;

    if record.status != ExecutionStatus::Running {
        return Err(AppError::BadRequest("Execution record is not running".to_string()));
    }

    if let Some(task_id) = &record.task_id {
        tracing::info!("Stopping execution record {} with task_id: {}", req.record_id, task_id);
        let cancelled = state.task_manager.cancel(task_id).await;
        if !cancelled {
            tracing::warn!("Task {} was not found in task manager (may have already finished)", task_id);
        }
        // 更新数据库状态为失败，保留定时刷新已写入的日志
        let logs_json = record.logs.clone();
        let _ = state.db.update_execution_record(
            req.record_id,
            crate::models::ExecutionStatus::Failed.as_str(),
            &logs_json,
            "任务已被手动停止",
            None,
            None,
        ).await;
        tracing::info!("Successfully stopped execution record {}", req.record_id);
        Ok(ApiResponse::ok(()))
    } else {
        Err(AppError::BadRequest("No task_id found for this execution record".to_string()))
    }
}

pub async fn get_running_execution_records_handler(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<crate::models::ExecutionRecord>>, AppError> {
    let records = state.db.get_running_execution_records().await?;
    Ok(ApiResponse::ok(records))
}

#[derive(Debug, Deserialize)]
pub struct ForceFailRequest {
    pub record_id: i64,
}

pub async fn force_fail_execution_handler(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ForceFailRequest>,
) -> Result<ApiResponse<()>, AppError> {
    let record = state.db.get_execution_record(req.record_id).await?
        .ok_or(AppError::BadRequest("Execution record not found".to_string()))?;

    if record.status != ExecutionStatus::Running {
        return Err(AppError::BadRequest("Execution record is not running".to_string()));
    }

    // Try to cancel in-memory task if it exists
    if let Some(task_id) = &record.task_id {
        state.task_manager.cancel(task_id).await;
    }

    state.db.force_fail_execution_record(req.record_id).await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(ApiResponse::ok(()))
}

#[derive(Debug, Deserialize)]
pub struct ResumeExecutionRequest {
    pub message: Option<String>,
}

pub async fn resume_execution_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<ResumeExecutionRequest>,
) -> Result<ApiResponse<serde_json::Value>, AppError> {
    let record = state.db.get_execution_record(id).await?
        .ok_or(AppError::NotFound)?;

    if record.status == ExecutionStatus::Running {
        return Err(AppError::BadRequest("Cannot resume a running execution".to_string()));
    }

    let executor_type = record.executor.as_deref()
        .and_then(parse_executor_type)
        .ok_or_else(|| AppError::BadRequest("Unknown executor type".to_string()))?;

    let executor = state.executor_registry.get(executor_type)
        .ok_or_else(|| AppError::Internal("Executor not found in registry".to_string()))?;

    if !executor.supports_resume() {
        return Err(AppError::BadRequest("This executor does not support resuming conversations".to_string()));
    }

    let todo_id = record.todo_id;
    let todo = state.db.get_todo(todo_id).await?
        .ok_or(AppError::NotFound)?;

    let message = req.message
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string())
        .unwrap_or_else(|| todo.prompt.clone());

    let resume_message = req.message
        .as_ref()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .map(|m| m.to_string());

    let resume_session_id = record.session_id
        .or(record.task_id)
        .ok_or_else(|| AppError::BadRequest("No session_id found for this execution record".to_string()))?;

    let result = start_todo_execution(
        state.db.clone(),
        state.executor_registry.clone(),
        state.tx.clone(),
        state.task_manager.clone(),
        todo_id,
        message,
        record.executor.clone(),
        "manual",
        None,
        Some(resume_session_id),
        resume_message,
    )
    .await?;
    let record_id = result.record_id.expect("record_id checked above");

    Ok(ApiResponse::ok(serde_json::json!({ "task_id": result.task_id, "record_id": record_id })))
}

pub async fn get_execution_summary(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<ExecutionSummary>, AppError> {
    Ok(ApiResponse::ok(
        state.db.get_execution_summary(id).await?,
    ))
}

pub async fn get_dashboard_stats(
    State(state): State<AppState>,
) -> Result<ApiResponse<DashboardStats>, AppError> {
    Ok(ApiResponse::ok(
        state.db.get_dashboard_stats().await?,
    ))
}

pub async fn get_running_todos(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<crate::models::Todo>>, AppError> {
    let running_todos = state.db.get_running_todos().await?;
    Ok(ApiResponse::ok(running_todos))
}
