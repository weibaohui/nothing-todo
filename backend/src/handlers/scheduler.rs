use axum::{
    extract::{Path, State},
    response::Json,
};

use crate::handlers::{AppError, AppState};
use crate::models::{ApiResponse, Todo, UpdateSchedulerRequest};

pub async fn update_scheduler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSchedulerRequest>,
) -> Result<Json<ApiResponse<Todo>>, AppError> {
    if req.scheduler_enabled {
        if let Some(ref config) = req.scheduler_config {
            if let Err(e) = state
                .scheduler
                .upsert_task(
                    state.db.clone(),
                    state.executor_registry.clone(),
                    state.tx.clone(),
                    id,
                    config.clone(),
                    state.task_manager.clone(),
                )
                .await
            {
                tracing::error!("Failed to upsert scheduled task for todo {}: {}", id, e);
            }
        } else {
            state.scheduler.remove_task_for_todo(id).await;
        }
    } else {
        state.scheduler.remove_task_for_todo(id).await;
    }

    state
        .db
        .update_todo_scheduler(id, req.scheduler_enabled, req.scheduler_config.as_deref())
        .await;

    match state.db.get_todo(id).await {
        Some(todo) => Ok(Json(ApiResponse::ok(todo))),
        None => Err(AppError::NotFound),
    }
}

pub async fn get_scheduler_todos(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Todo>>>, AppError> {
    Ok(Json(ApiResponse::ok(state.db.get_scheduler_todos().await)))
}
