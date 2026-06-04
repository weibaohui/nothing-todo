//! 云端同步 handlers
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::handlers::{ApiResponse, AppError, AppState};

// ============ Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub server_url: String,
    /// 同步 Token (ntd_xxx 格式)
    pub sync_token: Option<String>,
    /// 最后同步时间
    pub last_sync_at: Option<String>,
    /// 默认冲突解决模式
    pub default_conflict_mode: String,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            sync_token: None,
            last_sync_at: None,
            default_conflict_mode: "overwrite".to_string(),
        }
    }
}

// ============ Sync Status Handlers ============

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub connected: bool,
    pub authenticated: bool,
    pub last_sync_at: Option<String>,
    pub server_url: String,
}

#[derive(Deserialize)]
struct CloudSyncStatusResponse {
    last_sync_at: Option<String>,
}

/// GET /api/cloud/sync/status - 获取同步状态
pub async fn cloud_sync_status(
    State(state): State<AppState>,
) -> Result<ApiResponse<SyncStatusResponse>, AppError> {
    let cfg = state.config.read().await;

    let connected = !cfg.cloud_sync.server_url.is_empty();
    let authenticated = cfg.cloud_sync.sync_token.is_some();
    let server_url = cfg.cloud_sync.server_url.clone();

    // 如果已配置 token，尝试从云端获取真实同步状态
    let last_sync_at = if let Some(token) = &cfg.cloud_sync.sync_token {
        if !cfg.cloud_sync.server_url.is_empty() {
            match reqwest::Client::new()
                .get(format!("{}/api/v1/sync/status?data_type=todos", cfg.cloud_sync.server_url))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<CloudSyncStatusResponse>().await {
                        Ok(data) => data.last_sync_at,
                        Err(_) => cfg.cloud_sync.last_sync_at.clone(),
                    }
                }
                Ok(_) => cfg.cloud_sync.last_sync_at.clone(),
                Err(_) => cfg.cloud_sync.last_sync_at.clone(),
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(ApiResponse::ok(SyncStatusResponse {
        connected,
        authenticated,
        last_sync_at,
        server_url,
    }))
}

// ============ Config Handlers ============

#[derive(Deserialize)]
pub struct CloudConfigRequest {
    pub server_url: Option<String>,
    /// 同步 Token (ntd_xxx 格式)
    pub sync_token: Option<String>,
    pub default_conflict_mode: Option<String>,
}

#[derive(Serialize)]
pub struct CloudConfigResponse {
    pub server_url: String,
    /// 是否已配置 Token (不返回实际 token)
    pub has_token: bool,
    pub last_sync_at: Option<String>,
    pub default_conflict_mode: String,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub saved: bool,
}

/// GET /api/cloud/config - 获取云端配置
pub async fn cloud_get_config(
    State(state): State<AppState>,
) -> Result<ApiResponse<CloudConfigResponse>, AppError> {
    let cfg = state.config.read().await;

    Ok(ApiResponse::ok(CloudConfigResponse {
        server_url: cfg.cloud_sync.server_url.clone(),
        has_token: cfg.cloud_sync.sync_token.is_some(),
        last_sync_at: cfg.cloud_sync.last_sync_at.clone(),
        default_conflict_mode: cfg.cloud_sync.default_conflict_mode.clone(),
    }))
}

/// POST /api/cloud/config - 保存云端配置
pub async fn cloud_save_config(
    State(state): State<AppState>,
    Json(req): Json<CloudConfigRequest>,
) -> Result<ApiResponse<SaveResponse>, AppError> {
    let mut cfg = state.config.write().await;
    if let Some(url) = req.server_url {
        cfg.cloud_sync.server_url = url.trim_end_matches('/').to_string();
    }
    if let Some(token) = req.sync_token {
        cfg.cloud_sync.sync_token = Some(token);
    }
    if let Some(mode) = req.default_conflict_mode {
        cfg.cloud_sync.default_conflict_mode = mode;
    }
    cfg.save().map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(ApiResponse::ok(SaveResponse { saved: true }))
}

// ============ Sync Records Handlers ============

#[derive(Serialize)]
pub struct SyncRecord {
    pub id: i64,
    pub direction: String,
    pub conflict_mode: String,
    pub status: String,
    pub data_type: String,
    pub details: Option<String>,
    pub error_message: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Deserialize)]
pub struct SyncRecordsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/cloud/sync/records - 获取同步历史记录
pub async fn cloud_sync_records(
    State(state): State<AppState>,
    Query(query): Query<SyncRecordsQuery>,
) -> Result<ApiResponse<Vec<SyncRecord>>, AppError> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let records = state.db.get_sync_records(limit, offset).await
        .map_err(|e| AppError::Internal(format!("获取同步记录失败: {}", e)))?;

    let response: Vec<SyncRecord> = records.into_iter().map(|r| SyncRecord {
        id: r.id,
        direction: r.direction,
        conflict_mode: r.conflict_mode,
        status: r.status,
        data_type: r.data_type,
        details: r.details,
        error_message: r.error_message,
        created_at: r.created_at,
    }).collect();

    Ok(ApiResponse::ok(response))
}
