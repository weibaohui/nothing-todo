use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "execution_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub record_id: i64,
    pub timestamp: String,
    pub log_type: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub metadata: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::execution_records::Entity",
        from = "Column::RecordId",
        to = "super::execution_records::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    ExecutionRecords,
}

impl Related<super::execution_records::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExecutionRecords.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
