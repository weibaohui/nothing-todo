use std::sync::Arc;
use std::time::Duration;

use crate::core::{
    config::{Config, ConfigBuilder},
    constants::AppType,
};

#[cfg(feature = "im")]
use crate::service::im::ImService;

#[cfg(feature = "websocket")]
pub mod ws_client;

pub struct LarkClient {
    pub config: Config,
    shared_config: Arc<Config>,
    #[cfg(feature = "im")]
    pub im: ImService,
}

pub struct LarkClientBuilder {
    config_builder: ConfigBuilder,
}

impl LarkClientBuilder {
    #[cfg(test)]
    fn build_config(&self) -> Config {
        self.config_builder.clone().build()
    }

    pub fn with_app_type(mut self, app_type: AppType) -> Self {
        self.config_builder = self.config_builder.app_type(app_type);
        self
    }

    pub fn with_marketplace_app(mut self) -> Self {
        self.config_builder = self.config_builder.app_type(AppType::Marketplace);
        self
    }

    pub fn with_open_base_url(mut self, base_url: String) -> Self {
        self.config_builder = self.config_builder.base_url(base_url);
        self
    }

    pub fn with_enable_token_cache(mut self, enable: bool) -> Self {
        self.config_builder = self.config_builder.enable_token_cache(enable);
        self
    }

    pub fn with_req_timeout(mut self, timeout: Option<f32>) -> Self {
        if let Some(timeout) = timeout {
            self.config_builder = self
                .config_builder
                .req_timeout(Duration::from_secs_f32(timeout));
        }
        self
    }

    pub fn build(self) -> LarkClient {
        let config = self.config_builder.build();
        let shared_config = Arc::new(config.clone());
        LarkClient {
            config: config.clone(),
            shared_config: shared_config.clone(),
            #[cfg(feature = "im")]
            im: ImService::new_from_shared(shared_config.clone()),
        }
    }
}

impl LarkClient {
    pub fn builder(app_id: &str, app_secret: &str) -> LarkClientBuilder {
        LarkClientBuilder {
            config_builder: Config::builder().app_id(app_id).app_secret(app_secret),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn shared_config(&self) -> Arc<Config> {
        self.shared_config.clone()
    }
}
