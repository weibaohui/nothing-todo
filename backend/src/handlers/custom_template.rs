use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

use crate::db::Database;
use crate::handlers::{ApiJson, AppError, AppState};
use crate::models::{ApiResponse, TodoTemplate};

/// Remote template YAML format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTemplate {
    pub title: String,
    pub prompt: Option<String>,
    pub category: Option<String>,
}

impl RemoteTemplate {
    pub fn category_or_default(&self) -> String {
        self.category.clone().unwrap_or_else(|| "自定义".to_string())
    }
}

/// Custom template subscription status
#[derive(Serialize)]
pub struct CustomTemplateStatus {
    pub subscribed: bool,
    pub source_url: Option<String>,
    pub last_sync_at: Option<String>,
    pub auto_sync_enabled: bool,
    pub auto_sync_cron: String,
    pub templates: Vec<TodoTemplate>,
}

/// Subscribe to a remote template URL
#[derive(Deserialize)]
pub struct SubscribeRequest {
    pub url: String,
}

/// Update auto sync config
#[derive(Deserialize)]
pub struct UpdateAutoSyncRequest {
    pub enabled: bool,
    pub cron: String,
}

/// Fetch YAML from URL and parse it
pub async fn fetch_remote_templates(url: &str) -> Result<Vec<RemoteTemplate>, String> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Try parsing as YAML array first, then as single object
    let templates: Vec<RemoteTemplate> = if body.trim().starts_with('-') {
        serde_yaml::from_str(&body)
            .map_err(|e| format!("Invalid YAML array format: {}", e))?
    } else {
        let single: RemoteTemplate = serde_yaml::from_str(&body)
            .map_err(|e| format!("Invalid YAML format: {}", e))?;
        vec![single]
    };

    Ok(templates)
}

/// Get custom template subscription status
pub async fn get_custom_template_status(
    State(state): State<AppState>,
) -> Result<ApiResponse<CustomTemplateStatus>, AppError> {
    let cfg = crate::config::Config::load();

    let subscription = state.db.get_custom_template_subscription().await?;
    let (subscribed, source_url, last_sync_at) = match subscription {
        Some((url, sync_at)) => (true, Some(url), sync_at),
        None => (false, None, None),
    };

    // Get all templates with source_url set (custom templates)
    let all_templates = state.db.get_templates().await?;
    let custom_templates: Vec<TodoTemplate> = all_templates
        .into_iter()
        .filter(|t| t.source_url.is_some())
        .collect();

    Ok(ApiResponse::ok(CustomTemplateStatus {
        subscribed,
        source_url,
        last_sync_at,
        auto_sync_enabled: cfg.auto_sync_custom_templates_enabled,
        auto_sync_cron: cfg.auto_sync_custom_templates_cron.clone(),
        templates: custom_templates,
    }))
}

/// Subscribe to a remote template URL and sync immediately
pub async fn subscribe_custom_template(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<SubscribeRequest>,
) -> Result<ApiResponse<CustomTemplateStatus>, AppError> {
    let url = req.url.trim();
    if url.is_empty() {
        return Err(AppError::BadRequest("URL is required".to_string()));
    }

    // Validate URL format
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::BadRequest("URL must start with http:// or https://".to_string()));
    }

    // Delete existing custom templates (from any previous subscription)
    state.db.delete_all_custom_templates().await?;

    // Fetch templates from URL
    let remote_templates = fetch_remote_templates(url)
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to fetch templates: {}", e)))?;

    if remote_templates.is_empty() {
        return Err(AppError::BadRequest("No templates found in remote file".to_string()));
    }

    // Create templates in database
    for (idx, remote) in remote_templates.iter().enumerate() {
        state.db.create_template_from_remote(
            &remote.title,
            remote.prompt.as_deref(),
            &remote.category_or_default(),
            Some(idx as i32),
            url,
        ).await?;
    }

    // Return updated status
    get_custom_template_status(State(state)).await
}

/// Unsubscribe from remote template
pub async fn unsubscribe_custom_template(
    State(state): State<AppState>,
) -> Result<ApiResponse<()>, AppError> {
    state.db.delete_all_custom_templates().await?;
    Ok(ApiResponse::ok(()))
}

