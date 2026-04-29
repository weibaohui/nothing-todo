use axum::{
    extract::State,
    body::Bytes,
    response::IntoResponse,
    http::header,
};

use crate::handlers::{AppError, AppState};
use crate::models::{ApiResponse, BackupData, utc_timestamp};

/// 导出备份（返回 YAML 格式字符串）
pub async fn export_backup(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let tags = state.db.get_tag_backups().await;
    let todos = state.db.get_todo_backups().await;
    let data = BackupData {
        version: "1.0".to_string(),
        created_at: utc_timestamp(),
        tags,
        todos,
    };
    let yaml = serde_yaml::to_string(&data).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((
        [(header::CONTENT_TYPE, "application/x-yaml; charset=utf-8")],
        yaml,
    ))
}

/// 导入备份（接收 YAML 格式字符串，清空现有数据后导入）
pub async fn import_backup(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<ApiResponse<String>, AppError> {
    let yaml_str = String::from_utf8(body.to_vec())
        .map_err(|_| AppError::BadRequest("Invalid UTF-8 in request body".to_string()))?;
    let data: BackupData = serde_yaml::from_str(&yaml_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid YAML: {}", e)))?;

    if data.todos.is_empty() {
        return Err(AppError::BadRequest("Backup contains no todos".to_string()));
    }

    state.db.import_backup(&data.tags, &data.todos).await
        .map_err(|e| AppError::Internal(format!("Import failed, data unchanged: {}", e)))?;

    Ok(ApiResponse::ok(format!("Imported {} todos and {} tags", data.todos.len(), data.tags.len())))
}