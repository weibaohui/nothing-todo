use axum::extract::State;

use crate::config::Config;
use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, UpdateConfigRequest};

pub async fn get_config(State(state): State<AppState>) -> Result<ApiResponse<Config>, AppError> {
    let cfg = state.config.read().await.clone();
    Ok(ApiResponse::ok(cfg))
}

pub async fn update_config(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UpdateConfigRequest>,
) -> Result<ApiResponse<Config>, AppError> {
    let mut cfg = state.config.write().await;

    if let Some(port) = req.port {
        cfg.port = port;
    }
    if let Some(host) = req.host {
        cfg.host = host;
    }
    if let Some(db_path) = req.db_path {
        cfg.db_path = db_path;
    }
    if let Some(log_level) = req.log_level {
        cfg.log_level = log_level;
    }
    if let Some(executors) = req.executors {
        cfg.executors = executors;
    }
    if let Some(slash_command_rules) = req.slash_command_rules {
        cfg.slash_command_rules = slash_command_rules;
    }
    if let Some(default_response_todo_id) = req.default_response_todo_id {
        cfg.default_response_todo_id = Some(default_response_todo_id);
    }

    cfg.normalize_paths();

    if let Err(e) = cfg.save() {
        return Err(AppError::Internal(format!("Failed to save config: {}", e)));
    }

    Ok(ApiResponse::ok(cfg.clone()))
}
