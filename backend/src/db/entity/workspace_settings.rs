use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工作空间设置表：存储每个工作空间的独立配置
///
/// 目前存储 default_response_todo_id，替代原有的 Config.default_response_todo_id。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workspace_settings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 工作空间 ID（唯一）
    pub workspace_id: i64,
    /// 默认响应 Todo ID
    pub default_response_todo_id: Option<i64>,
    pub updated_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
