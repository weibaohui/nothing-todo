use axum::extract::{Path, State};
use axum::Json;

use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, CreateTemplateRequest, TodoTemplate, UpdateTemplateRequest};

pub async fn get_templates(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<TodoTemplate>>>, AppError> {
    let templates = state.db.get_templates().await?;
    Ok(Json(ApiResponse::ok(templates)))
}

pub async fn get_templates_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<ApiResponse<Vec<TodoTemplate>>>, AppError> {
    let templates = state.db.get_templates_by_category(&category).await?;
    Ok(Json(ApiResponse::ok(templates)))
}

pub async fn create_template(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateTemplateRequest>,
) -> Result<Json<ApiResponse<TodoTemplate>>, AppError> {
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("Title is required".to_string()));
    }

    let category = req.category.trim();
    if category.is_empty() {
        return Err(AppError::BadRequest("Category is required".to_string()));
    }

    let id = state.db
        .create_template(title, req.prompt.as_deref(), category, req.sort_order)
        .await?;

    let templates = state.db.get_templates().await?;
    let template = templates
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::Internal("Failed to get created template".to_string()))?;

    Ok(Json(ApiResponse::ok(template)))
}

pub async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateTemplateRequest>,
) -> Result<Json<ApiResponse<TodoTemplate>>, AppError> {
    let templates = state.db.get_templates().await?;
    let existing = templates
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::NotFound)?;

    let title = req.title.unwrap_or_else(|| existing.title.clone());
    let prompt = req.prompt.or(existing.prompt.clone());
    let category = req.category.unwrap_or_else(|| existing.category.clone());
    let sort_order = req.sort_order.or(Some(existing.sort_order));

    state.db
        .update_template(id, &title, prompt.as_deref(), &category, sort_order)
        .await?;

    let templates = state.db.get_templates().await?;
    let template = templates
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::Internal("Failed to get updated template".to_string()))?;

    Ok(Json(ApiResponse::ok(template)))
}

pub async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.db.delete_template(id).await?;
    Ok(Json(ApiResponse::ok(())))
}
