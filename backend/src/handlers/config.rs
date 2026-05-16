use std::path::PathBuf;

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
        // 路径穿越防护：验证数据库路径位于安全目录 ~/.ntd/ 内
        let normalized = Config::normalize_single_path(&db_path);
        let canonicalized = std::fs::canonicalize(&PathBuf::from(&normalized))
            .map_err(|_| AppError::BadRequest("Invalid database path: cannot resolve".to_string()))?;
        let safe_dir = dirs::home_dir()
            .ok_or_else(|| AppError::Internal("Cannot determine home directory".to_string()))?
            .join(".ntd");
        let safe_dir_canonical = std::fs::canonicalize(&safe_dir).unwrap_or(safe_dir);
        if !canonicalized.starts_with(&safe_dir_canonical) {
            return Err(AppError::BadRequest(
                "Database path must be within ~/.ntd/ directory".to_string(),
            ));
        }
        cfg.db_path = db_path;
    }
    if let Some(log_level) = req.log_level {
        cfg.log_level = log_level;
    }
    if let Some(slash_command_rules) = req.slash_command_rules {
        cfg.slash_command_rules = slash_command_rules;
    }
    if let Some(default_response_todo_id) = req.default_response_todo_id {
        cfg.default_response_todo_id = Some(default_response_todo_id);
    }
    if let Some(history_message_max_age_secs) = req.history_message_max_age_secs {
        cfg.history_message_max_age_secs = history_message_max_age_secs;
    }
    if let Some(max_concurrent_todos) = req.max_concurrent_todos {
        if max_concurrent_todos == 0 {
            return Err(AppError::BadRequest("max_concurrent_todos must be at least 1".to_string()));
        }
        cfg.max_concurrent_todos = max_concurrent_todos;
    }
    if let Some(execution_timeout_secs) = req.execution_timeout_secs {
        if execution_timeout_secs < 60 {
            return Err(AppError::BadRequest("execution_timeout_secs must be at least 60".to_string()));
        }
        cfg.execution_timeout_secs = execution_timeout_secs;
    }

    cfg.normalize_paths();

    let cfg_clone = cfg.clone();
    tokio::task::spawn_blocking(move || cfg_clone.save())
        .await
        .map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
        .map_err(|e| AppError::Internal(format!("Failed to save config: {}", e)))?;

    Ok(ApiResponse::ok(cfg.clone()))
}
