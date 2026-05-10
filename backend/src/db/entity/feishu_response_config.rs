use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "feishu_response_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_id: i64,
    /// "p2p" or "group"
    pub target_type: String,
    /// Whether to enable message response
    pub enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}