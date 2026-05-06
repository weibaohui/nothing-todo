use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

use crate::db::Database;
use crate::handlers::{AppError, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/xyz/agent-bots", get(list_agent_bots))
        .route("/xyz/agent-bots/:id", delete(delete_agent_bot))
        .route("/xyz/agent-bots/feishu/init", post(feishu_init))
        .route("/xyz/agent-bots/feishu/begin", post(feishu_begin))
        .route("/xyz/agent-bots/feishu/poll", post(feishu_poll))
        .with_state(state)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuInitResponse {
    pub supported: bool,
    pub auth_methods: Vec<String>,
}

pub async fn feishu_init() -> Result<impl IntoResponse, AppError> {
    let client = Client::new();
    let res = client
        .post("https://accounts.feishu.cn/oauth/v1/app/registration")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[("action", "init")])
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let body: serde_json::Value = res.json().await.map_err(|e| AppError::Internal(e.to_string()))?;

    let supported_auth_methods: Vec<String> = body
        .get("supported_auth_methods")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let supported = supported_auth_methods.contains(&"client_secret".to_string());

    Ok(Json(serde_json::json!({
        "supported": supported,
        "auth_methods": supported_auth_methods
    })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuBeginResponse {
    pub device_code: String,
    pub qr_url: String,
    pub user_code: String,
    pub interval: u64,
    pub expire_in: u64,
}

pub async fn feishu_begin() -> Result<impl IntoResponse, AppError> {
    let client = Client::new();
    let res = client
        .post("https://accounts.feishu.cn/oauth/v1/app/registration")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("action", "begin"),
            ("archetype", "PersonalAgent"),
            ("auth_method", "client_secret"),
            ("request_user_info", "open_id"),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let body: serde_json::Value = res.json().await.map_err(|e| AppError::Internal(e.to_string()))?;

    let device_code = body
        .get("device_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("Missing device_code".to_string()))?
        .to_string();

    let mut qr_url = body
        .get("verification_uri_complete")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Internal("Missing verification_uri_complete".to_string()))?
        .to_string();

    // 添加追踪参数
    if qr_url.contains('?') {
        qr_url += "&from=ntd&tp=ntd";
    } else {
        qr_url += "?from=ntd&tp=ntd";
    }

    let user_code = body
        .get("user_code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let interval = body
        .get("interval")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);

    let expire_in = body
        .get("expire_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(600);

    Ok(Json(serde_json::json!({
        "device_code": device_code,
        "qr_url": qr_url,
        "user_code": user_code,
        "interval": interval,
        "expire_in": expire_in
    })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuPollResponse {
    pub success: bool,
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub domain: Option<String>,
    pub open_id: Option<String>,
    pub bot_name: Option<String>,
    pub bot_id: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeishuPollRequest {
    pub device_code: String,
    pub interval: Option<u64>,
    pub expire_in: Option<u64>,
}

pub async fn feishu_poll(
    State(state): State<AppState>,
    Json(req): Json<FeishuPollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let client = Client::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(req.expire_in.unwrap_or(600));
    let interval = req.interval.unwrap_or(5);

    loop {
        if std::time::Instant::now() > deadline {
            return Ok(Json(serde_json::json!({
                "success": false,
                "error": "timeout"
            })));
        }

        let res = client
            .post("https://accounts.feishu.cn/oauth/v1/app/registration")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("action", "poll"),
                ("device_code", &req.device_code),
                ("tp", "ob_app"),
            ])
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let body: serde_json::Value = res.json().await.map_err(|e| AppError::Internal(e.to_string()))?;

        // 检查是否成功
        if let (Some(app_id), Some(app_secret)) = (
            body.get("client_id").and_then(|v| v.as_str()),
            body.get("client_secret").and_then(|v| v.as_str()),
        ) {
            let user_info = body.get("user_info");
            let tenant_brand = user_info.and_then(|v| v.get("tenant_brand")).and_then(|v| v.as_str());
            let open_id = user_info.and_then(|v| v.get("open_id")).and_then(|v| v.as_str());

            let domain = if tenant_brand == Some("lark") {
                Some("lark".to_string())
            } else {
                Some("feishu".to_string())
            };

            // 尝试获取 bot 信息
            let bot_name = probe_bot(app_id, app_secret).await.ok();

            // 保存到数据库
            let bot_id = state
                .db
                .create_agent_bot("feishu", bot_name.as_deref().unwrap_or("Feishu Bot"), app_id, app_secret, open_id.map(String::from), domain.clone())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            return Ok(Json(serde_json::json!({
                "success": true,
                "app_id": app_id,
                "app_secret": app_secret,
                "domain": domain,
                "open_id": open_id,
                "bot_name": bot_name,
                "bot_id": bot_id
            })));
        }

        // 检查是否失败
        let error = body.get("error").and_then(|v| v.as_str());
        if let Some(err) = error {
            if err == "access_denied" || err == "expired_token" {
                return Ok(Json(serde_json::json!({
                    "success": false,
                    "error": err
                })));
            }
        }

        // 继续轮询
        sleep(Duration::from_secs(interval)).await;
    }
}

async fn probe_bot(app_id: &str, app_secret: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();

    // 获取 tenant_access_token
    let token_res = client
        .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
        .json(&serde_json::json!({
            "app_id": app_id,
            "app_secret": app_secret
        }))
        .send()
        .await?;

    let token_body: serde_json::Value = token_res.json().await?;
    let token = token_body
        .get("tenant_access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing tenant_access_token")?;

    // 获取 bot 信息
    let bot_res = client
        .get("https://open.feishu.cn/open-apis/bot/v3/info")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let bot_body: serde_json::Value = bot_res.json().await?;
    let bot_name = bot_body
        .get("bot")
        .and_then(|v| v.get("app_name"))
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(bot_name.unwrap_or_else(|| "Feishu Bot".to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBotResponse {
    pub id: i64,
    pub bot_type: String,
    pub bot_name: String,
    pub app_id: String,
    pub bot_open_id: Option<String>,
    pub domain: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

pub async fn list_agent_bots(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let bots = state.db.get_agent_bots().await.map_err(|e| AppError::Internal(e.to_string()))?;

    let response: Vec<AgentBotResponse> = bots
        .into_iter()
        .map(|b| AgentBotResponse {
            id: b.id,
            bot_type: b.bot_type,
            bot_name: b.bot_name,
            app_id: b.app_id,
            bot_open_id: b.bot_open_id,
            domain: b.domain,
            enabled: b.enabled,
            created_at: b.created_at,
        })
        .collect();

    Ok(Json(response))
}

pub async fn delete_agent_bot(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    state.db.delete_agent_bot(id).await.map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(serde_json::json!({"success": true})))
}
