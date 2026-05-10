use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "feishu_push_targets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_id: i64,
    /// "p2p" or "group"
    pub target_type: String,
    pub chat_id: Option<String>,
    pub receive_id: String,
    pub receive_id_type: String,
    /// Push level: "disabled", "result_only", or "all"
    pub push_level: String,
    /// Whether to enable message response for p2p chats
    pub p2p_response_enabled: bool,
    /// Whether to enable message response for group chats
    pub group_response_enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
