use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "feishu_homes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub bot_id: i64,
    pub user_open_id: String,
    pub chat_id: Option<String>,
    pub receive_id: String,
    pub receive_id_type: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