/// Sync templates from the subscribed URL
pub async fn sync_custom_template(
    State(state): State<AppState>,
) -> Result<ApiResponse<CustomTemplateStatus>, AppError> {
    let subscription = state.db.get_custom_template_subscription().await?
        .ok_or_else(|| AppError::BadRequest("Not subscribed to any remote template".to_string()))?;

    let (url, _) = subscription;

    // Delete existing custom templates
    state.db.delete_templates_by_source_url(&url).await?;

    // Fetch templates from URL
    let remote_templates = fetch_remote_templates(&url)
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to fetch templates: {}", e)))?;

    if remote_templates.is_empty() {
        return Err(AppError::BadRequest("No templates found in remote file".to_string()));
    }

    // Create templates in database
    for (idx, remote) in remote_templates.iter().enumerate() {
        state.db.create_template_from_remote(
            &remote.title,
            remote.prompt.as_deref(),
            &remote.category_or_default(),
            Some(idx as i32),
            &url,
        ).await?;
    }

    // Return updated status
    get_custom_template_status(State(state)).await
}

/// Update auto sync configuration
pub async fn update_auto_sync_config(
    ApiJson(req): ApiJson<UpdateAutoSyncRequest>,
) -> Result<ApiResponse<String>, AppError> {
    // Validate cron expression
    if req.enabled {
        let schedule = cron::Schedule::from_str(&req.cron)
            .map_err(|e| AppError::BadRequest(format!("Invalid cron expression: {}", e)))?;
        schedule.upcoming(chrono::Utc).next()
            .ok_or_else(|| AppError::BadRequest("Cron expression has no future executions".to_string()))?;
    }

    let mut cfg = crate::config::Config::load();
    cfg.auto_sync_custom_templates_enabled = req.enabled;
    cfg.auto_sync_custom_templates_cron = req.cron;
    cfg.save().map_err(AppError::Internal)?;

    Ok(ApiResponse::ok("自动同步配置已更新".to_string()))
}

/// Perform custom template sync (called by scheduler)
pub async fn perform_custom_template_sync(state: AppState) -> Result<(), String> {
    let subscription = state.db.get_custom_template_subscription().await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Not subscribed".to_string())?;

    let (url, _) = subscription;

    // Delete existing custom templates
    state.db.delete_templates_by_source_url(&url).await
        .map_err(|e| format!("Failed to delete old templates: {}", e))?;

    // Fetch templates from URL
    let remote_templates = fetch_remote_templates(&url).await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if remote_templates.is_empty() {
        return Err("No templates found in remote file".to_string());
    }

    // Create templates in database
    for (idx, remote) in remote_templates.iter().enumerate() {
        state.db.create_template_from_remote(
            &remote.title,
            remote.prompt.as_deref(),
            &remote.category_or_default(),
            Some(idx as i32),
            &url,
        ).await
        .map_err(|e| format!("Failed to create template: {}", e))?;
    }

    tracing::info!("Custom template sync completed: {} templates imported", remote_templates.len());
    Ok(())
}

/// Start custom template auto sync scheduler
pub fn start_custom_template_auto_sync(
    cron_expr: &str,
    db: Arc<Database>,
) -> Result<(), String> {
    let schedule = cron::Schedule::from_str(cron_expr)
        .map_err(|e| format!("Invalid cron: {}", e))?;

    let db_clone = db.clone();
    tokio::spawn(async move {
        loop {
            let next = schedule.upcoming(chrono::Utc).next();
            let delay = match next {
                Some(dt) => {
                    let now = chrono::Utc::now();
                    (dt - now).to_std().unwrap_or(std::time::Duration::from_secs(60))
                }
                None => std::time::Duration::from_secs(3600),
            };
            tokio::time::sleep(delay).await;

            let db = db_clone.clone();
            match perform_custom_template_sync_internal(&db).await {
                Ok(msg) => tracing::info!("{}", msg),
                Err(e) => tracing::error!("Auto custom template sync failed: {}", e),
            }
        }
    });

    Ok(())
}

async fn perform_custom_template_sync_internal(db: &Arc<Database>) -> Result<String, String> {
    let subscription = db.get_custom_template_subscription().await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Not subscribed".to_string())?;

    let (url, _) = subscription;

    // Delete existing custom templates
    db.delete_templates_by_source_url(&url).await
        .map_err(|e| format!("Failed to delete old templates: {}", e))?;

    // Fetch templates from URL
    let remote_templates = fetch_remote_templates(&url).await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    if remote_templates.is_empty() {
        return Err("No templates found in remote file".to_string());
    }

    // Create templates in database
    for (idx, remote) in remote_templates.iter().enumerate() {
        db.create_template_from_remote(
            &remote.title,
            remote.prompt.as_deref(),
            &remote.category_or_default(),
            Some(idx as i32),
            &url,
        ).await
        .map_err(|e| format!("Failed to create template: {}", e))?;
    }

    Ok(format!("Auto custom template sync completed: {} templates imported", remote_templates.len()))
}