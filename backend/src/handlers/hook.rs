use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::handlers::{ApiJson, AppError, AppState};
use crate::hooks::db::HookDb;
use crate::hooks::models::*;

/// List all hooks
pub async fn list_hooks(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let hooks = HookDb::get_hooks(&state.db.conn).await?;
    let responses: Vec<HookResponse> = hooks.into_iter().map(|h| h.into()).collect();
    Ok(crate::models::ApiResponse::ok(responses))
}

/// Get a single hook by ID
pub async fn get_hook(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let hook = HookDb::get_hook_by_id(&state.db.conn, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(crate::models::ApiResponse::ok(HookResponse::from(hook)))
}

/// Create a new hook
pub async fn create_hook(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateHookRequest>,
) -> Result<impl IntoResponse, AppError> {
    let hook = HookDb::create_hook(&state.db.conn, req).await?;
    Ok((StatusCode::CREATED, crate::models::ApiResponse::ok(HookResponse::from(hook))))
}

/// Update a hook
pub async fn update_hook(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateHookRequest>,
) -> Result<impl IntoResponse, AppError> {
    let hook = HookDb::update_hook(&state.db.conn, id, req).await?;
    Ok(crate::models::ApiResponse::ok(HookResponse::from(hook)))
}

/// Delete a hook
pub async fn delete_hook(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    HookDb::delete_hook(&state.db.conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Test a hook: validate the action points at an existing todo and return a summary.
/// Does not start a real execution — use the dry-run summary to confirm wiring.
pub async fn test_hook(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let hook = HookDb::get_hook_by_id(&state.db.conn, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let result = match state.db.get_todo(hook.action.target_todo_id).await {
        Ok(Some(target)) => {
            let rendered = hook
                .action
                .prompt_template
                .clone()
                .unwrap_or_else(|| target.prompt.clone());
            let summary = serde_json::json!({
                "valid": true,
                "target_todo_id": target.id,
                "target_todo_title": target.title,
                "rendered_prompt_preview": rendered.chars().take(200).collect::<String>(),
                "trigger": hook.trigger.as_str(),
                "message": format!(
                    "Hook would trigger todo #{} ('{}') with rendered prompt ({} chars).",
                    target.id, target.title, rendered.len()
                ),
            });
            crate::hooks::models::HookResult::success(
                0,
                summary.to_string(),
                String::new(),
                0,
            )
        }
        Ok(None) => crate::hooks::models::HookResult::error(
            format!(
                "Target todo #{} not found. Edit the hook to point at a real todo.",
                hook.action.target_todo_id
            ),
            0,
        ),
        Err(e) => crate::hooks::models::HookResult::error(
            format!("Failed to look up target todo: {}", e),
            0,
        ),
    };

    Ok(crate::models::ApiResponse::ok(result))
}

/// Get global hook config
pub async fn get_global_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let config = HookDb::get_global_config(&state.db.conn).await?;
    Ok(crate::models::ApiResponse::ok(GlobalHookConfigResponse {
        enabled: config.enabled,
        default_timeout_secs: config.default_timeout_secs,
        max_concurrency: config.max_concurrency,
    }))
}

/// Update global hook config
pub async fn update_global_config(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UpdateGlobalHookConfigRequest>,
) -> Result<impl IntoResponse, AppError> {
    let config = HookDb::update_global_config(&state.db.conn, req).await?;

    // Update the hook service concurrency if needed
    // Note: This would require exposing a method to update the semaphore size

    Ok(crate::models::ApiResponse::ok(GlobalHookConfigResponse {
        enabled: config.enabled,
        default_timeout_secs: config.default_timeout_secs,
        max_concurrency: config.max_concurrency,
    }))
}

/// Set global default hooks
pub async fn set_global_default_hooks(
    State(state): State<AppState>,
    ApiJson(hook_ids): ApiJson<Vec<i64>>,
) -> Result<impl IntoResponse, AppError> {
    HookDb::set_global_default_hooks(&state.db.conn, hook_ids).await?;
    Ok(crate::models::ApiResponse::ok(()))
}

/// Get per-todo hook config
pub async fn get_todo_hooks(
    State(state): State<AppState>,
    Path(todo_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let config = HookDb::get_todo_hook_config(&state.db.conn, todo_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let rule_ids = HookDb::get_todo_hook_rule_ids(&state.db.conn, todo_id).await?;

    Ok(crate::models::ApiResponse::ok(TodoHookConfigResponse {
        todo_id,
        hook_mode: config.hook_mode.as_str().to_string(),
        override_enabled: config.override_enabled,
        rule_ids,
    }))
}

/// Update per-todo hook config
pub async fn update_todo_hooks(
    State(state): State<AppState>,
    Path(todo_id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTodoHookRequest>,
) -> Result<impl IntoResponse, AppError> {
    let config = HookDb::update_todo_hook_config(&state.db.conn, todo_id, req).await?;
    let rule_ids = HookDb::get_todo_hook_rule_ids(&state.db.conn, todo_id).await?;

    Ok(crate::models::ApiResponse::ok(TodoHookConfigResponse {
        todo_id,
        hook_mode: config.hook_mode.as_str().to_string(),
        override_enabled: config.override_enabled,
        rule_ids,
    }))
}

/// Get hook logs
pub async fn get_hook_logs(
    State(state): State<AppState>,
    Query(query): Query<HookLogQuery>,
) -> Result<impl IntoResponse, AppError> {
    let (logs, total) = HookDb::get_hook_logs(&state.db.conn, query.clone()).await?;

    Ok(crate::models::ApiResponse::ok(HookLogPage {
        logs,
        total,
        page: query.page,
        limit: query.limit,
    }))
}

/// Clear hook logs
pub async fn clear_hook_logs(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let deleted = HookDb::delete_hook_logs(&state.db.conn).await?;
    Ok(crate::models::ApiResponse::ok(deleted))
}
