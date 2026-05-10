use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "feishu_push_targets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_id: i64,
    /// 单聊ID (open_id of the user for p2p messaging)
    pub p2p_receive_id: String,
    /// 群ID (chat_id of the group for group messaging)
    pub group_chat_id: String,
    /// 发送类型: "open_id" (use p2p_receive_id) or "chat_id" (use group_chat_id)
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
