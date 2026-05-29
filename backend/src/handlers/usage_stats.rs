use axum::extract::{Query, State};
use serde::Deserialize;
use std::str::FromStr;

use super::{AppError, AppState};
use crate::models::ApiResponse;
use crate::services::usage_stats::{UsageReport, UsageStat, UsageStatsService};

#[derive(Debug, Deserialize)]
pub struct UsageStatsQuery {
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct UsageStatsResponse {
    pub daily: Vec<UsageStat>,
    pub weekly: Vec<UsageStat>,
    pub monthly: Vec<UsageStat>,
}

impl From<UsageReport> for UsageStatsResponse {
    fn from(report: UsageReport) -> Self {
        Self {
            daily: report.daily,
            weekly: report.weekly,
            monthly: report.monthly,
        }
    }
}

pub async fn get_usage_stats(
    State(state): State<AppState>,
    Query(query): Query<UsageStatsQuery>,
) -> Result<ApiResponse<UsageStatsResponse>, AppError> {
    let service = UsageStatsService::new(state.db.clone());

    let daily = service
        .get_stats("daily", query.since.as_deref(), query.until.as_deref())
        .await
        .map_err(|e| AppError::Internal(e))?;

    let weekly = service
        .get_stats("weekly", query.since.as_deref(), query.until.as_deref())
        .await
        .map_err(|e| AppError::Internal(e))?;

    let monthly = service
        .get_stats("monthly", query.since.as_deref(), query.until.as_deref())
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok(ApiResponse::ok(UsageStatsResponse {
        daily,
        weekly,
        monthly,
    }))
}

pub async fn refresh_usage_stats(
    State(state): State<AppState>,
) -> Result<ApiResponse<UsageStatsResponse>, AppError> {
    let service = UsageStatsService::new(state.db.clone());

    let report = service
        .refresh_all_stats()
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok(ApiResponse::ok(report.into()))
}

#[derive(Debug, serde::Serialize)]
pub struct UsageStatsSettings {
    pub auto_usage_stats_enabled: bool,
    pub auto_usage_stats_cron: String,
}

pub async fn get_usage_stats_settings(
    State(state): State<AppState>,
) -> Result<ApiResponse<UsageStatsSettings>, AppError> {
    let cfg = state.config.read().await;
    Ok(ApiResponse::ok(UsageStatsSettings {
        auto_usage_stats_enabled: cfg.auto_usage_stats_enabled,
        auto_usage_stats_cron: cfg.auto_usage_stats_cron.clone(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsageStatsSettingsRequest {
    pub enabled: bool,
    pub cron: String,
}

pub async fn update_usage_stats_settings(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<UpdateUsageStatsSettingsRequest>,
) -> Result<ApiResponse<String>, AppError> {
    // Validate cron expression
    if req.enabled {
        let schedule = cron::Schedule::from_str(&req.cron)
            .map_err(|e| AppError::BadRequest(format!("Invalid cron expression: {}", e)))?;
        schedule.upcoming(chrono::Utc).next()
            .ok_or_else(|| AppError::BadRequest("Cron expression has no future executions".to_string()))?;
    }

    let mut cfg = state.config.write().await;
    cfg.auto_usage_stats_enabled = req.enabled;
    cfg.auto_usage_stats_cron = req.cron;
    cfg.normalize_paths();

    let cfg_clone = cfg.clone();
    tokio::task::spawn_blocking(move || cfg_clone.save())
        .await
        .map_err(|e| AppError::Internal(format!("Join error: {}", e)))?
        .map_err(|e| AppError::Internal(format!("Failed to save config: {}", e)))?;

    Ok(ApiResponse::ok("AI 使用统计配置已更新".to_string()))
}
