use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agent_bots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_type: String,           // "feishu", "wechat", "qq", etc.
    pub bot_name: String,            // 机器人名称
    pub app_id: String,              // App ID / Client ID
    pub app_secret: String,          // App Secret / Client Secret
    pub bot_open_id: Option<String>, // 机器人的 open_id
    pub domain: Option<String>,      // "feishu" or "lark"
    pub enabled: Option<bool>,       // 是否启用
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
