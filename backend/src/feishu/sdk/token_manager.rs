use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;

use super::api_types::RawResponse;
use super::cache::QuickCache;
use super::config::{
    Config, APP_ACCESS_TOKEN_KEY_PREFIX, EXPIRY_DELTA, TENANT_ACCESS_TOKEN_INTERNAL_URL_PATH,
};
use super::error::{LarkAPIError, SDKResult};

#[derive(Debug)]
pub struct TokenManager {
    cache: Arc<RwLock<QuickCache<String>>>,
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(QuickCache::new())),
        }
    }

    pub async fn get_tenant_access_token(&self, config: &Config) -> SDKResult<String> {
        let key = tenant_access_token_key(&config.app_id);

        // Fast path: read lock cache hit
        {
            let cache = self.cache.read().await;
            if let Some(token) = cache.get(&key) {
                if !token.is_empty() {
                    return Ok(token);
                }
            }
        }

        // Slow path: fetch new token
        let url = format!(
            "{}{}",
            config.base_url, TENANT_ACCESS_TOKEN_INTERNAL_URL_PATH
        );

        let body = TenantAccessTokenReq {
            app_id: config.app_id.clone(),
            app_secret: config.app_secret.clone(),
        };

        let response = config.http_client.post(&url).json(&body).send().await?;
        let resp: TenantAccessTokenResp = response.json().await?;

        if resp.raw_response.code == 0 {
            let expire = resp.expire - EXPIRY_DELTA;
            let mut cache = self.cache.write().await;
            cache.set(&key, resp.tenant_access_token.clone(), expire);
            Ok(resp.tenant_access_token)
        } else {
            warn!("tenant access token response error: code={}, msg={}", resp.raw_response.code, resp.raw_response.msg);
            Err(LarkAPIError::IllegalParamError(resp.raw_response.msg.clone()))
        }
    }
}

fn tenant_access_token_key(app_id: &str) -> String {
    format!("{APP_ACCESS_TOKEN_KEY_PREFIX}-{app_id}")
}

#[derive(Debug, Serialize, Deserialize)]
struct TenantAccessTokenReq {
    app_id: String,
    app_secret: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TenantAccessTokenResp {
    #[serde(flatten)]
    raw_response: RawResponse,
    expire: i32,
    tenant_access_token: String,
}

