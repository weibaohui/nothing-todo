use axum::extract::State;
use axum::routing::{delete, get, post, put};
use axum::Router;
use serde::{Deserialize, Serialize};

use super::{ApiJson, AppState};
use crate::models::ApiResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectDirectoryRequest {
    pub path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectDirectoryRequest {
    pub name: Option<String>,
}

pub async fn list_project_directories(
    State(state): State<AppState>,
) -> ApiResponse<Vec<crate::db::project_directory::ProjectDirectory>> {
    let directories = state.db.get_project_directories().await.unwrap_or_default();
    ApiResponse::ok(directories)
}

pub async fn create_project_directory(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateProjectDirectoryRequest>,
) -> ApiResponse<crate::db::project_directory::ProjectDirectory> {
    if req.path.trim().is_empty() {
        return ApiResponse::err(crate::models::codes::BAD_REQUEST, "Path is required");
    }
    // 检查是否已存在
    if let Some(existing) = state.db.get_project_directory_by_path(&req.path).await.unwrap_or(None) {
        return ApiResponse::ok(existing);
    }
    let id = state
        .db
        .create_project_directory(&req.path, req.name.as_deref())
        .await
        .unwrap_or(0);
    let directory = crate::db::project_directory::ProjectDirectory {
        id,
        path: req.path,
        name: req.name,
        created_at: crate::models::utc_timestamp(),
        updated_at: crate::models::utc_timestamp(),
    };
    ApiResponse::ok(directory)
}

pub async fn update_project_directory(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    ApiJson(req): ApiJson<UpdateProjectDirectoryRequest>,
) -> ApiResponse<()> {
    state
        .db
        .update_project_directory(id, req.name.as_deref())
        .await
        .ok();
    ApiResponse::ok(())
}

pub async fn delete_project_directory(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> ApiResponse<()> {
    state.db.delete_project_directory(id).await.ok();
    ApiResponse::ok(())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/xyz/project-directories", get(list_project_directories))
        .route("/xyz/project-directories", post(create_project_directory))
        .route("/xyz/project-directories/{id}", put(update_project_directory))
        .route("/xyz/project-directories/{id}", delete(delete_project_directory))
}