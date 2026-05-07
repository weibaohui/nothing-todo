use serde::{Deserialize, Serialize};

/// Feishu API domain (Feishu for China, Lark for international).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FeishuDomain {
    #[default]
    Feishu,
    Lark,
}

impl FeishuDomain {
    pub fn base_url(&self) -> &'static str {
        match self {
            Self::Feishu => "https://open.feishu.cn",
            Self::Lark => "https://open.larksuite.com",
        }
    }
}

/// Connection mode for receiving Feishu events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FeishuConnectionMode {
    #[default]
    WebSocket,
    Webhook,
}

/// Configuration for a Feishu bot channel.
#[derive(Debug, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub domain: FeishuDomain,
    pub connection_mode: FeishuConnectionMode,
    pub allowed_users: Vec<String>,
    pub group_require_mention: bool,
    pub dm_policy: Option<String>,
    pub group_policy: Option<String>,
    pub allow_from: Option<Vec<String>>,
    pub group_allow_from: Vec<String>,
    pub encrypt_key: Option<String>,
    pub verification_token: Option<String>,
    pub webhook_port: Option<u16>,
}
