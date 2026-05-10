use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "feishu_group_whitelist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_id: i64,
    pub sender_open_id: String,
    pub sender_name: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}