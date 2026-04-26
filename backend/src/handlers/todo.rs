use axum::{
    extract::{Path, State},
};

use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, CreateTodoRequest, Todo, UpdateTagsRequest, UpdateTodoRequest, utc_timestamp};

pub async fn get_todos(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<Todo>>, AppError> {
    Ok(ApiResponse::ok(state.db.get_todos().await))
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
    let id = state.db.create_todo(title, &prompt).await;

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
        executor: Some("claudecode".to_string()),
        scheduler_enabled: false,
        scheduler_config: None,
        scheduler_next_run_at: None,
        task_id: None,
    }))
}

pub async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTodoRequest>,
) -> Result<ApiResponse<Todo>, AppError> {
    let prompt = if req.prompt.trim().is_empty() {
        req.title.clone()
    } else {
        req.prompt.clone()
    };
    state
        .db
        .update_todo_full(
            id,
            &req.title,
            &prompt,
            req.status,
            req.executor.as_deref(),
            req.scheduler_enabled,
            req.scheduler_config.as_deref(),
        )
        .await;

    match state.db.get_todo(id).await {
        Some(todo) => Ok(ApiResponse::ok(todo)),
        None => Err(AppError::NotFound),
    }
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
    state.db.delete_todo(id).await;
    Ok(ApiResponse::ok(()))
}

pub async fn force_update_todo_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTodoRequest>,
) -> Result<ApiResponse<Todo>, AppError> {
    state.db.force_update_todo_status(id, req.status).await;
    match state.db.get_todo(id).await {
        Some(todo) => Ok(ApiResponse::ok(todo)),
        None => Err(AppError::NotFound),
    }
}
