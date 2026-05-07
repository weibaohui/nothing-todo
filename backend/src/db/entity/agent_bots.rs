use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "agent_bots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_type: String,
    pub bot_name: String,
    pub app_id: String,
    pub app_secret: String,
    pub bot_open_id: Option<String>,
    pub domain: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
