use axum::{
    extract::{Path, State},
};
use cron::Schedule;
use std::str::FromStr;

use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, CreateTodoRequest, Todo, UpdateTagsRequest, UpdateTodoRequest, utc_timestamp};

/// Validate cron expression, return helpful error for invalid ones
fn validate_cron_expression(expr: &str) -> Result<(), String> {
    Schedule::from_str(expr)
        .map(|_| ())
        .map_err(|_| {
            format!(
                "Invalid cron expression: '{}'. AI must convert natural language to valid cron format. \
                Expected format with 6 fields (seconds + 5 standard): '0 */12 * * * *' (every 12 min), \
                '0 0 * * * *' (every minute), '0 0 9 * * *' (daily at 9am). See https://crontab.guru/",
                expr
            )
        })
}

pub async fn get_todos(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<Todo>>, AppError> {
    Ok(ApiResponse::ok(state.db.get_todos().await))
}

pub async fn get_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<Todo>, AppError> {
    let todo = state.require_todo(id).await?;
    Ok(ApiResponse::ok(todo))
}

pub async fn create_todo(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateTodoRequest>,
) -> Result<ApiResponse<Todo>, AppError> {
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("Title is required".to_string()));
    }
    let now = utc_timestamp();
    let prompt = if req.prompt.trim().is_empty() {
        title.to_string()
    } else {
        req.prompt.trim().to_string()
    };
    let executor = req.executor.clone().unwrap_or_else(|| "claudecode".to_string());
    let id = state.db.create_todo(title, &prompt).await?;

    // Update executor if specified
    if let Some(ref exec) = req.executor {
        state.db.update_todo_executor(id, exec).await.ok();
    }

    for tag_id in &req.tag_ids {
        state.db.add_todo_tag(id, *tag_id).await;
    }

    Ok(ApiResponse::ok(Todo {
        id,
        title: title.to_string(),
        prompt,
        status: crate::models::TodoStatus::Pending,
        created_at: now.clone(),
        updated_at: now,
        tag_ids: req.tag_ids.clone(),
        executor: Some(executor),
        scheduler_enabled: false,
        scheduler_config: None,
        scheduler_next_run_at: None,
        task_id: None,
        workspace: None,
    }))
}

pub async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTodoRequest>,
) -> Result<ApiResponse<Todo>, AppError> {
    // 获取当前值用于填充
    let current = state.require_todo(id).await?;

    let title = req.title.unwrap_or(current.title);
    let prompt = req.prompt.unwrap_or(current.prompt);
    let status = req.status.unwrap_or(current.status);
    let executor = req.executor.or(current.executor);
    let workspace = req.workspace.or(current.workspace);

    let scheduler_config = req.scheduler_config
        .as_ref()
        .filter(|s| !s.is_empty())
        .cloned();

    // Validate cron expression if scheduler config is provided
    if let Some(ref config) = scheduler_config {
        validate_cron_expression(config)?;
    }
    state
        .db
        .update_todo_full(
            id,
            &title,
            &prompt,
            status,
            executor.as_deref(),
            req.scheduler_enabled,
            scheduler_config.as_deref(),
            workspace.as_deref(),
        )
        .await
        .map_err(AppError::from)?;

    let todo = state.require_todo(id).await?;
    Ok(ApiResponse::ok(todo))
}

pub async fn update_todo_tags(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTagsRequest>,
) -> Result<ApiResponse<()>, AppError> {
    state.db.set_todo_tags(id, &req.tag_ids).await;
    Ok(ApiResponse::ok(()))
}

pub async fn delete_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    let _ = state.db.delete_todo(id).await;
    Ok(ApiResponse::ok(()))
}

pub async fn force_update_todo_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTodoRequest>,
) -> Result<ApiResponse<Todo>, AppError> {
    if let Some(status) = req.status {
        state.db.force_update_todo_status(id, status).await;
    }
    let todo = state.require_todo(id).await?;
    Ok(ApiResponse::ok(todo))
}
