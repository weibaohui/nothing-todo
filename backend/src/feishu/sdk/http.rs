use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use tracing::debug;

use super::api_types::{ApiRequest, BaseResponse};
use super::config::{AccessTokenType, Config, CONTENT_TYPE_JSON};
use super::error::{LarkAPIError, SDKResult};

pub struct Transport;

impl Transport {
    pub async fn request<T: DeserializeOwned>(
        req: ApiRequest,
        config: &Config,
    ) -> SDKResult<BaseResponse<T>> {
        let access_token = if !req.supported_access_token_types.is_empty()
            && req.supported_access_token_types.contains(&AccessTokenType::Tenant)
        {
            let tm = config.token_manager.lock().await;
            Some(tm.get_tenant_access_token(config).await?)
        } else {
            None
        };

        let url = format!("{}{}", config.base_url, req.api_path);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE_JSON));

        if let Some(token) = &access_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))
                    .map_err(|e| LarkAPIError::RequestError(e.to_string()))?,
            );
        }

        let mut builder = config
            .http_client
            .request(req.http_method.clone(), &url)
            .headers(headers);

        // Add query parameters
        for (key, value) in &req.query_params {
            builder = builder.query(&[*key, value.as_str()]);
        }

        debug!("HTTP {} {}", req.http_method, url);

        let response = builder.body(req.body).send().await?;
        let resp: BaseResponse<T> = response.json().await?;

        Ok(resp)
    }
}
