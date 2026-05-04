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

    cfg.port = req.port;
    cfg.host = req.host;
    cfg.db_path = req.db_path;
    cfg.log_level = req.log_level;
    cfg.executors = req.executors;

    cfg.normalize_paths();

    if let Err(e) = cfg.save() {
        return Err(AppError::Internal(format!("Failed to save config: {}", e)));
    }

    Ok(ApiResponse::ok(cfg.clone()))
}
