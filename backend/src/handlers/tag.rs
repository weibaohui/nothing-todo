use axum::{
    extract::{Path, State},
};

use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, CreateTagRequest, Tag, utc_timestamp};

pub async fn get_tags(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<Tag>>, AppError> {
    Ok(ApiResponse::ok(state.db.get_tags().await))
}

pub async fn create_tag(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateTagRequest>,
) -> Result<ApiResponse<Tag>, AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Tag name is required".to_string()));
    }
    let now = utc_timestamp();
    let id = state.db.create_tag(name, &req.color).await.map_err(AppError::from)?;
    Ok(ApiResponse::ok(Tag {
        id,
        name: name.to_string(),
        color: req.color,
        created_at: now,
    }))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse<()>, AppError> {
    state.db.delete_tag(id).await;
    Ok(ApiResponse::ok(()))
}
